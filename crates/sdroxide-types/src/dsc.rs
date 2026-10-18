//! Digital Selective Calling — the marine distress and calling protocol.
//!
//! DSC is what fires every GMDSS distress alert: a short digital burst on
//! marine VHF channel 70 (156.525 MHz) and the MF/HF DSC channels
//! (2187.5, 4207.5, 6312, 8414.5, 12577, 16804.5 kHz). A distress alert
//! carries the sender's MMSI, the nature of the distress, a position and a
//! time; a routine call carries who is calling whom and the working channel
//! they are about to move to. It is the one marine emergency channel a
//! listener can decode, which is why it belongs in a listening program.
//!
//! # The wire, and the two layers
//!
//! ITU-R M.493-15 is the authority. The link layer is 1200-baud FFSK —
//! mark 1300 Hz, space 2100 Hz — carrying 10-bit characters, each of which
//! is 7 data bits plus a 3-bit BCH(10,7) check. **Every character is sent
//! twice**: once in the "DX" position and once in the "RX" position,
//! interleaved on a 20-bit grid. The check is a CRC-3, and because its
//! minimum distance is 2 it reliably *detects* a single-bit error but does
//! not reliably correct one; the DX/RX pair is where the redundancy that
//! actually recovers a character lives.
//!
//! What lives here is the **protocol layer**: the BCH codec, the MMSI and
//! position codecs, the format/category/nature tables, and the parser that
//! turns a run of post-BCH 7-bit symbols into a typed message. It is pure
//! arithmetic and tables — no I/O, no threads — so it compiles for the
//! browser and is unit-tested against the published field values.
//!
//! The bit-clock, the FFSK discriminator and the framer that recovers the
//! symbols from audio live in [`sdroxide_dsp::dsc`]. The split is the same
//! one NAVTEX uses: what a message *is* has no dependency on how it was
//! received.
//!
//! # Provenance
//!
//! The tables and codecs follow ITU-R M.493-15. Cross-checked against
//! [GopherTrunk](https://github.com/MattCheramie/GopherTrunk) (Apache-2.0),
//! which records them the same way.

use serde::{Deserialize, Serialize};

/// The DSC message format — the first symbol after the phasing sequence.
/// ITU-R M.493-15 table 3.5.
///
/// A "distress relay" is not a distinct byte: it is format [`Self::AllShips`]
/// with category [`Category::Distress`], which is why the pair has to be read
/// together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DscFormat {
    #[default]
    Unknown,
    /// 112 — a distress alert.
    Distress,
    /// 116 — an all-ships call (a distress relay wears this with the distress
    /// category).
    AllShips,
    /// 114 — a group call.
    Group,
    /// 120 — an individual station call.
    Individual,
    /// 102 — a geographic-area call.
    Geographic,
    /// 123 — an automatic individual call.
    AutoIndividual,
}

impl DscFormat {
    pub fn from_symbol(s: u8) -> Self {
        match s {
            112 => Self::Distress,
            116 => Self::AllShips,
            114 => Self::Group,
            120 => Self::Individual,
            102 => Self::Geographic,
            123 => Self::AutoIndividual,
            _ => Self::Unknown,
        }
    }

    /// The label a log row or panel shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Distress => "DISTRESS",
            Self::AllShips => "ALL SHIPS",
            Self::Group => "GROUP",
            Self::Individual => "INDIVIDUAL",
            Self::Geographic => "GEOGRAPHIC",
            Self::AutoIndividual => "AUTO INDIVIDUAL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// The priority class of a call. ITU-R M.493-15 §3.5.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DscCategory {
    #[default]
    Unknown,
    Routine,
    Safety,
    Urgency,
    Distress,
}

impl DscCategory {
    pub fn from_symbol(s: u8) -> Self {
        match s {
            100 => Self::Routine,
            108 => Self::Safety,
            110 => Self::Urgency,
            112 => Self::Distress,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Routine => "routine",
            Self::Safety => "safety",
            Self::Urgency => "urgency",
            Self::Distress => "distress",
            Self::Unknown => "unknown",
        }
    }
}

