//! The MIL-STD-188-141A 2G ALE receiver: 8-FSK demodulator, Golay FEC and the
//! 24-bit word layer.
//!
//! ALE is 8-ary FSK at 125 baud — tones 750–2500 Hz, 250 Hz apart, 3 bits per
//! symbol (375 bps). A **word** is 24 bits (`3-bit type + 21-bit payload`, the
//! payload three 7-bit characters from the ALE-64 set); it is Golay(24,12)
//! coded over its two 12-bit halves into 48 bits, bit-interleaved with one
//! stuffing bit to 49, and every word is sent **three times**. The receiver
//! majority-votes the three copies as it de-interleaves.
//!
//! [`AleRx`] is the streaming symbol→word machine; [`demodulate`] turns 8 kHz
//! audio into symbols. The controller resamples the 48 kHz tap to
//! [`ALE_RATE`] and feeds blocks to the two.
//!
//! The constants in [`crate::ale_tables`] are the standard MIL-STD-188-141A
//! tables, from the NTIA/ITS ALE reference (US Government, stated not subject
//! to copyright).

use crate::ale_tables::{ENC, ERR, MTABLE, WT};

/// Audio sample rate the demodulator expects.
pub const ALE_RATE: f64 = 8000.0;
/// The eight ALE tones, Hz.
pub const TONES: [f64; 8] = [750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2250.0, 2500.0];
/// Samples per 125-baud symbol at [`ALE_RATE`].
pub const SAMPLES_PER_SYMBOL: usize = 64;
/// Golay errors the decoder will correct in one 24-bit codeword.
const GOLAY_POWER: usize = 3;

/// The ALE word type, the 3-bit preamble (MIL-STD-188-141B Table A-II).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    Data,
    Thru,
    To,
    Tws,
    From,
    Tis,
    Cmd,
    Rep,
}

impl WordKind {
    fn from_bits(b: u32) -> WordKind {
        match b & 7 {
            0 => WordKind::Data,
            1 => WordKind::Thru,
            2 => WordKind::To,
            3 => WordKind::Tws,
            4 => WordKind::From,
            5 => WordKind::Tis,
            6 => WordKind::Cmd,
            _ => WordKind::Rep,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WordKind::Data => "DATA",
            WordKind::Thru => "THRU",
            WordKind::To => "TO",
            WordKind::Tws => "TWS",
            WordKind::From => "FROM",
            WordKind::Tis => "TIS",
            WordKind::Cmd => "CMD",
            WordKind::Rep => "REP",
        }
    }
}

/// A decoded ALE word: its type and the three-character address it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AleWord {
    pub kind: WordKind,
    pub addr: [u8; 3],
}

impl AleWord {
    /// The address as text.
    pub fn address(&self) -> String {
        self.addr.iter().map(|&c| c as char).collect()
    }
}

/// Whether a byte is in the ALE-64 character set.
fn ale_char(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, b' ' | b'@' | b'?' | b'.' | b'-' | b'/')
}

fn encode(x: u16) -> u32 {
    ((x as u32) << 12) | u32::from(ENC[x as usize])
}

/// Decode one 24-bit Golay codeword to its 12 information bits.
fn golay_decode(w: u32, power: usize) -> Option<u16> {
    let s = ((encode((w >> 12) as u16) ^ w) & 0xfff) as usize;
    if WT[s] as usize > power {
        return None;
    }
    Some(((w >> 12) as u16) ^ ERR[s])
}

/// Decode two 24-bit Golay codewords into one 24-bit ALE word.
fn degolay(g0: u32, g1: u32, power: usize) -> Option<u32> {
    let a = golay_decode(g0, power)?;
    let b = golay_decode(g1, power)?;
    Some(((a as u32) << 12) | (u32::from(b) & 0xfff))
}

/// Turn a 24-bit ALE word into its type and address, or `None` if the three
/// characters are not all in the ALE-64 set.
pub fn parse_word(w: u32) -> Option<AleWord> {
    let payload = (w >> 3) & 0x1f_ffff;
    let mut addr = [0u8; 3];
    for (j, slot) in addr.iter_mut().enumerate() {
        let c = ((payload >> (7 * j)) & 0x7f) as u8;
        if !ale_char(c) {
            return None;
        }
        *slot = c;
    }
    Some(AleWord { kind: WordKind::from_bits(w), addr })
}

/// The streaming ALE word receiver: feed it symbols, it returns a 24-bit word
/// at each 49-symbol boundary.
///
/// The state mirrors the standard receiver: a 49-symbol circular buffer, the
/// ping-pong bit-interleaver, and the majority vote folded into `MTABLE`.
pub struct AleRx {
    sr: [u8; 49],
    p0: usize,
    p1a: usize,
    p1b: usize,
    p2a: usize,
    p2b: usize,
    g1: u32,
    g2: u32,
    ping: bool,
    sptr: usize,
    primed: bool,
}

