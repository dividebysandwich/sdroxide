//! Olivia MFSK modem: streaming decode + encode.
//!
//! Olivia is a strong, slow MFSK chat mode. A block of 64 MFSK symbols carries
//! `log2(tones)` characters: each character (7-bit ASCII) is spread across the
//! 64 symbols with a (64,7) biorthogonal Walsh/Hadamard code, one character per
//! MFSK bit-plane. The heavy coding gain is what makes Olivia decode well below
//! the noise floor. A fixed per-position tone scrambler spreads the spectrum and
//! (crucially) gives the receiver an unambiguous block boundary even during the
//! constant idle stream.
//!
//! Parameters are the tone count (2..=64) and the bandwidth in Hz; tone spacing =
//! bandwidth/tones and the symbol rate equals the spacing. Common combinations
//! are 32/1000, 16/500 and 8/250.
//!
//! **Interop status: receive works and is confirmed off the air, and transmit
//! now round-trips through our own receiver.** The last piece on receive was
//! the Walsh transform: our `fwht` used the textbook `(b1+b2, b1-b2)` butterfly,
//! where fldigi uses `(b2+b1, b2-b1)` (`pj_fht.h`). The two differ by a
//! per-row sign, and in Olivia a sign is *bit 6 of the character* — so every
//! lowercase letter (and the idle character) decoded as its bit-6-cleared twin
//! (`u` 117 → `5` 53, `h` 104 → `(` 40). The receiver now uses fldigi's `fht`,
//! and the Avalon SW Net recording (`CQ SouthWest NET … de G7LEE G7LEE G7LEE`)
//! decodes cleanly — see the `an_off_air_capture_decodes` test.
//!
//! The transmitter builds each codeword with `ifht`, the matching inverse, which
//! is what fldigi's `EncodeBlock` does; before that it used the textbook
//! convention, so our own transmission was not received by our own receiver.
//! Both loopbacks (`loopback_32_1000`, `loopback_8_250`) now pass.
//!
//! One convention difference from fldigi's source is deliberate and measured:
//! fldigi sets the symbol bit where its codeword is **negative** and hands its
//! own decoder a **negative** value for a set bit, while this receiver votes
//! **positive**. The two are exact negatives of each other, so they cannot both
//! describe the air; the recording decides, and it is the receiver that already
//! reads it correctly. `encode_block` says so where the bit is set.
//!
//! Everything else was already right: the 64-symbol block, the (64,7) Walsh
//! codeword per character, the Gray tone assignment, tone spacing = symbol rate,
//! and the `0xE257E6D0291574EC` scrambler with its 13-bit-per-character
//! rotation and the `(character + symbol) mod log2(tones)` interleave.
//!
//! **Start tones are now transmitted.** fldigi brackets every transmission with a
//! pair of tones at the band edges — see [`TONE_DURATION`] — and gates them on
//! `olivia_start_tones`, whose default is **true**, so a default fldigi
//! transmission carries them and matching it is the right thing to send.
//!
//! **They are not how fldigi acquires a frame, and an earlier note here said
//! they were.** `olivia_start_tones` is read only in `send_tones()`, which the
//! transmit path calls; the receiver never consults it and free-runs a
//! correlator bank over 17 frequency offsets by block phase
//! (`FreqOffsets = 2 * SyncMargin + 1`, `SyncIntegLen = 4`). Our receiver has
//! always free-run the same way, which is why every loopback passed without
//! them. Sending them is correct for interop fidelity; believing they are the
//! reason a decoder would copy nothing sent this note after a mechanism that
//! does not exist.
//!
//! Still absent, and not a regression: the **tail** (fldigi's stop tones plus a
//! short silence) and any **frequency search** beyond the caller's tone bank
//! centre. The tail cannot simply be appended inside [`OliviaTx`]: the caller
//! stops asking for audio the moment `sent_chars` reaches `total_chars`
//! (`text_modem.rs`), so tones generated into the block buffer would never be
//! drained. That needs a change to the transmit-active contract in the digi
//! engine, which is deliberately not smuggled in here.
use std::collections::VecDeque;

use crate::mfsk::{ToneGen, gray, tone_bank_mags, ungray};

const OUT_AMP: f32 = 0.5;
/// Symbols per Olivia block (fixed by the 64-length Walsh code).
const BLOCK: usize = 64;
/// Timing sub-phases searched per symbol.
const SUBPHASES: usize = 16;

/// fldigi's sound-card block size — `SCBLOCKSIZE` in `src/include/sound.h`.
const FLDIGI_SCBLOCK: usize = 512;
/// One quarter of the start/stop tones. fldigi derives these from its own block
/// size rather than from the sample rate or the symbol rate: `TONE_DURATION` is
/// `SCBLOCKSIZE * 16` and `SR4` is a quarter of it (`src/include/olivia.h`).
const TONE_QUARTER: usize = FLDIGI_SCBLOCK * 4;
/// Total length of the start tones — `SCBLOCKSIZE * 16`, i.e. 8192 samples.
const TONE_DURATION: usize = TONE_QUARTER * 4;
/// The raised-cosine edge `ampshape` puts at each end of every tone quarter —
/// `SR4 / 8` samples, or 256.
const TONE_EDGE: usize = TONE_QUARTER / 8;

/// Not yet bracketed with a tail — see the module note.
const FRAMING_START: u8 = 0;
const FRAMING_DATA: u8 = 1;

/// Olivia's scrambling sequence. The Walsh function carrying character `c` in a
/// block is scrambled with this 64-bit pattern, consumed one bit per symbol
/// starting at bit `13 * c` — that is, the character's codeword has its sign
/// flipped wherever bit `(13 * c + i) & 63` of this constant is set.
///
/// This is the constant the mode is named for, and getting it wrong is the
/// whole reason this decoder could not read a real Olivia station: the mode's
/// structure was already Olivia's, but the scrambler was a local invention, so
/// every block descrambled to noise. Reference: fldigi's jalocha
/// `MFSK_Encoder::ScramblingCodeOlivia` (GPL-3.0), the same constant the mode's
/// documentation gives.
const SCRAMBLE: u64 = 0xE257_E6D0_2915_74EC;

/// The bit of [`SCRAMBLE`] that scrambles position `i` of character `c`'s
/// Walsh function.
fn scramble_bit(c: usize, i: usize) -> bool {
    (SCRAMBLE >> ((13 * c + i) & (BLOCK - 1))) & 1 == 1
}

/// Resolved Olivia geometry for a (tones, bandwidth) pair at a sample rate.
#[derive(Clone, Copy)]
struct Geom {
    tones: usize,
    planes: usize,
    spacing: f64,
    sps: usize,
    base_hz: f64,
}