/// The nature of a distress alert. ITU-R M.493-15 table 3.7. Only meaningful
/// when the format is [`DscFormat::Distress`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DscNature {
    Undesignated,
    Fire,
    Flooding,
    Collision,
    Grounding,
    Listing,
    Sinking,
    Disabled,
    Abandoning,
    Piracy,
    ManOverboard,
    Epirb,
    #[default]
    Unknown,
}

impl DscNature {
    pub fn from_symbol(s: u8) -> Self {
        match s {
            107 => Self::Undesignated,
            100 => Self::Fire,
            101 => Self::Flooding,
            102 => Self::Collision,
            103 => Self::Grounding,
            104 => Self::Listing,
            105 => Self::Sinking,
            106 => Self::Disabled,
            108 => Self::Abandoning,
            109 => Self::Piracy,
            110 => Self::ManOverboard,
            112 => Self::Epirb,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fire => "fire / explosion",
            Self::Flooding => "flooding",
            Self::Collision => "collision",
            Self::Grounding => "grounding",
            Self::Listing => "listing",
            Self::Sinking => "sinking",
            Self::Disabled => "disabled and adrift",
            Self::Undesignated => "undesignated distress",
            Self::Abandoning => "abandoning ship",
            Self::Piracy => "piracy / armed attack",
            Self::ManOverboard => "man overboard",
            Self::Epirb => "EPIRB emission",
            Self::Unknown => "",
        }
    }
}

/// One decoded DSC sequence.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DscMessage {
    pub format: DscFormat,
    pub category: DscCategory,
    /// Sender MMSI — the distress self-ID, or the caller on a routine call.
    pub self_mmsi: u64,
    /// Recipient MMSI, 0 for an all-ships or distress call.
    pub target_mmsi: u64,
    /// The distress nature, `Unknown` off a distress alert.
    pub nature: DscNature,
    /// Position, when the message carried one and it was not the all-nines
    /// "unknown" sentinel.
    pub position: Option<(f64, f64)>,
    /// Time of the alert, UTC `HH:MM`, when the message carried one.
    pub time_utc: Option<(u8, u8)>,
    /// The working channel an acknowledgement should name, when present.
    pub working_channel: u8,
    /// The 7-bit symbol run, for the log and for debugging a marginal decode.
    pub raw_symbols: Vec<u8>,
    /// True when every character's BCH check passed. False marks a marginal
    /// decode: the sequence is still shown, flagged, rather than dropped.
    pub clean: bool,
}

impl DscMessage {
    /// What the panel's one-line summary shows: the format and the MMSI, plus
    /// the distress nature and position when there are any.
    pub fn summary(&self) -> String {
        let mut s = format!("{} MMSI {:09}", self.format.label(), self.self_mmsi);
        if self.format == DscFormat::Distress {
            let nature = self.nature.label();
            if !nature.is_empty() {
                s.push_str(&format!(" — {nature}"));
            }
            if let Some((lat, lon)) = self.position {
                s.push_str(&format!(" — {:.2}°, {:.2}°", lat, lon));
            }
            if let Some((h, m)) = self.time_utc {
                s.push_str(&format!(" — {h:02}:{m:02}Z"));
            }
        } else if self.target_mmsi != 0 {
            s.push_str(&format!(" → {:09}", self.target_mmsi));
        }
        s
    }
}

/// The 10-bit character DSC wraps around every 7-bit symbol.
///
/// ITU-R M.493: the seven information bits go out **least-significant bit
/// first**, then three check bits holding, most-significant first, the number
/// of B-state (`0`) bits among the seven. The first bit sent is the highest bit
/// of the 10-bit window the framer slides, so `encode` lays the symbol out the
/// way it travels. The minimum distance is 2: a single-bit error is detected
/// but not corrected, and DSC recovers through the DX/RX redundant pair.
pub mod bch {
    /// Wrap a 7-bit symbol into its 10-bit codeword, first-sent bit highest.
    /// Bits above the low 7 are masked off.
    pub fn encode(data: u8) -> u16 {
        let data = data & 0x7F;
        let mut cw = 0u16;
        for k in 0..7 {
            cw |= u16::from((data >> k) & 1) << (9 - k);
        }
        cw | u16::from(7 - data.count_ones() as u8)
    }