impl Default for AleRx {
    fn default() -> AleRx {
        AleRx {
            sr: [0; 49],
            p0: 0,
            p1a: 0,
            p1b: 0,
            p2a: 0,
            p2b: 0,
            g1: 0,
            g2: 0,
            ping: false,
            sptr: 0,
            primed: false,
        }
    }
}

impl AleRx {
    pub fn new() -> AleRx {
        // The pointer layout is fixed: the three copies sit 16/17 and 32/33
        // symbols behind the current one.
        AleRx { p1a: 16, p1b: 17, p2a: 32, p2b: 33, ..Default::default() }
    }

    /// Feed one symbol (0..=7). Returns a decoded word every 49 symbols.
    pub fn push(&mut self, s: u8) -> Option<u32> {
        let idx = ((usize::from(self.sr[self.p1a]) & 3) << 7)
            | ((usize::from(self.sr[self.p1b]) & 4) << 4)
            | ((usize::from(self.sr[self.p2a]) & 1) << 5)
            | ((usize::from(self.sr[self.p2b]) & 6) << 2)
            | (usize::from(s) & 7);
        self.sr[self.p0] = s & 7;
        let maj = u32::from(MTABLE[idx]);
        let hi = ((maj & 4) >> 1) | (maj & 1);
        let lo = (maj & 2) >> 1;
        if self.ping {
            self.g1 = ((self.g1 << 2) | hi) & 0xffff_ffff;
            self.g2 = ((self.g2 << 1) | lo) & 0xffff_ffff;
        } else {
            self.g2 = ((self.g2 << 2) | hi) & 0xffff_ffff;
            self.g1 = ((self.g1 << 1) | lo) & 0xffff_ffff;
        }
        self.ping = !self.ping;
        self.p0 = (self.p0 + 1) % 49;
        self.p1a = (self.p1a + 1) % 49;
        self.p1b = (self.p1b + 1) % 49;
        self.p2a = (self.p2a + 1) % 49;
        self.p2b = (self.p2b + 1) % 49;
        self.sptr = (self.sptr + 1) % 49;
        if self.sptr != 0 {
            self.primed = true;
            return None;
        }
        if !self.primed {
            return None;
        }
        let word = if self.ping {
            degolay((self.g2 >> 1) & 0xff_ffff, (self.g1 ^ 0xfff) & 0xff_ffff, GOLAY_POWER)
        } else {
            degolay((self.g1 >> 1) & 0xff_ffff, (self.g2 ^ 0xfff) & 0xff_ffff, GOLAY_POWER)
        };
        word
    }
}

/// Demodulate 8 kHz audio into ALE symbols, one per 125-baud window.
///
/// Rectangular matched filter: the energy at each of the eight tones over one
/// 64-sample window; the strongest tone is the symbol. Trailing samples that
/// do not fill a window are dropped.
pub fn demodulate(audio: &[f32]) -> Vec<u8> {
    let n = audio.len() / SAMPLES_PER_SYMBOL;
    let mut out = Vec::with_capacity(n);
    for w in 0..n {
        let win = &audio[w * SAMPLES_PER_SYMBOL..(w + 1) * SAMPLES_PER_SYMBOL];
        let mut best = 0usize;
        let mut best_e = f64::NEG_INFINITY;
        for (k, &f) in TONES.iter().enumerate() {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &s) in win.iter().enumerate() {
                let ph = -2.0 * std::f64::consts::PI * f * i as f64 / ALE_RATE;
                re += f64::from(s) * ph.cos();
                im += f64::from(s) * ph.sin();
            }
            let e = re * re + im * im;
            if e > best_e {
                best_e = e;
                best = k;
            }
        }
        out.push(best as u8);
    }
    out
}

/// The eight tone energies of one 64-sample window.
fn window_energies(win: &[f32]) -> [f64; 8] {
    let mut e = [0.0f64; 8];
    for (k, &f) in TONES.iter().enumerate() {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, &s) in win.iter().enumerate() {
            let ph = -2.0 * std::f64::consts::PI * f * i as f64 / ALE_RATE;
            re += f64::from(s) * ph.cos();
            im += f64::from(s) * ph.sin();
        }
        e[k] = re * re + im * im;
    }
    e
}