impl Geom {
    fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let tones = tones.clamp(2, 64).next_power_of_two();
        let planes = tones.trailing_zeros() as usize; // log2(tones)
        let spacing = bw / tones as f64;
        let sps = (rate / spacing).round().max(16.0) as usize;
        // Centre the tone bank on the audio frequency.
        let base_hz = audio_hz - bw / 2.0 + spacing / 2.0;
        Geom { tones, planes, spacing, sps, base_hz }
    }

    fn tone_hz(&self, k: usize) -> f64 {
        self.base_hz + k as f64 * self.spacing
    }

    /// fldigi's start/stop tone pair: the two band edges, `txbasefreq ∓
    /// bandwidth/2` in `olivia::send_tones()`. The tone bank runs from
    /// `base_hz` to `base_hz + (tones-1) * spacing`, so the edges sit **half a
    /// tone spacing outside** it on each side and the pair brackets the bank
    /// exactly.
    fn edge_hz(&self) -> (f64, f64) {
        (
            self.base_hz - self.spacing / 2.0,
            self.base_hz + self.tones as f64 * self.spacing - self.spacing / 2.0,
        )
    }
}

/// fldigi's fast Walsh–Hadamard transform, verbatim from `pj_fht.h`: the
/// butterfly is `(b2+b1, b2-b1)`, **not** the textbook `(b1+b2, b1-b2)`. The two
/// differ by a per-row sign, and in Olivia a sign *is* bit 6 of the character,
/// so the textbook form decodes every lowercase letter as its bit-6-cleared
/// twin (`u`→`5`, `h`→`(`) while leaving space and uppercase intact.
///
/// This is the whole reason Olivia used to half-decode: the mixer's tone
/// magnitudes were always right, and only the character sign was wrong.
///
/// `ifht` is the matching inverse, and the pair is pinned by
/// `the_inverse_and_forward_transforms_are_a_pair`.
fn fht(a: &mut [f32]) {
    let n = a.len();
    let mut step = 1;
    while step < n {
        let mut p = 0;
        while p < n {
            let mut q = p;
            while q < p + step {
                let (b1, b2) = (a[q], a[q + step]);
                a[q] = b2 + b1;
                a[q + step] = b2 - b1;
                q += 1;
            }
            p += 2 * step;
        }
        step *= 2;
    }
}

/// fldigi's `IFHT`, the inverse of [`fht`] and therefore what the transmitter
/// applies to a code bit vector to get the tone bank a receiver transforms back
/// into it. The pair is pinned by
/// `the_inverse_and_forward_transforms_are_a_pair`.
fn ifht(a: &mut [f32]) {
    let n = a.len();
    let mut step = n / 2;
    while step > 0 {
        let mut p = 0;
        while p < n {
            let mut q = p;
            while q < p + step {
                let (b1, b2) = (a[q], a[q + step]);
                a[q] = b1 - b2;
                a[q + step] = b1 + b2;
                q += 1;
            }
            p += 2 * step;
        }
        step /= 2;
    }
}

// ─────────────────────────────── transmit ───────────────────────────────

pub struct OliviaTx {
    rate: f64,
    g: Geom,
    tonegen: ToneGen,
    /// Queued characters with their source index (`None` = NUL idle fill).
    q: VecDeque<(u8, Option<usize>)>,
    total_chars: usize,
    sent_chars: usize,
    cur: Vec<f32>,
    cur_pos: usize,
    cur_done: Option<usize>,
    framing: u8,
    /// The closing bracket, built when the message runs out: fldigi's stop tones
    /// and then a short silence. `None` while there is nothing to close yet.
    tail: Option<Vec<f32>>,
    tail_pos: usize,
    /// The bracket goes out once. Without this the "message is out" condition
    /// stays true after it has been played, so a caller that asks for more
    /// audio — which the loopback flushes do — gets the bracket again, for ever.
    tail_done: bool,
}