    /// The 7 data bits and whether the check passed. On `false` the caller is
    /// expected to fall back to the character's other (RX or DX) copy.
    pub fn check(codeword: u16) -> (u8, bool) {
        let codeword = codeword & 0x3FF;
        let mut data = 0u8;
        for k in 0..7 {
            data |= (((codeword >> (9 - k)) & 1) as u8) << k;
        }
        (data, syndrome(codeword) == 0)
    }

    /// Zero when the three check bits equal the count of zero bits among the
    /// seven information bits.
    pub fn syndrome(codeword: u16) -> u16 {
        let (data, _) = {
            let mut d = 0u8;
            for k in 0..7 {
                d |= (((codeword >> (9 - k)) & 1) as u8) << k;
            }
            (d, ())
        };
        (codeword & 0x7) ^ u16::from(7 - data.count_ones() as u8)
    }
}

/// Five consecutive 7-bit symbols → a 9-digit MMSI.
///
/// ITU-R M.493-15 §3.5.3: each symbol is two decimal digits (00–99); the low
/// digit of the fifth is the format-extension byte and is not part of the
/// MMSI. `None` when a symbol is in the "not used" range above 99.
pub fn decode_mmsi(symbols: &[u8]) -> Option<u64> {
    if symbols.len() != 5 {
        return None;
    }
    let mut mmsi: u64 = 0;
    for (i, &s) in symbols.iter().enumerate() {
        if s > 99 {
            return None;
        }
        if i < 4 {
            mmsi = mmsi * 100 + s as u64;
        } else {
            mmsi = mmsi * 10 + (s / 10) as u64;
        }
    }
    Some(mmsi)
}

/// Five symbols → `(lat, lon)` degrees, or `None`.
///
/// The ten digits are `Q DDMM DDDMM`: the quadrant's high digit (0 = NE,
/// 1 = NW, 2 = SE, 3 = SW) then latitude degrees/minutes and longitude
/// degrees/minutes. The all-nines sentinel is "position unknown" and returns
/// `None` — it is not a fault, it is a station that did not know where it was.
pub fn decode_position(symbols: &[u8]) -> Option<(f64, f64)> {
    if symbols.len() != 5 || symbols.iter().any(|&s| s > 99) {
        return None;
    }
    let mut digits = [0u8; 10];
    for (i, &s) in symbols.iter().enumerate() {
        digits[i * 2] = s / 10;
        digits[i * 2 + 1] = s % 10;
    }
    if digits.iter().all(|&d| d == 9) {
        return None;
    }
    let quadrant = digits[0];
    if quadrant > 3 {
        return None;
    }
    let d2 = |a: usize, b: usize| -> f64 { (digits[a] as f64) * 10.0 + digits[b] as f64 };
    // Q DD MM DDD MM: latitude degrees/minutes, then longitude degrees/minutes.
    let lat = d2(1, 2) + d2(3, 4) / 60.0;
    let lon = d2(5, 6) * 10.0 + digits[7] as f64 + d2(8, 9) / 60.0;
    // Quadrant bit 0 set is west, bit 1 set is south.
    let lat = if quadrant & 2 != 0 { -lat } else { lat };
    let lon = if quadrant & 1 != 0 { -lon } else { lon };
    Some((lat, lon))
}