/// Demodulate starting at sample `phase`, one ALE symbol per window.
pub fn demodulate_from(audio: &[f32], phase: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut p = phase;
    while p + SAMPLES_PER_SYMBOL <= audio.len() {
        let e = window_energies(&audio[p..p + SAMPLES_PER_SYMBOL]);
        let mut best = 0usize;
        for k in 1..8 {
            if e[k] > e[best] {
                best = k;
            }
        }
        out.push(best as u8);
        p += SAMPLES_PER_SYMBOL;
    }
    out
}

/// The symbol-timing metric for a phase: the average, over windows, of how
/// concentrated the tone energy is on the strongest tone. Peaks when the
/// windows line up with the symbols; a straddling window splits its energy
/// across two tones and scores lower. Cheap and FEC-free, so all 64 phases can
/// be tried.
pub fn timing_metric(audio: &[f32], phase: usize) -> f64 {
    let (mut score, mut n) = (0.0f64, 0u32);
    let mut p = phase;
    while p + SAMPLES_PER_SYMBOL <= audio.len() {
        let e = window_energies(&audio[p..p + SAMPLES_PER_SYMBOL]);
        let total: f64 = e.iter().sum();
        if total > 1e-9 {
            let mx = e.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            score += mx / total;
            n += 1;
        }
        p += SAMPLES_PER_SYMBOL;
    }
    if n == 0 { 0.0 } else { score / f64::from(n) }
}