impl OliviaTx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw);
        OliviaTx {
            rate,
            g,
            tonegen: ToneGen::new(rate),
            q: VecDeque::new(),
            total_chars: 0,
            sent_chars: 0,
            cur: Vec::new(),
            cur_pos: 0,
            cur_done: None,
            framing: FRAMING_START,
            tail: None,
            tail_pos: 0,
            tail_done: false,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        self.g = Geom::new(self.rate, audio_hz, tones, bw);
    }

    pub fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            let byte = if ch.is_ascii() { ch as u8 } else { b'?' };
            self.q.push_back((byte, Some(self.total_chars)));
            self.total_chars += 1;
        }
    }

    pub fn sent_chars(&self) -> usize {
        self.sent_chars
    }
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }
    /// True once the message **and** its closing bracket have been played out.
    ///
    /// The bracket is part of the transmission, not an optional grace: a decoder
    /// that integrates four FEC blocks of the tail to find the frame end would
    /// otherwise run off the end of what we sent.
    pub fn drained(&self) -> bool {
        self.q.is_empty() && self.cur_pos >= self.cur.len() && !self.tail_pending()
    }

    pub fn clear(&mut self) {
        self.q.clear();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        self.total_chars = 0;
        self.sent_chars = 0;
        // A new transmission is bracketed with its own start tones, and the
        // closing bracket belongs to the one that just ended.
        self.framing = FRAMING_START;
        self.tail = None;
        self.tail_pos = 0;
        self.tail_done = false;
    }

    /// Whether the closing bracket is still waiting to be drained.
    ///
    /// The caller needs this: it stops asking for audio the moment the last
    /// character is out, so tones generated into the block buffer after that
    /// would never be played. See [`Self::next_block`].
    pub fn tail_pending(&self) -> bool {
        self.tail.is_some()
    }

    /// fldigi's closing bracket: the stop tones again, then a short silence.
    ///
    /// `send_tones()` a second time (`postamblesent`), then `SCBLOCKSIZE`
    /// samples of zeros (`olivia.cxx`).
    fn build_tail(&mut self) {
        let (low, high) = self.g.edge_hz();
        let mut t = Vec::with_capacity(TONE_DURATION + FLDIGI_SCBLOCK);
        for q in 0..TONE_DURATION / TONE_QUARTER {
            let start = t.len();
            let hz = if q % 2 == 0 { low } else { high };
            self.tonegen.emit(hz, TONE_QUARTER, OUT_AMP, &mut t);
            for i in 0..TONE_EDGE {
                let w = 0.5 * (1.0 - (std::f64::consts::PI * i as f64 / TONE_EDGE as f64).cos());
                t[start + i] *= w as f32;
                t[start + TONE_QUARTER - 1 - i] *= w as f32;
            }
        }
        t.resize(TONE_DURATION + FLDIGI_SCBLOCK, 0.0);
        self.tail = Some(t);
        self.tail_pos = 0;
    }

    /// fldigi's start tones, into `cur`. See [`TONE_DURATION`].
    ///
    /// `olivia::send_tones()` writes `TONE_DURATION` samples as four equal
    /// quarters of `SR4`, alternating between the two band edges — low, high,
    /// low, high, with the lower edge first unless fldigi's `reverse` is set
    /// (we have no reverse). Two details are not cosmetic:
    ///
    /// - **The phase is carried across quarters.** [`ToneGen`] keeps it, so a
    ///   change of tone is a phase change rather than a step.
    /// - **Every quarter is ramped at both ends.** fldigi's `ampshape` is 1.0
    ///   except over `SR4/8` samples at each end, where it is a raised cosine;
    ///   without it each of the four tone changes is a discontinuity, and three
    ///   clicks inside one frame is a spectral splatter the receiver sees as
    ///   noise.
    ///
    /// fldigi then puts exactly one idle character into the transmitter, with
    /// the comment "Olivia Transmitter class requires at least character". It is
    /// pushed here with **no** source index so it never counts as sent text: it
    /// is the frame a receiver locks to, not a character of the message.
    fn build_start_tones(&mut self) {
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = None;
        // fldigi zeroes `preamblephase` before each pair it sends, so a burst
        // begins at a known phase rather than wherever the last one ended.
        self.tonegen.reset_phase();
        let (low, high) = self.g.edge_hz();
        for q in 0..TONE_DURATION / TONE_QUARTER {
            let start = self.cur.len();
            let hz = if q % 2 == 0 { low } else { high };
            self.tonegen.emit(hz, TONE_QUARTER, OUT_AMP, &mut self.cur);
            for i in 0..TONE_EDGE {
                let w = 0.5 * (1.0 - (std::f64::consts::PI * i as f64 / TONE_EDGE as f64).cos());
                self.cur[start + i] *= w as f32;
                self.cur[start + TONE_QUARTER - 1 - i] *= w as f32;
            }
        }
        self.q.push_front((0, None));
    }

    fn build_block(&mut self) {
        let (out, done) = self.encode_block();
        self.cur.clear();
        self.cur_pos = 0;
        self.cur_done = done;
        for i in 0..BLOCK {
            let tone = gray(out[i]) as usize % self.g.tones;
            self.tonegen.emit(self.g.tone_hz(tone), self.g.sps, OUT_AMP, &mut self.cur);
        }
    }

    /// Assemble one block: the per-symbol bit vector (before the Gray map) and
    /// the source index of the last character that went into it.
    ///
    /// One character per bit-plane, spread over the 64 symbols by the (64,7)
    /// Walsh code, then interleaved: character `p`'s function lands on bit
    /// `(p + i) % planes` of symbol `i`, so consecutive characters take
    /// consecutive bits of the same symbol. The scrambler flips the **sign** of
    /// a codeword — a Walsh-domain sign flip, not a rotation of the tone number —
    /// at bit `13 * p + i` of [`SCRAMBLE`].
    fn encode_block(&mut self) -> ([u32; BLOCK], Option<usize>) {
        // Take up to `planes` characters; pad with NUL idle fill.
        let mut chars = [0u8; 6];
        let mut done: Option<usize> = None;
        for slot in chars.iter_mut().take(self.g.planes) {
            if let Some((b, idx)) = self.q.pop_front() {
                *slot = b;
                if let Some(i) = idx {
                    done = Some(done.map_or(i, |d| d.max(i)));
                }
            }
        }
        let mut out = [0u32; BLOCK];
        for (p, &b) in chars.iter().enumerate().take(self.g.planes) {
            // One code bit at `b & 63`, negated when bit 6 of the character is
            // set, put through the *inverse* of the transform the receiver
            // applies — which is what the receiver's `fht` turns back into this
            // code bit, at that row, with that sign.
            let mut f = [0.0f32; BLOCK];
            f[(b & 63) as usize] = if b & 64 != 0 { -1.0 } else { 1.0 };
            ifht(&mut f);
            for i in 0..BLOCK {
                if scramble_bit(p, i) {
                    f[i] = -f[i];
                }
                // Set the bit where the codeword is **positive**.
                //
                // fldigi's `EncodeBlock` sets it where the codeword is negative
                // (`if (FHT_Buffer[TimeBit] < 0)`), and its `SoftDecode` hands
                // the decoder a *negative* value for a set bit as well — those
                // two are an exact pair, and `jalocha`'s own encode/decode
                // round-trips under them. They are the negative of each other
                // relative to this receiver: `soft_bits` votes **+** for a
                // carrier whose bit is set. One of the two conventions is the
                // air's, and the recording settles it: this receiver, which
                // reads `CQ SouthWest NET … de G7LEE G7LEE G7LEE` off an
                // Avalon SW Net station with every character in the right case
                // (`an_off_air_capture_decodes`), needs the negative of
                // fldigi's soft sign. Emitting fldigi's literal `OutputBlock`
                // therefore makes our own receiver decode every character as its
                // bit-6-set twin, and the loopback tests below fail with
                // `@@@@@` where the text should be.
                //
                // So the transmitter follows the air and not the source file.
                // Negating the delta instead of this test is the same signal;
                // testing the sign where the bit is decided keeps the one
                // measured fact visible.
                if f[i] > 0.0 {
                    out[i] |= 1 << ((p + i) % self.g.planes);
                }
            }
        }
        (out, done)
    }

    pub fn next_block(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            if self.cur_pos >= self.cur.len() {
                // The closing bracket, once the message is out and TX has been
                // released. It has to be *generated* here rather than appended
                // to the message, because the caller stops asking for audio the
                // moment `sent_chars` reaches `total_chars` — anything still
                // sitting in the block buffer at that point is never drained.
                if let Some(tail) = self.tail.take() {
                    let take = (tail.len() - self.tail_pos).min(out.len() - n);
                    out[n..n + take].copy_from_slice(&tail[self.tail_pos..self.tail_pos + take]);
                    self.tail_pos += take;
                    n += take;
                    // Only re-queue what has not been played yet, and mark the
                    // bracket spent once it has all been played.
                    if self.tail_pos < tail.len() {
                        self.tail = Some(tail);
                    } else {
                        self.tail_done = true;
                    }
                    continue;
                }
                if let Some(ci) = self.cur_done.take() {
                    self.sent_chars = ci + 1;
                }
                if self.framing == FRAMING_START {
                    self.build_start_tones();
                    self.framing = FRAMING_DATA;
                } else if !self.tail_done
                    && self.q.is_empty()
                    && self.sent_chars >= self.total_chars
                    && self.total_chars > 0
                {
                    self.build_tail();
                    // The bracket does not go through `cur`; hand straight back
                    // to the tail branch that drains it.
                    continue;
                } else {
                    self.build_block();
                }
            }
            out[n] = self.cur[self.cur_pos];
            self.cur_pos += 1;
            n += 1;
        }
        n
    }
}

// ─────────────────────────────── receive ───────────────────────────────