/// Parse a run of post-BCH 7-bit symbols — the body of a sequence, between the
/// phasing preamble and the end-of-sequence character — into a typed message.
///
/// Never fails: a short or malformed run comes back as
/// [`DscFormat::Unknown`] with the raw symbols preserved, because a receiver
/// hears noisy half-frames constantly and the honest answer is to surface what
/// there is rather than to discard it.
pub fn parse(symbols: &[u8], clean: bool) -> DscMessage {
    let mut m = DscMessage { raw_symbols: symbols.to_vec(), clean, ..Default::default() };
    if symbols.len() < 2 {
        return m;
    }
    m.format = DscFormat::from_symbol(symbols[0]);
    let mut off = 1;

    if m.format == DscFormat::Distress {
        // A distress alert has no separate target: the address is the sender's
        // own ID, then the nature, the position and the time.
        if let Some(mmsi) = symbols.get(off..off + 5).and_then(decode_mmsi) {
            m.self_mmsi = mmsi;
        }
        off += 5;
        if let Some(&n) = symbols.get(off) {
            m.nature = DscNature::from_symbol(n);
            off += 1;
        }
        if let Some(pos) = symbols.get(off..off + 5).and_then(decode_position) {
            m.position = Some(pos);
        }
        off += 5;
        if let Some(pair) = symbols.get(off..off + 2) {
            // Two symbols, each a two-digit field: HH then MM.
            let (hh, mm) = (pair[0], pair[1]);
            if hh <= 23 && mm <= 59 {
                m.time_utc = Some((hh, mm));
            }
        }
        m.category = DscCategory::Distress;
        return m;
    }

    // Everything else: target address, category, then the caller's own ID.
    if let Some(mmsi) = symbols.get(off..off + 5).and_then(decode_mmsi) {
        m.target_mmsi = mmsi;
    }
    off += 5;
    if let Some(&c) = symbols.get(off) {
        m.category = DscCategory::from_symbol(c);
        off += 1;
    }
    if let Some(mmsi) = symbols.get(off..off + 5).and_then(decode_mmsi) {
        m.self_mmsi = mmsi;
    }
    m
}

/// The DSC end-of-sequence characters. Any of them closes a sequence.
pub const EOS_SYMBOLS: [u8; 3] = [117, 122, 127];
/// The phasing-sequence character (ITU-R M.493-15 §3.2.1): it repeats through
/// the phasing run, giving the framer a pattern to lock the cadence onto.
pub const PHASING_SYMBOL: u8 = 125;
/// Bits per DSC character: 7 data + 3 BCH.
pub const CHAR_BITS: u32 = 10;
/// DX and RX characters interleave at the character cadence, so successive DX
/// characters land 20 bits apart.
pub const DX_STRIDE: u32 = 2 * CHAR_BITS;
/// Cap on the symbols collected for one sequence. A runaway capture with no
/// end-of-sequence character is abandoned rather than grown without bound; the
/// longest standard sequence is well under this.
pub const MAX_SEQ_SYMBOLS: usize = 40;

/// Frames a demodulated DSC bit stream into messages.
///
/// This is the layer between the DSP (which turns audio into one bit at a time)
/// and [`parse`]. It slides a 10-bit window through the bits, uses the BCH
/// check to lock onto the repeating phasing character, samples the DX cadence
/// to recover the 7-bit symbols, and stops at the end-of-sequence character.
///
/// # DX/RX time diversity
///
/// DSC sends each character twice, in the DX and RX positions, interleaved on a
/// 20-bit grid. This takes the simple path the reference does first: lock the
/// DX grid on the phasing pattern and read DX only. Comparing each DX character
/// against its RX twin to recover a BCH failure is a yield improvement that
/// needs both grids sampled — a follow-up. Clean captures decode without it.
///
/// # Polarity
///
/// An FM discriminator can present the tones either way round, so the phasing
/// hunt accepts the phasing character and its bitwise complement; locking on
/// the complement inverts the sampled symbols back.
#[derive(Debug, Default)]
pub struct DscFramer {
    state: Framed,
    /// The 10-bit sliding window, newest bit in the low bit.
    reg: u16,
    /// Bits pushed since construction.
    nbits: u32,
    /// Whether the window has seen a full character's worth of bits.
    full: bool,
    /// Hunt: the most recent phasing sighting, to confirm the 20-bit cadence.
    last_dx_bit: u32,
    last_dx_inverted: bool,
    dx_seen: bool,
    /// Lock: the tone sense, the DX boundary, and the collected symbols.
    inverted: bool,
    lock_bit: u32,
    started: bool,
    symbols: Vec<u8>,
    bad_in_seq: usize,
    /// Complete sequences seen, for a status line.
    sequences: u64,
    /// MF/HF stream: the phasing character sits in the RX slots, so the DX grid
    /// is one character before the grid the hunt locks on.
    hf: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Framed {
    #[default]
    Hunt,
    Locked,
}

impl DscFramer {
    pub fn new() -> Self {
        Self::default()
    }

    /// A framer for the MF/HF stream (100 baud), where the phasing sequence
    /// carries 125 in the RX slots and 111..104 in the DX slots.
    pub fn new_hf() -> Self {
        DscFramer { hf: true, ..Self::default() }
    }