/// The phase whose windows best line up with the symbols.
pub fn best_phase(audio: &[f32]) -> usize {
    (0..SAMPLES_PER_SYMBOL)
        .max_by(|&a, &b| {
            timing_metric(audio, a)
                .partial_cmp(&timing_metric(audio, b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0)
}

/// Decode a burst of 8 kHz audio into the ALE words it carries, collapsing the
/// three repetitions each word is sent with.
///
/// Finds the symbol clock, runs the FEC, and keeps only words that (a) parse to
/// the ALE-64 set and (b) were heard at least twice in a row — which is what
/// separates a real word from FEC noise in the gaps.
pub fn decode_burst(audio: &[f32]) -> Vec<AleWord> {
    // A recording is a run of bursts separated by silence, and the symbol clock
    // phase is measured per burst — averaging it over the whole file lets the
    // silent spans choose the phase. Walk it in overlapping segments, decode
    // each on its own clock, and merge the words.
    let seg = 2 * ALE_RATE as usize;
    let hop = seg / 2;
    let mut out: Vec<AleWord> = Vec::new();
    if audio.len() < SAMPLES_PER_SYMBOL * 49 {
        return out;
    }
    let mut at = 0usize;
    loop {
        let end = (at + seg).min(audio.len());
        for w in decode_segment(&audio[at..end]) {
            if !out.contains(&w) {
                out.push(w);
            }
        }
        if end == audio.len() {
            break;
        }
        at += hop;
    }
    out
}

/// Decode one segment on its own best clock phase, keeping only words heard at
/// least twice in a row (a real word is sent three times; FEC noise is not).
fn decode_segment(audio: &[f32]) -> Vec<AleWord> {
    let phase = best_phase(audio);
    let mut rx = AleRx::new();
    let mut run: Option<u32> = None;
    let mut count = 0u32;
    let mut out = Vec::new();
    fn flush(run: &mut Option<u32>, count: &mut u32, out: &mut Vec<AleWord>) {
        if *count >= 2
            && let Some(w) = *run
            && let Some(word) = parse_word(w)
        {
            out.push(word);
        }
    }
    for s in demodulate_from(audio, phase) {
        if let Some(w) = rx.push(s) {
            if run == Some(w) {
                count += 1;
            } else {
                flush(&mut run, &mut count, &mut out);
                run = Some(w);
                count = 1;
            }
        }
    }
    flush(&mut run, &mut count, &mut out);
    out
}

/// Encode a 24-bit word into its 49 transmitted symbols. Used by the tests and
/// mirrors the standard transmitter.
pub fn transmit_symbols(w: u32) -> [u8; 49] {
    let g0 = encode((w >> 12) as u16);
    let g1 = encode((w & 0xfff) as u16) ^ 0xfff;
    let mut a = g0 << 2;
    let mut b = g1 << 1;
    let mut t = [0u8; 49];
    t[48] = ((a & 4) | (b & 2)) as u8;
    let seg = |lo: usize, hi: isize, t: &mut [u8; 49], a: &mut u32, b: &mut u32| {
        let mut i = lo as isize;
        while i > hi {
            t[i as usize] = ((*a & 0o40) | (*b & 0o20)) as u8;
            *a >>= 1;
            *b >>= 1;
            t[i as usize] = (((u32::from(t[i as usize]) | (*a & 0o10))) >> 3) as u8;
            t[i as usize + 1] = (*b & 4) as u8;
            *a >>= 1;
            *b >>= 1;
            t[i as usize + 1] |= ((*a & 2) | (*b & 1)) as u8;
            *a >>= 1;
            *b >>= 1;
            i -= 2;
        }
    };
    seg(46, 33, &mut t, &mut a, &mut b);
    a |= g1 << 5;
    b |= g0 << 5;
    seg(32, 16, &mut t, &mut a, &mut b);
    a |= g0 << 6;
    b |= g1 << 5;
    seg(16, -1, &mut t, &mut a, &mut b);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(word: u32) {
        let syms = transmit_symbols(word);
        let mut rx = AleRx::new();
        let mut got = Vec::new();
        // Three copies, as on the air.
        for _ in 0..3 {
            for &s in &syms {
                if let Some(w) = rx.push(s) {
                    got.push(w);
                }
            }
        }
        assert!(got.contains(&word), "word {word:#x} not recovered: {got:x?}");
    }

    #[test]
    fn the_fec_round_trips_a_word_three_times() {
        for w in [0x2c_c4c5, 0x000000, 0xff_ffff, 0x12_3456, 0xab_cdef] {
            round_trip(w & 0xff_ffff);
        }
    }

    #[test]
    fn a_word_parses_to_its_type_and_address() {
        // type = FROM (4), address "ABC" = 0x41,0x42,0x43 in 7-bit fields.
        let w = 4 | (0x41 << 3) | (0x42 << 10) | (0x43 << 17);
        let word = parse_word(w).expect("valid ALE-64 address");
        assert_eq!(word.kind, WordKind::From);
        assert_eq!(word.address(), "ABC");
    }

    #[test]
    fn a_non_character_word_is_rejected() {
        // 0x01 is not in the ALE-64 set.
        let w = (0x01) | (0x41 << 10) | (0x42 << 17);
        assert!(parse_word(w).is_none());
    }

    #[test]
    fn audio_round_trips_through_the_demodulator() {
        let word = 0x2c_c4c5u32 & 0xff_ffff;
        let tone_of = |s: u8| TONES[s as usize];
        let mut audio = Vec::new();
        for _ in 0..3 {
            for &s in &transmit_symbols(word) {
                let f = tone_of(s);
                for i in 0..SAMPLES_PER_SYMBOL {
                    let t = i as f64 / ALE_RATE;
                    audio.push((2.0 * std::f64::consts::PI * f * t).cos() as f32);
                }
            }
        }
        let syms = demodulate(&audio);
        let mut rx = AleRx::new();
        let mut got = Vec::new();
        for s in syms {
            if let Some(w) = rx.push(s) {
                got.push(w);
            }
        }
        assert!(got.contains(&word), "demod+fec failed: {got:x?}");
    }

    fn from_abc() -> u32 {
        4 | (u32::from(b'A') << 3) | (u32::from(b'B') << 10) | (u32::from(b'C') << 17)
    }

    fn synth(word: u32, copies: usize, offset: usize, amp: f32, noise: f32) -> Vec<f32> {
        let mut audio = vec![0.0f32; offset];
        for c in 0..copies {
            for &s in &transmit_symbols(word) {
                let f = TONES[s as usize];
                for i in 0..SAMPLES_PER_SYMBOL {
                    let t = i as f64 / ALE_RATE;
                    audio.push(amp * (2.0 * std::f64::consts::PI * f * t).cos() as f32);
                }
            }
            let _ = c;
        }
        let mut seed = 0x1234_5678u32;
        for a in audio.iter_mut() {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *a += ((seed >> 8) as f32 / 16_777_215.0 - 0.5) * 2.0 * noise;
        }
        audio
    }

    #[test]
    fn the_front_end_finds_the_clock_offset_and_decodes_a_noisy_burst() {
        // A clock offset (the burst does not start on a window boundary) and
        // noise, which the fixed-phase test does not exercise.
        let words = decode_burst(&synth(from_abc(), 3, 23, 0.5, 0.1));
        assert!(
            words.iter().any(|w| w.kind == WordKind::From && w.address() == "ABC"),
            "front end failed: {words:?}"
        );
    }

    /// Off the air, behind an ignored test: point `SDROXIDE_ALE_SAMPLE` at a
    /// raw little-endian `f32` mono 8 kHz file and it must yield words.
    #[test]
    #[ignore]
    fn an_off_air_recording_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_ALE_SAMPLE") else {
            return;
        };
        let bytes = std::fs::read(path).expect("read sample");
        let audio: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        let words = decode_burst(&audio);
        eprintln!("decoded {} words", words.len());
        for w in &words {
            eprintln!("{} {}", w.kind.label(), w.address());
        }
        assert!(!words.is_empty(), "no words decoded from the recording");
    }
}