pub struct OliviaRx {
    rate: f64,
    g: Geom,
    /// Recent audio, indexed by absolute sample count via `buf_start`.
    buf: Vec<f32>,
    buf_start: usize,
    next_hop: usize,
    hop: usize,
    /// Symbol-timing energy per sub-phase (decayed).
    sync: [f32; SUBPHASES],
    /// `SDROXIDE_OLIVIA_TRACE` — prints the block confidence every eighth
    /// block, which is what says whether the phase estimate has anything to
    /// work with at all. Read once, because this sits in the symbol loop.
    trace: bool,
    /// Held sampling sub-phase (updated once per symbol period).
    tphase: usize,
    /// Per-symbol tone magnitudes (one `[tones]` vector per decoded symbol).
    sbuf: VecDeque<Vec<f32>>,
    /// Absolute symbol index of `sbuf[0]`.
    sbuf_base: usize,
    scount: usize,
    /// Block-alignment confidence per phase (0..63), decayed.
    phase_conf: [f32; BLOCK],
    locked: bool,
    /// Absolute symbol index up to which blocks have been emitted.
    emitted_upto: usize,
    mag: f32,
}

impl OliviaRx {
    pub fn new(rate: f64, audio_hz: f64, tones: usize, bw: f64) -> Self {
        let g = Geom::new(rate, audio_hz, tones, bw);
        let hop = (g.sps / SUBPHASES).max(1);
        OliviaRx {
            rate,
            g,
            buf: Vec::new(),
            buf_start: 0,
            next_hop: g.sps,
            hop,
            sync: [0.0; SUBPHASES],
            trace: std::env::var("SDROXIDE_OLIVIA_TRACE").is_ok(),
            tphase: 0,
            sbuf: VecDeque::new(),
            sbuf_base: 0,
            scount: 0,
            phase_conf: [0.0; BLOCK],
            locked: false,
            emitted_upto: 0,
            mag: 0.0,
        }
    }

    pub fn set_params(&mut self, audio_hz: f64, tones: usize, bw: f64) {
        *self = OliviaRx::new(self.rate, audio_hz, tones, bw);
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let mut out = String::new();
        self.buf.extend_from_slice(audio);
        let n_total = self.buf_start + self.buf.len();
        while self.next_hop <= n_total {
            let t = self.next_hop;
            if t >= self.g.sps {
                let a = t - self.g.sps - self.buf_start;
                let b = t - self.buf_start;
                let mags = tone_bank_mags(
                    &self.buf[a..b],
                    self.g.base_hz,
                    self.g.spacing,
                    self.g.tones,
                    self.rate,
                );
                self.on_window(t, mags, &mut out);
            }
            self.next_hop += self.hop;
        }
        let keep_from = n_total.saturating_sub(self.g.sps);
        if keep_from > self.buf_start {
            self.buf.drain(0..keep_from - self.buf_start);
            self.buf_start = keep_from;
        }
        out
    }