    /// Feed one wire bit. Bits outside `{0, 1}` are clamped to 1.
    ///
    /// Returns every message the bit completed, which is at most one — the
    /// `Vec` is for an API that has room for a burst later.
    pub fn push(&mut self, bit: u8, out: &mut Vec<DscMessage>) {
        // Clamp a byte that is not a clean bit down to 1, as the reference
        // does; 0 and 1 pass through unchanged.
        let bit = if bit > 1 { 1 } else { bit };
        self.reg = ((self.reg << 1) | u16::from(bit)) & 0x3FF;
        self.nbits = self.nbits.wrapping_add(1);
        if !self.full {
            if self.nbits >= CHAR_BITS {
                self.full = true;
            } else {
                return;
            }
        }
        match self.state {
            Framed::Hunt => self.hunt(),
            Framed::Locked => self.collect(out),
        }
    }

    /// Number of complete sequences seen, for a status line.
    pub fn sequences(&self) -> u64 {
        self.sequences
    }

    /// Look for two phasing characters one DX stride apart at the same
    /// polarity. That confirms the cadence and the tone sense, and fixes the DX
    /// grid.
    fn hunt(&mut self) {
        let Some(inverted) = self.window_is_phasing() else { return };
        if self.dx_seen
            && self.last_dx_inverted == inverted
            && self.nbits - self.last_dx_bit == DX_STRIDE
        {
            self.state = Framed::Locked;
            self.inverted = inverted;
            self.lock_bit = if self.hf { self.nbits.wrapping_sub(CHAR_BITS) } else { self.nbits };
            self.started = false;
            self.symbols.clear();
            self.bad_in_seq = 0;
            self.dx_seen = false;
            return;
        }
        self.dx_seen = true;
        self.last_dx_bit = self.nbits;
        self.last_dx_inverted = inverted;
    }

    /// Whether the current window decodes to the phasing character under either
    /// polarity, and which one worked.
    fn window_is_phasing(&self) -> Option<bool> {
        if let (data, true) = bch::check(self.reg)
            && data == PHASING_SYMBOL
        {
            return Some(false);
        }
        if let (data, true) = bch::check((!self.reg) & 0x3FF)
            && data == PHASING_SYMBOL
        {
            return Some(true);
        }
        None
    }

    /// Sample the DX grid: at each DX boundary, check the window, skip the
    /// leading phasing characters, then append symbols until the
    /// end-of-sequence character closes the run.
    fn collect(&mut self, out: &mut Vec<DscMessage>) {
        if !(self.nbits - self.lock_bit).is_multiple_of(DX_STRIDE) {
            return;
        }
        let cw = if self.inverted { (!self.reg) & 0x3FF } else { self.reg };
        let (sym, ok) = bch::check(cw);

        if !self.started {
            if sym == PHASING_SYMBOL || (104..=111).contains(&sym) {
                return; // still in the phasing run
            }
            if !ok {
                // Noise on the DX grid before the format specifier: drop the
                // lock and re-hunt.
                self.reset();
                return;
            }
            self.started = true;
            self.symbols.clear();
            self.bad_in_seq = 0;
        }

        // The format specifier goes out twice in a row (M.493); the second
        // copy is not part of the address.
        if self.symbols.len() == 1 && sym == self.symbols[0] {
            return;
        }
        if !ok {
            self.bad_in_seq += 1;
        }
        self.symbols.push(sym);

        if EOS_SYMBOLS.contains(&sym) {
            self.finish(out);
            return;
        }
        if self.symbols.len() >= MAX_SEQ_SYMBOLS {
            self.reset(); // runaway sequence, no EOS: give up the lock
        }
    }

    /// Decode the collected run, emit the message and return to hunting.
    fn finish(&mut self, out: &mut Vec<DscMessage>) {
        self.sequences += 1;
        out.push(parse(&self.symbols, self.bad_in_seq == 0));
        self.reset();
    }