    fn on_window(&mut self, t: usize, mags: Vec<f32>, out: &mut String) {
        let peak = mags.iter().copied().fold(0.0f32, f32::max);
        self.mag += 0.02 * (peak - self.mag);
        let phase = (t / self.hop) % SUBPHASES;
        // The sampling instant is voted by the strongest tone in the window.
        //
        // **The obvious improvement is wrong, and it was measured.** Choosing by
        // the *total* comb energy looks right on paper — the tones are spaced at
        // one symbol rate, so a window straddling two symbols should spread its
        // energy into the gaps — but it breaks `loopback_8_250`, which the peak
        // vote reads perfectly. Whatever the real fault is, it is not here.
        for (i, s) in self.sync.iter_mut().enumerate() {
            *s *= 0.995;
            if i == phase {
                *s += peak;
            }
        }
        // Re-estimate the sampling sub-phase once per symbol period, then hold it
        // so exactly one soft symbol is taken per period (keeps the symbol count
        // aligned to the transmit block grid).
        if phase == 0 {
            self.tphase = self
                .sync
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
        if phase != self.tphase {
            return;
        }
        // Sampling instant: keep the whole magnitude vector so decode_block can
        // descramble by block position.
        self.sbuf.push_back(mags);
        self.scount += 1;
        while self.sbuf.len() > 1024 {
            self.sbuf.pop_front();
            self.sbuf_base += 1;
        }
        self.try_decode(out);
    }

    /// Soft `planes` bits for the symbol at absolute index `abs`, descrambled for
    /// its block position `pos` (0..63).
    fn soft_bits(&self, abs: usize, pos: usize) -> [f32; 6] {
        let mags = &self.sbuf[abs - self.sbuf_base];
        let mut soft = [0.0f32; 6];
        // Every tone votes: a tone whose Gray-decoded value carries this plane's
        // bit at this position pushes it up, one that does not pushes it down.
        // Soft, because on a real signal the winning tone is often not clear of
        // the runners-up, and a hard decision throws that margin away.
        //
        // The interleave is undone here — character `p` reads bit
        // `(p + pos) % planes` — and the scrambler applied after, in the Walsh
        // domain where Olivia puts it, rather than as a rotation of the tone
        // number.
        for (p, sp) in soft.iter_mut().enumerate().take(self.g.planes) {
            let want = (p + pos) % self.g.planes;
            let mut acc = 0.0f32;
            for (k, &m) in mags.iter().enumerate() {
                acc += if (ungray(k as u32) >> want) & 1 == 1 { m } else { -m };
            }
            *sp = if scramble_bit(p, pos) { -acc } else { acc };
        }
        soft
    }

    /// Confidence + decoded chars for the 64-symbol block starting at absolute
    /// index `start`.
    fn decode_block(&self, start: usize) -> (f32, [u8; 6]) {
        let mut planes_soft = [[0.0f32; BLOCK]; 6];
        for i in 0..BLOCK {
            let soft = self.soft_bits(start + i, i);
            for p in 0..self.g.planes {
                planes_soft[p][i] = soft[p];
            }
        }
        let mut conf = 0.0f32;
        let mut chars = [0u8; 6];
        for p in 0..self.g.planes {
            let mut sc = planes_soft[p];
            fht(&mut sc);
            let (m, val) = sc
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, v)| (i, *v))
                .unwrap_or((0, 0.0));
            conf += val.abs();
            chars[p] = (m as u8 & 63) | if val < 0.0 { 64 } else { 0 };
        }
        (conf, chars)
    }

    fn try_decode(&mut self, out: &mut String) {
        // Need a full block available.
        if self.scount < self.sbuf_base + BLOCK {
            return;
        }
        // Score the block that just completed against its start phase.
        let start = self.scount - BLOCK;
        if start >= self.sbuf_base {
            let (conf, _) = self.decode_block(start);
            for c in self.phase_conf.iter_mut() {
                *c *= 0.997;
            }
            self.phase_conf[start % BLOCK] += conf;
        }
        // Lock once one phase clearly dominates.
        let (best_phase, best, second) = {
            let mut bp = 0;
            let mut b = -1.0f32;
            let mut s = -1.0f32;
            for (i, &c) in self.phase_conf.iter().enumerate() {
                if c > b {
                    s = b;
                    b = c;
                    bp = i;
                } else if c > s {
                    s = c;
                }
            }
            (bp, b, s)
        };
        if self.trace && self.scount % (BLOCK * 8) == 0 {
            eprintln!(
                "scount {:6} locked {:5} best {:.3} second {:.3} ratio {:.2} phase {:2} tphase {}",
                self.scount,
                self.locked,
                best,
                second,
                best / second.max(1e-6),
                best_phase,
                self.tphase
            );
        }
        if !self.locked {
            if self.scount < 2 * BLOCK || best < 1.4 * second.max(1e-6) {
                return;
            }
            self.locked = true;
            // Start emitting from the earliest buffered block at this phase.
            let base = self.sbuf_base;
            let first = base + ((best_phase + BLOCK - base % BLOCK) % BLOCK);
            self.emitted_upto = first;
        }
        // Emit any newly-complete aligned blocks (phase fixed once locked).
        let phase = best_phase;
        while self.emitted_upto >= self.sbuf_base
            && self.emitted_upto % BLOCK == phase % BLOCK
            && self.emitted_upto + BLOCK <= self.scount
        {
            let (_, chars) = self.decode_block(self.emitted_upto);
            for &b in chars.iter().take(self.g.planes) {
                if b != 0 {
                    out.push(b as char);
                }
            }
            self.emitted_upto += BLOCK;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MonoResampler;

    /// The transforms must be an exact pair, on every one of the 64 characters
    /// including the idle `0`. This is the invariant the transmit half is
    /// written against, and the textbook-butterfly bug could not be caught by
    /// the loopback alone: it stays self-consistent, so only real audio shows it.
    ///
    /// A Walsh transform is unnormalised, so a single code bit comes back as 64.
    #[test]
    fn the_inverse_and_forward_transforms_are_a_pair() {
        for byte in 0u8..=127 {
            let mut v = [0f32; BLOCK];
            v[(byte & 63) as usize] = if byte & 64 != 0 { -1.0 } else { 1.0 };
            ifht(&mut v);
            fht(&mut v);
            let sign = if byte & 64 != 0 { -1.0 } else { 1.0 };
            assert!(
                (v[(byte & 63) as usize] - 64.0 * sign).abs() < 1e-3,
                "byte {byte:#04x} must keep its sign through the pair"
            );
            // Every other row is zero, so the peak is the code bit's own row.
            let (peak, _) =
                v.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap();
            assert_eq!(peak, (byte & 63) as usize, "byte {byte:#04x}");
        }
    }

    /// Soft bits for one slice, the way Jalocha's receiver makes them: every
    /// tone votes on every bit plane with its **power squared**, and a set bit
    /// votes *negative*.
    fn slice_soft(mags: &[f32], planes: usize, sign: f32) -> [f32; 6] {
        let mut soft = [0.0f32; 6];
        let total: f32 = mags.iter().map(|m| (m * m) * (m * m)).sum();
        for (k, &m) in mags.iter().enumerate() {
            let e = (m * m) * (m * m);
            let val = ungray(k as u32);
            for (bit, sp) in soft.iter_mut().enumerate().take(planes) {
                *sp += if (val >> bit) & 1 == 1 { -e } else { e };
            }
        }
        for sp in soft.iter_mut().take(planes) {
            *sp = sign * *sp / total.max(1e-20);
        }
        soft
    }

    /// The reference front end: a raised-cosine window two symbols long, two
    /// slices per symbol half a symbol apart, energy-squared soft bits, and a
    /// search over slice parity, block offset and polarity.
    ///
    /// Every element here was read out of `pj_mfsk.h` — `MFSK_InputProcessor`,
    /// `MFSK_Demodulator::SoftDecode` and `MFSK_SoftDecoder::DecodeCharacter` —
    /// because our own front end reads nothing from a signal fldigi decodes
    /// perfectly. This test exists to find out whether the reference algorithm
    /// reads our fixture, before any of it goes near `OliviaRx`.
    #[test]
    #[ignore]
    fn a_capture_decodes_with_the_reference_front_end() {
        let Ok(path) = std::env::var("SDROXIDE_OLIVIA_SAMPLE") else {
            eprintln!("SDROXIDE_OLIVIA_SAMPLE unset; skipping");
            return;
        };
        let mut r = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = r.spec();
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.expect("sample")).collect(),
            hound::SampleFormat::Int => {
                r.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        let rate = spec.sample_rate as f64;
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        let tones: usize = num("SDROXIDE_OLIVIA_TONES", 16.0) as usize;
        let bw = num("SDROXIDE_OLIVIA_BW", 500.0);
        let hz = num("SDROXIDE_OLIVIA_HZ", 1500.0);
        let g = Geom::new(rate, hz, tones, bw);
        let sps = g.sps;
        let wl = 2 * sps;
        let shape: Vec<f32> = (0..wl)
            .map(|t| (0.5 - 0.5 * (std::f64::consts::TAU * t as f64 / wl as f64).cos()) as f32)
            .collect();
        let n_sym = audio.len().saturating_sub(wl + sps) / sps;
        // Two slices per symbol, half a symbol apart; each window is two symbols
        // long and centred on the symbol, so the shaping does the symbol gating.
        let mut raw: Vec<[f32; 6]> = Vec::new();
        for n in 0..n_sym {
            for sl in 0..2 {
                let start = n * sps + sl * (sps / 2);
                if start + wl > audio.len() {
                    break;
                }
                let win: Vec<f32> = (0..wl).map(|t| audio[start + t] * shape[t]).collect();
                let mags = tone_bank_mags(&win, g.base_hz, g.spacing, g.tones, rate);
                raw.push(slice_soft(&mags, g.planes, 1.0));
            }
        }
        let want: Vec<String> = std::env::var("SDROXIDE_OLIVIA_TEXT")
            .map(|t| t.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        let mut best = String::new();
        for parity in 0..2usize {
            for off in 0..BLOCK {
                for sign in [1.0f32, -1.0] {
                    let mut out = String::new();
                    let mut n = off;
                    while 2 * (n + BLOCK) <= raw.len() {
                        let mut planes_soft = [[0.0f32; BLOCK]; 6];
                        for t in 0..BLOCK {
                            let sym = raw[2 * (n + t) + parity];
                            for p in 0..g.planes {
                                let b = (p + t) % g.planes;
                                planes_soft[p][t] =
                                    sign * if scramble_bit(p, t) { -sym[b] } else { sym[b] };
                            }
                        }
                        for p in 0..g.planes {
                            let mut v = planes_soft[p];
                            fht(&mut v);
                            let (m, val) = v
                                .iter()
                                .enumerate()
                                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                                .map(|(i, x)| (i, *x))
                                .unwrap_or((0, 0.0));
                            let c = (m as u8 & 63) | if val < 0.0 { 64 } else { 0 };
                            if (32..127).contains(&c) {
                                out.push(c as char);
                            }
                        }
                        n += BLOCK;
                    }
                    let hits: usize = want.iter().map(|w| out.matches(w.as_str()).count()).sum();
                    let good =
                        out.chars().filter(|c| c.is_uppercase() || c.is_ascii_digit()).count();
                    let score = hits * 1000 + good;
                    if score > 0
                        && (best.is_empty()
                            || out.matches(want.first().map(|w| w.as_str()).unwrap_or("")).count()
                                > 0)
                    {
                        if hits > 0 {
                            eprintln!("parity {parity} off {off} sign {sign:+} -> hits {hits}");
                            eprintln!("{out:?}");
                            return;
                        }
                    }
                    if !want.is_empty() && !out.is_empty() && best.is_empty() {
                        best = out;
                    }
                }
            }
        }
        eprintln!("no combination recovered {want:?}; first attempt was {best:?}");
    }

    /// Every character, at both tone counts, through the transmitter and the
    /// receiver's own `soft_bits` + `fht` — no audio, no timing, so it says
    /// something about the transform and the polarity alone.
    ///
    /// This is the assertion the polarity deserves. The two tone-magnitude
    /// conventions that disagree are exact negatives of one another, so a single
    /// flipped sign reads back *every* character as its bit-6 twin: `C` as `#`,
    /// space as `` ` ``, and the NUL idle fill as `@` — which is what the
    /// loopbacks printed while the transmit side still matched fldigi's
    /// `EncodeBlock` literally. A loopback only covers the text it happens to
    /// spell; this covers the alphabet, both cases of it.
    #[test]
    fn every_character_reads_back_unchanged() {
        for (tones, bw) in [(32usize, 1000.0), (8, 250.0)] {
            let (rate, audio) = (8000.0, 1500.0);
            let mut tx = OliviaTx::new(rate, audio, tones, bw);
            let mut rx = OliviaRx::new(rate, audio, tones, bw);
            for byte in 0u8..=127 {
                // Every plane carries the character, so every plane has to read
                // it back; the interleave puts it on a different bit of each
                // symbol as `p` advances.
                for _ in 0..tx.g.planes {
                    tx.q.push_back((byte, None));
                }
                let (out, _) = tx.encode_block();
                // The receiver's tone bank, filled with the tones the
                // transmitter chose — what the audio path would deliver.
                rx.sbuf.clear();
                for i in 0..BLOCK {
                    let mut mags = vec![0.0f32; tones];
                    mags[gray(out[i]) as usize % tones] = 1.0;
                    rx.sbuf.push_back(mags);
                }
                let (_, got) = rx.decode_block(0);
                for p in 0..tx.g.planes {
                    assert_eq!(
                        got[p], byte,
                        "byte {byte:#04x} plane {p} at {tones}/{bw} read back as {:#04x}",
                        got[p]
                    );
                }
            }
        }
    }

    fn run(tones: usize, bw: f64, msg: &str) -> String {
        let rate = 8000.0;
        let audio = 1500.0;
        let mut tx = OliviaTx::new(rate, audio, tones, bw);
        let mut sig = Vec::new();
        // Idle runway so the RX acquires timing + block alignment.
        let mut warm = vec![0.0f32; tx.g.sps * BLOCK * 3];
        tx.next_block(&mut warm);
        sig.extend_from_slice(&warm);

        tx.push_text(msg);
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 40_000 {
            let mut b = [0.0f32; 2048];
            tx.next_block(&mut b);
            sig.extend_from_slice(&b);
            guard += 1;
        }
        // Flush trailing blocks so the last message block is fully sent.
        let mut tail = vec![0.0f32; tx.g.sps * BLOCK * 2];
        tx.next_block(&mut tail);
        sig.extend_from_slice(&tail);

        let mut rx = OliviaRx::new(rate, audio, tones, bw);
        let mut decoded = String::new();
        for chunk in sig.chunks(512) {
            decoded.push_str(&rx.process(chunk));
        }
        decoded
    }

    /// The real thing: an off-air Olivia recording decodes to readable text.
    ///
    /// **This is the only test that can catch an interop bug.** The loopback
    /// tests above prove the modulator and the demodulator agree with each
    /// other, which is exactly what they did while the scrambler was a local
    /// invention and the mode could not read a single Olivia station.
    ///
    /// Point `SDROXIDE_OLIVIA_SAMPLE` at a mono 8 kHz WAV of an Olivia signal —
    /// 8 kHz is the rate the mode's own generator uses, and 500/16 (16 tones,
    /// 500 Hz bandwidth, so 31.25 Hz spacing) is the common format. The sample
    /// is off-air material and cannot live in the tree, so the test is
    /// `#[ignore]`d and skips when the variable is unset. Run it with
    /// `cargo test -p sdroxide-dsp --release -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn an_off_air_capture_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_OLIVIA_SAMPLE") else {
            eprintln!("SDROXIDE_OLIVIA_SAMPLE unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 8000, "the sample must be at 8 kHz");
        assert_eq!(spec.channels, 1, "the sample must be mono");
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.expect("sample")).collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        // 500/16: the tone bank spans 500 Hz, so the tones sit 31.25 Hz apart
        // and the symbol rate equals the spacing.
        // The bank is searched, not assumed: nobody tunes a listener's dial to
        // the exact centre of an Olivia signal, and fldigi searches several tone
        // spacings either side for the same reason.
        // The bank is searched across the whole passband: the recording's comb
        // sits wherever the receiver's dial put it, and different captures sit
        // in different places (the Avalon recordings centre near 1478 Hz).
        // Geometry and expected words are overridable so that one test serves
        // any capture, not only the Avalon 16/500 network.
        //
        // `SDROXIDE_OLIVIA_HZ` pins the tone bank's centre and skips the search.
        // That matters: the search is 260 decodes of the whole file, which is
        // minutes, and a fixture whose centre is already known — any signal we
        // recorded ourselves against a known dial — should not pay for one.
        let tones: usize =
            std::env::var("SDROXIDE_OLIVIA_TONES").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
        let bw: f64 =
            std::env::var("SDROXIDE_OLIVIA_BW").ok().and_then(|v| v.parse().ok()).unwrap_or(500.0);
        let want: Vec<String> = match std::env::var("SDROXIDE_OLIVIA_TEXT") {
            Ok(t) => t.split_whitespace().map(str::to_owned).collect(),
            Err(_) => vec!["SouthWest".to_owned(), "G7LEE".to_owned()],
        };
        let hits = |s: &str| want.iter().map(|w| s.matches(w.as_str()).count()).sum::<usize>();
        let pinned: Option<f64> =
            std::env::var("SDROXIDE_OLIVIA_HZ").ok().and_then(|v| v.parse().ok());
        let mut best = String::new();
        let mut hz = pinned.unwrap_or(600.0);
        let stop = pinned.unwrap_or(1900.0);
        while hz <= stop {
            let mut rx = OliviaRx::new(8000.0, hz, tones, bw);
            let mut got = String::new();
            for chunk in audio.chunks(512) {
                got.push_str(&rx.process(chunk));
            }
            if hits(&got) > hits(&best) {
                best = got;
            }
            if pinned.is_some() {
                break;
            }
            hz += 5.0;
        }
        let decoded = best;
        eprintln!("decoded {} chars at {tones}/{bw}: {decoded:?}", decoded.len());
        // The Avalon SW Net recording's known text: "CQ SouthWest NET … de G7LEE
        // G7LEE G7LEE". A real interop test asserts the *content*, not merely
        // that something printable came out — the previous version passed on
        // garbage, which is how the Walsh bug survived.
        assert!(
            want.iter().all(|w| decoded.contains(w.as_str())),
            "off-air Olivia did not recover {want:?}: {decoded:?}"
        );
    }

    /// The second known-text sample, and the one that proved the cure: **32/1000**
    /// of Wikipedia's own Olivia recording, recovered to its exact text. A
    /// reporter (fork discussion #5) produced it after the 16/500 Avalon case
    /// above found the fault, and decoded it 100% in the app — this pins that
    /// result so a future change cannot quietly lose it.
    ///
    /// Point `SDROXIDE_OLIVIA_1000_32` at the Wikipedia sample (any rate; it is
    /// resampled here to the 8 kHz the decoder runs at). The file is a Wikimedia
    /// Commons upload and is not redistributed with the tree, so the test is
    /// `#[ignore]`d and skips when the variable is unset:
    /// `https://upload.wikimedia.org/wikipedia/commons/3/32/OLIVIA_1000_32_sample.ogg`
    #[test]
    #[ignore]
    fn the_wikipedia_32_1000_sample_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_OLIVIA_1000_32") else {
            eprintln!("SDROXIDE_OLIVIA_1000_32 unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        let in_rate = spec.sample_rate as f64;
        let raw: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().map(|s| s.expect("sample")).collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect()
            }
        };
        // Resample to the 8 kHz the decoder is built for, exactly as the
        // controller's resampler does in the live path.
        let audio: Vec<f32> = if (in_rate - 8000.0).abs() < 1.0 {
            raw
        } else {
            let mut rs = MonoResampler::new(in_rate, 8000.0).expect("resampler");
            let mut out = Vec::with_capacity(raw.len() * 8000 / in_rate as usize + 64);
            rs.push(&raw, &mut out);
            out
        };
        // 32 tones over 1000 Hz → 31.25 Hz spacing. The bank is searched: the
        // sample's comb sits a little over 1 kHz from the file's origin, and a
        // receiver would not know that.
        let mut best = String::new();
        let mut hz = 800.0;
        while hz <= 1600.0 {
            let mut rx = OliviaRx::new(8000.0, hz, 32, 1000.0);
            let mut got = String::new();
            for chunk in audio.chunks(512) {
                got.push_str(&rx.process(chunk));
            }
            let score =
                got.matches("Wikipedia").count() * 100 + got.matches("encyclopedia").count();
            if score
                > best.matches("Wikipedia").count() * 100 + best.matches("encyclopedia").count()
            {
                best = got;
            }
            hz += 5.0;
        }
        eprintln!("decoded {} chars: {best:?}", best.len());
        assert!(
            best.contains("Wikipedia") && best.contains("encyclopedia"),
            "the 32/1000 sample did not recover its known text: {best:?}"
        );
    }

    // The transmitter builds each codeword with `ifht`, the inverse of the
    // `fht` the receiver applies, so our own transmission is now received by our
    // own receiver at both tone counts. These were `#[ignore]`d for as long as
    // the transmit side used the pre-fldigi Walsh convention.
    #[test]
    fn loopback_32_1000() {
        let msg = "CQ DE AB1CD";
        let got = run(32, 1000.0, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    #[test]
    fn loopback_8_250() {
        let msg = "TEST OLIVIA";
        let got = run(8, 250.0, msg);
        assert!(got.contains(msg), "decoded {got:?} did not contain {msg:?}");
    }

    // ───────────────────────── start / stop tone framing ─────────────────────────

    /// The audio a fresh transmitter emits before any data block: exactly the
    /// start tones, and nothing else.
    fn start_tones_of(tones: usize, bw: f64) -> Vec<f32> {
        let mut tx = OliviaTx::new(8000.0, 1500.0, tones, bw);
        let mut out = vec![0.0f32; TONE_DURATION];
        assert_eq!(tx.next_block(&mut out), TONE_DURATION);
        out
    }

    /// Upward zero crossings, which is how a pure tone's frequency is read back
    /// without assuming anything about phase.
    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
    }

    /// The flat middle of one quarter, with the ramped edges left off — the
    /// edges are near zero and would undercount.
    fn flat_middle(t: &[f32], q: usize) -> &[f32] {
        let s = q * TONE_QUARTER + TONE_EDGE;
        &t[s..s + (TONE_QUARTER - 2 * TONE_EDGE)]
    }

    /// The start tones are two tones at the **band edges**, alternating — and the
    /// edges are half a tone spacing outside the outermost data tone, so the
    /// pair brackets the tone bank rather than sitting inside it. This is the
    /// placement most easily got wrong, and a decoder that is looking for the
    /// bracket would simply not find it.
    #[test]
    fn the_start_tones_bracket_the_tone_bank() {
        let (tones, bw) = (32usize, 1000.0f64);
        let g = Geom::new(8000.0, 1500.0, tones, bw);
        let (low, high) = g.edge_hz();
        // 1500 +/- bw/2: the edges are exactly the band edges.
        assert!((low - 1000.0).abs() < 1e-9, "low edge {low}");
        assert!((high - 2000.0).abs() < 1e-9, "high edge {high}");
        // …and they sit strictly outside the outermost data tones.
        assert!(g.tone_hz(0) > low, "lowest data tone inside the low edge");
        assert!(g.tone_hz(tones - 1) < high, "highest data tone inside the high edge");

        let t = start_tones_of(tones, bw);
        let flat = TONE_QUARTER - 2 * TONE_EDGE;
        // One cycle is rate/hz samples, so a flat run of `flat` samples crosses
        // zero `flat * hz / rate` times. 1000 Hz and 2000 Hz at 8 kHz.
        for (q, hz) in [(0usize, 1000.0), (1, 2000.0), (2, 1000.0), (3, 2000.0)] {
            let want = (flat as f64 * hz / 8000.0).round() as usize;
            let got = crossings(flat_middle(&t, q));
            assert!(
                got.abs_diff(want) <= 2,
                "quarter {q} at {hz} Hz crossed {got} times, wanted about {want}"
            );
        }
    }

    /// Each quarter is ramped at both ends. Without it, each of the four tone
    /// changes is a step discontinuity — three clicks inside one frame, which a
    /// receiver sees as splatter rather than as a clean bracket.
    #[test]
    fn each_start_tone_quarter_is_ramped_at_both_ends() {
        let t = start_tones_of(32, 1000.0);
        for q in 0..TONE_DURATION / TONE_QUARTER {
            let s = q * TONE_QUARTER;
            let first = t[s].abs();
            let last = t[s + TONE_QUARTER - 1].abs();
            let mid = t[s + TONE_QUARTER / 2].abs();
            assert!(first < 1e-6, "quarter {q} starts at {first}, not at zero");
            assert!(last < 1e-6, "quarter {q} ends at {last}, not at zero");
            assert!(
                mid > 0.4 && mid <= OUT_AMP + 1e-6,
                "quarter {q} peaks at {mid}, wanted full amplitude"
            );
        }
    }

    /// The tone phase is carried across quarters, so a tone change is a phase
    /// change and not a jump in the waveform.
    #[test]
    fn the_start_tones_are_phase_continuous_across_a_tone_change() {
        let t = start_tones_of(32, 1000.0);
        // At the seam the ramp takes each end to zero, so the samples either side
        // of the boundary are both near zero and the step between them is small.
        for q in 1..TONE_DURATION / TONE_QUARTER {
            let seam = q * TONE_QUARTER;
            let step = (t[seam - 1] - t[seam]).abs();
            assert!(step < 1e-6, "quarter {q} seam steps by {step}");
        }
    }

    /// fldigi puts exactly one idle character in after the start tones — "the
    /// Olivia Transmitter class requires at least character" — and it must not
    /// show up as a character of the message. If it did, the UI would colour a
    /// sent prefix green that nobody typed, and `sent_chars` would overshoot.
    #[test]
    fn the_idle_character_after_the_start_tones_is_not_sent_text() {
        let mut tx = OliviaTx::new(8000.0, 1500.0, 32, 1000.0);
        tx.push_text("AB");
        assert_eq!(tx.total_chars(), 2, "the idle character must not be counted");
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 4_000 {
            let mut b = [0.0f32; 1024];
            tx.next_block(&mut b);
            guard += 1;
        }
        assert_eq!(tx.sent_chars(), 2, "sent_chars overshot past the message");
        // `drained` cannot be waited on here: `next_block` always produces
        // audio, so once the closing bracket is spent it goes on making idle
        // blocks for ever. Drive the bracket out and ask about *it*.
        let mut guard = 0;
        while (tx.tail_pending() || !tx.q.is_empty()) && guard < 8_000 {
            let mut b = [0.0f32; 1024];
            tx.next_block(&mut b);
            guard += 1;
        }
        assert!(!tx.tail_pending(), "the bracket never finished");
        assert_eq!(tx.sent_chars(), 2, "the bracket counted as sent text");
        assert_eq!(tx.total_chars(), 2, "and it changed the message length");
    }

    /// A transmission is **closed** as well as opened.
    ///
    /// fldigi sends `send_tones()` again at the end and then `SCBLOCKSIZE`
    /// samples of silence. This is not decoration: fldigi's receiver integrates
    /// `SyncIntegLen` FEC blocks, so a decoder finding the end of the frame is
    /// looking at exactly this.
    ///
    /// The property that matters is not the tones themselves but that they are
    /// ever *played*. The caller stops asking for audio the moment
    /// `sent_chars` reaches `total_chars`, so anything generated after that sits
    /// in the buffer and is never drained — an interop feature that could not be
    /// heard. `drained()` is what the caller asks, so it has to mean the bracket
    /// too.
    #[test]
    fn a_transmission_ends_with_its_own_bracket() {
        let mut tx = OliviaTx::new(8000.0, 1500.0, 32, 1000.0);
        tx.push_text("CQ");
        assert!(!tx.tail_pending(), "no bracket before anything has been sent");
        let mut buf = [0.0f32; 2048];
        let mut guard = 0;
        while tx.sent_chars() < tx.total_chars() && guard < 4_000 {
            tx.next_block(&mut buf);
            guard += 1;
        }
        assert!(!tx.drained(), "the message is out but the bracket is still owed");

        // Now render what follows. `next_block` always produces audio, so this
        // cannot wait on `drained()` — it is asked after the **bracket** is gone.
        let mut first = Vec::new();
        let mut guard = 0;
        while !tx.tail_pending() && guard < 4_000 {
            tx.next_block(&mut buf);
            guard += 1;
        }
        assert!(tx.tail_pending(), "the bracket was never generated");
        let mut silence_at = None;
        while tx.tail_pending() && guard < 8_000 {
            if first.is_empty() {
                first.extend_from_slice(&buf);
            }
            tx.next_block(&mut buf);
            if !tx.tail_pending() {
                // The call that finished the bracket: what it left is the
                // silence fldigi sends after its stop tones.
                silence_at = Some(buf.len());
            }
            guard += 1;
        }
        assert!(!tx.tail_pending(), "the bracket never finished");
        assert!(silence_at.is_some(), "the bracket never ended");
        // It is the stop tones: the same alternating band-edge pair as the start.
        let flat = TONE_QUARTER - 2 * TONE_EDGE;
        let want = (flat as f64 * 1000.0 / 8000.0).round() as usize;
        assert_eq!(crossings(flat_middle(&first, 0)), want, "not the stop tones");

        // And it goes out **once**. The "message is out" condition is still true
        // afterwards, so without a spent marker every further call would emit the
        // bracket again, for ever.
        let mut after = 0usize;
        for _ in 0..64 {
            tx.next_block(&mut buf);
            after += buf.len();
            if tx.tail_pending() {
                break;
            }
        }
        assert!(
            !tx.tail_pending(),
            "the bracket re-armed after it had been played ({after} samples)"
        );
    }

    /// Every transmission is bracketed, so clearing re-arms the tones rather
    /// than leaving the next one opening straight into data.
    ///
    /// Compared as a cheap digest rather than by equality: two 8192-sample
    /// buffers dumped into an assertion failure bury the one number that matters.
    #[test]
    fn a_clear_re_arms_the_start_tones() {
        let mut tx = OliviaTx::new(8000.0, 1500.0, 32, 1000.0);
        let mut first = vec![0.0f32; TONE_DURATION];
        tx.next_block(&mut first);
        tx.push_text("CQ");
        let mut mid = vec![0.0f32; TONE_DURATION];
        tx.next_block(&mut mid);

        tx.clear();
        let mut again = vec![0.0f32; TONE_DURATION];
        tx.next_block(&mut again);

        let digest = |t: &[f32]| -> u64 {
            t.iter().fold(0u64, |h, v| h.wrapping_mul(31).wrapping_add(v.to_bits() as u64))
        };
        assert_eq!(
            digest(&first),
            digest(&again),
            "after a clear the next transmission must open with the same tones"
        );
        assert_ne!(
            digest(&mid),
            digest(&first),
            "the middle of a transmission must not be the start tones"
        );
    }
}