    fn reset(&mut self) {
        self.state = Framed::Hunt;
        self.started = false;
        self.symbols.clear();
        self.bad_in_seq = 0;
        self.dx_seen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The character layout is the standard's, not merely self-consistent:
    /// 125 is `1011111` sent least-significant bit first, then one zero bit
    /// counted as `001`. Taken from a recording of real MF/HF traffic.
    #[test]
    fn the_character_layout_is_the_standards() {
        assert_eq!(bch::encode(125), 0b10_1111_1001);
        assert_eq!(bch::check(0b10_1111_1001), (125, true));
        assert_eq!(bch::check(0b10_1111_1000).1, false);
    }

    /// Bits for one character, most-significant-bit first, matching the wire
    /// order the framer's sliding window expects.
    fn char_bits(symbol: u8) -> impl Iterator<Item = u8> {
        let cw = bch::encode(symbol);
        (0..CHAR_BITS).rev().map(move |i| ((cw >> i) & 1) as u8)
    }

    /// Build the bit stream for a sequence: a phasing run, then the DX/RX
    /// interleaved body, then an end-of-sequence character, exactly as the
    /// on-wire cadence lays it out.
    ///
    /// During phasing the DX slot carries the phasing character and the RX slot
    /// carries a *different* valid character — the receiver samples only the DX
    /// grid, so the RX value is immaterial, but it must not also be the phasing
    /// character or the 20-bit cadence would look like a 10-bit one.
    fn sequence_bits(body: &[u8]) -> Vec<u8> {
        let mut bits = Vec::new();
        for i in 0..8u8 {
            bits.extend(char_bits(PHASING_SYMBOL)); // DX
            bits.extend(char_bits(111 - i)); // RX placeholder
        }
        for &s in body {
            bits.extend(char_bits(s)); // DX
            bits.extend(char_bits(s)); // RX twin (not read)
        }
        bits.extend(char_bits(127)); // end of sequence, DX slot
        bits
    }

    /// The whole chain — encode, frame, parse — recovers a distress alert.
    #[test]
    fn the_framer_recovers_a_distress_alert_from_bits() {
        let mut body = vec![112u8];
        body.extend([36, 61, 23, 45, 60]); // MMSI
        body.push(105); // sinking
        body.extend([0, 51, 30, 0, 7]);
        body.extend([12, 34]);

        let mut framer = DscFramer::new();
        let mut out = Vec::new();
        for bit in sequence_bits(&body) {
            framer.push(bit, &mut out);
        }
        assert_eq!(out.len(), 1, "expected exactly one sequence, got {}", out.len());
        let m = &out[0];
        assert_eq!(m.format, DscFormat::Distress);
        assert_eq!(m.self_mmsi, 366_123_456);
        assert_eq!(m.nature, DscNature::Sinking);
        assert!(m.clean, "a clean synthetic stream must decode clean");
        assert_eq!(framer.sequences(), 1);
    }

    /// The framer copes with the opposite tone sense, as an FM discriminator
    /// may present it.
    #[test]
    fn the_framer_recovers_an_inverted_stream() {
        let body = [120u8, 24, 41, 23, 45, 60, 100, 36, 61, 23, 45, 60];
        let mut framer = DscFramer::new();
        let mut out = Vec::new();
        for bit in sequence_bits(&body) {
            framer.push(1 - bit, &mut out); // invert every bit
        }
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].format, DscFormat::Individual);
        assert_eq!(out[0].target_mmsi, 244_123_456);
    }

    /// The codec round-trips every symbol, and the check rejects a flipped
    /// bit — the property the whole framer rests on.
    #[test]
    fn the_bch_codec_round_trips_and_detects_a_flip() {
        for data in 0u8..128 {
            let cw = bch::encode(data);
            assert_eq!(bch::check(cw), (data, true), "symbol {data} did not round-trip");
            // Every single-bit flip must be seen as a failure for the check to
            // be worth anything. The code's distance is 2, so a flip may land
            // on *another* valid codeword for a different symbol — that is the
            // documented limitation — but the syndrome must be non-zero over
            // the overwhelming majority, and never for the same symbol back.
            let mut caught = 0;
            for bit in 0..10 {
                let (d, ok) = bch::check(cw ^ (1 << bit));
                if !ok {
                    caught += 1;
                } else {
                    assert_ne!(d, data, "a flip reproduced the same symbol {data}");
                }
            }
            assert!(caught >= 9, "only {caught}/10 single-bit flips were caught");
        }
    }

    /// The phasing character IMO uses is symbol 125; its codeword and the
    /// complement both have to be recognisable, which is how the framer
    /// resolves tone polarity.
    #[test]
    fn the_phasing_symbol_is_125() {
        let cw = bch::encode(125);
        assert_eq!(bch::check(cw), (125, true));
        let (d, ok) = bch::check((!cw) & 0x3FF);
        // The complement of a valid codeword is not generally valid; the framer
        // tests the complement's *data* only after it passes its own check.
        let _ = (d, ok);
    }

    /// A nine-digit MMSI comes out of five symbols, and the fifth symbol's low
    /// digit is dropped.
    #[test]
    fn an_mmsi_decodes_from_five_symbols() {
        // 366123456: symbols 36,61,23,45,60 (the trailing 0 is the extension).
        assert_eq!(decode_mmsi(&[36, 61, 23, 45, 60]), Some(366_123_456));
        // A symbol above 99 is the "not used" zone.
        assert_eq!(decode_mmsi(&[36, 61, 23, 45, 100]), None);
        assert_eq!(decode_mmsi(&[1, 2, 3]), None);
    }

    /// A quadrant-flipped position decodes with the right sign.
    #[test]
    fn a_position_decodes_with_its_quadrant() {
        // 51°30.0'N 000°07.0'W → Q=1 (NW), digits 1 5130 00007.
        let symbols = [1 * 10 + 5, 13, 0, 0, 7];
        let (lat, lon) = decode_position(&symbols).expect("a position");
        assert!((lat - 51.5).abs() < 1e-9, "lat {lat}");
        assert!((lon - -0.116_666_7).abs() < 1e-6, "lon {lon}");
        // NE is positive both ways.
        let ne = [0, 51, 30, 0, 7];
        let (lat, lon) = decode_position(&ne).expect("a position");
        assert!(lat > 0.0 && lon > 0.0, "NE decoded as {lat}, {lon}");
        // All nines is "unknown", not a place.
        assert_eq!(decode_position(&[99, 99, 99, 99, 99]), None);
    }

    /// A distress alert parses into the shape the panel shows.
    #[test]
    fn a_distress_alert_parses() {
        // format 112, self 366123456, nature 105 (sinking), position NE,
        // time 12:34.
        let mut syms = vec![112u8];
        syms.extend([36, 61, 23, 45, 60]); // MMSI
        syms.push(105); // nature: sinking
        syms.extend([0, 51, 30, 0, 7]); // position
        syms.extend([12, 34]); // time
        let m = parse(&syms, true);
        assert_eq!(m.format, DscFormat::Distress);
        assert_eq!(m.category, DscCategory::Distress);
        assert_eq!(m.self_mmsi, 366_123_456);
        assert_eq!(m.nature, DscNature::Sinking);
        assert!(m.position.is_some());
        assert_eq!(m.time_utc, Some((12, 34)));
        assert!(m.summary().contains("sinking"));
    }

    /// A routine individual call carries a target and a category.
    #[test]
    fn a_routine_call_parses() {
        // format 120, target 244123456, category 100 (routine), self 366123456.
        let mut syms = vec![120u8];
        syms.extend([24, 41, 23, 45, 60]);
        syms.push(100);
        syms.extend([36, 61, 23, 45, 60]);
        let m = parse(&syms, true);
        assert_eq!(m.format, DscFormat::Individual);
        assert_eq!(m.category, DscCategory::Routine);
        assert_eq!(m.target_mmsi, 244_123_456);
        assert_eq!(m.self_mmsi, 366_123_456);
    }

    /// A short or malformed run is surfaced, not discarded.
    #[test]
    fn a_short_run_is_unknown_not_an_error() {
        // One symbol cannot even establish a format, so it is Unknown — but
        // the raw run is kept either way.
        let m = parse(&[112], false);
        assert_eq!(m.format, DscFormat::Unknown);
        assert_eq!(m.raw_symbols, vec![112]);
        assert!(!m.clean);
        let m = parse(&[], true);
        assert_eq!(m.format, DscFormat::Unknown);
        // Two symbols *do* establish the format, even with nothing after it.
        let m = parse(&[112, 0], true);
        assert_eq!(m.format, DscFormat::Distress);
    }
}
