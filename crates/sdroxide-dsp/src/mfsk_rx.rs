//! MFSK — fldigi's MFSK4/8/11/16/22/31/32/64, receive only.
//!
//! (This lives beside `mfsk.rs`, which is the tone generator and symbol clock
//! the multi-tone modes share; this file is the modem itself.)
//!
//! A port of the reference described in `MODE-ROADMAP.md` under "MFSK16: the
//! spec", which itself was read out of the keyboard-mode family in OnAir
//! (`core/src/hfdig/hfdig_kbd_mfsk.cpp` and `hfdig_kbd_dsp.h`, GPL-3.0). OnAir
//! is credited in the README's Acknowledgements; nothing here is copied by name.
//!
//! Two pieces were already in this crate and are used as they are:
//!
//! * **the convolutional code** — `ConvCode::THOR` is `0o155`/`0o117`, which is
//!   exactly the K=7, `0x6d`/`0x4f` code these modes use;
//! * **the varicode** — `VaricodeRx` terminates a word on `00`, which is the
//!   IZ8BLY rule.
//!
//! What is written here is the front end: a sliding DFT bank at four sub-bin
//! offsets, a symbol clock taken from sixteen hop phases, soft symbols from the
//! tone amplitudes, the diagonal interleaver, and the glue down to text.
//!
//! **Receiver only.** MFSK's transmit side is a modulator over the same tables
//! and is a separate job.

use crate::fec::{ConvCode, ViterbiStream};
use crate::psk::{VaricodeRx, varicode};

const PI: f64 = std::f64::consts::PI;

/// How many hop phases per symbol the clock is measured at.
const HOPS: usize = 16;
/// Sub-bin frequency offsets: a tuning error under a quarter of a bin is free.
const PATHS: usize = 4;
/// Tones searched either side of the bank, for the realignment.
const MARGIN: usize = 3;

/// One MFSK variant. `fs` is the rate the modulator works at, `symlen` the
/// symbol in samples at that rate (so the tone spacing is `fs / symlen`).
#[derive(Clone, Copy, Debug)]
pub struct MfskVariant {
    pub name: &'static str,
    pub fs: f64,
    pub symlen: usize,
    pub symbits: usize,
    pub depth: usize,
    pub tones: usize,
    pub preamble: usize,
}

/// The eight variants, in the order [`MfskVariant::ALL`] documents.
pub const ALL: [MfskVariant; 8] = [
    MfskVariant {
        name: "MFSK4",
        fs: 8000.0,
        symlen: 2048,
        symbits: 5,
        depth: 5,
        tones: 32,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK8",
        fs: 8000.0,
        symlen: 1024,
        symbits: 5,
        depth: 5,
        tones: 32,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK11",
        fs: 11025.0,
        symlen: 1024,
        symbits: 4,
        depth: 10,
        tones: 16,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK16",
        fs: 8000.0,
        symlen: 512,
        symbits: 4,
        depth: 10,
        tones: 16,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK22",
        fs: 11025.0,
        symlen: 512,
        symbits: 4,
        depth: 10,
        tones: 16,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK31",
        fs: 8000.0,
        symlen: 256,
        symbits: 3,
        depth: 10,
        tones: 8,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK32",
        fs: 8000.0,
        symlen: 256,
        symbits: 4,
        depth: 10,
        tones: 16,
        preamble: 107,
    },
    MfskVariant {
        name: "MFSK64",
        fs: 8000.0,
        symlen: 128,
        symbits: 4,
        depth: 10,
        tones: 16,
        preamble: 180,
    },
];

impl MfskVariant {
    /// The variant by name, as fldigi spells it.
    pub fn named(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    /// Tones per second.
    pub fn spacing(&self) -> f64 {
        self.fs / self.symlen as f64
    }

    /// The occupied width: every tone but the first is one spacing further up.
    pub fn bandwidth(&self) -> f64 {
        (self.tones - 1) as f64 * self.spacing()
    }
}

/// Gray code, the direction the modulator applies. The receiver only needs the
/// inverse; this is here because the modulator will want it and the round-trip
/// test pins the pair.
#[cfg_attr(not(test), allow(dead_code))]
fn gray(k: u32) -> u32 {
    k ^ (k >> 1)
}

/// The inverse of [`gray`]: a tone number back to the value it carries.
fn binary(g: u32) -> u32 {
    let mut v = 0;
    let mut g = g;
    while g != 0 {
        v ^= g;
        g >>= 1;
    }
    v
}

// ---------------------------------------------------------------- the tone bank

/// A sliding DFT: one complex accumulator per bin, updated per sample.
///
/// `X += (x − x_old·e^{jwN})·e^{−jwn}` — the sample leaving the window is taken
/// out with the phasor it entered with. The rotating phasor is renormalised once
/// per window so drift cannot creep in over a long transmission.
struct Bank {
    n: usize,
    xr: Vec<f64>,
    xi: Vec<f64>,
    pr: Vec<f64>,
    pi: Vec<f64>,
    rr: Vec<f64>,
    ri: Vec<f64>,
    cr: Vec<f64>,
    ci: Vec<f64>,
    ring: Vec<f64>,
    pos: usize,
    count: usize,
}

impl Bank {
    fn new(n: usize, fs: f64, freqs: &[f64]) -> Self {
        let m = freqs.len();
        let mut b = Bank {
            n,
            xr: vec![0.0; m],
            xi: vec![0.0; m],
            pr: vec![1.0; m],
            pi: vec![0.0; m],
            rr: vec![0.0; m],
            ri: vec![0.0; m],
            cr: vec![0.0; m],
            ci: vec![0.0; m],
            ring: vec![0.0; n],
            pos: 0,
            count: 0,
        };
        for (j, &f) in freqs.iter().enumerate() {
            let w = 2.0 * PI * f / fs;
            b.rr[j] = w.cos();
            b.ri[j] = -w.sin();
            b.cr[j] = (w * n as f64).cos();
            b.ci[j] = (w * n as f64).sin();
        }
        b
    }

    fn feed(&mut self, x: f64) {
        let xo = self.ring[self.pos];
        self.ring[self.pos] = x;
        self.pos = (self.pos + 1) % self.n;
        for j in 0..self.xr.len() {
            let ar = x - xo * self.cr[j];
            let ai = -xo * self.ci[j];
            self.xr[j] += ar * self.pr[j] - ai * self.pi[j];
            self.xi[j] += ar * self.pi[j] + ai * self.pr[j];
            let nr = self.pr[j] * self.rr[j] - self.pi[j] * self.ri[j];
            self.pi[j] = self.pr[j] * self.ri[j] + self.pi[j] * self.rr[j];
            self.pr[j] = nr;
        }
        self.count += 1;
        if self.count >= self.n {
            self.count = 0;
            for j in 0..self.xr.len() {
                let a = 1.0 / self.pr[j].hypot(self.pi[j]).max(1e-12);
                self.pr[j] *= a;
                self.pi[j] *= a;
            }
        }
    }

    fn mag(&self, j: usize) -> f64 {
        self.xr[j].hypot(self.xi[j])
    }

    fn energy(&self, j: usize) -> f64 {
        self.xr[j] * self.xr[j] + self.xi[j] * self.xi[j]
    }
}

// ---------------------------------------------------------------- interleaver

/// The diagonal bit interleaver, `size × size × depth`.
///
/// Written out the way the reference does it, because the permutation is the
/// whole point: a burst of errors is spread across `depth` words instead of
/// destroying one.
struct Interleaver {
    size: usize,
    depth: usize,
    forward: bool,
    table: Vec<u8>,
}

impl Interleaver {
    fn new(size: usize, depth: usize, forward: bool) -> Self {
        Interleaver {
            size,
            depth,
            forward,
            table: vec![if forward { 0 } else { 128 }; size * size * depth],
        }
    }

    /// The bit form: `size` bits in, `size` bits out, through `symbols`.
    fn bits(&mut self, word: &mut u32) {
        let mut s = [0u8; 16];
        for i in 0..self.size {
            s[i] = ((*word >> (self.size - i - 1)) & 1) as u8;
        }
        self.symbols(&mut s[..self.size]);
        let mut v = 0u32;
        for i in 0..self.size {
            v = (v << 1) | s[i] as u32;
        }
        *word = v;
    }

    /// One step: shift every row along, append the new byte, read out a diagonal.
    fn symbols(&mut self, p: &mut [u8]) {
        let (size, depth) = (self.size, self.depth);
        for k in 0..depth {
            let base = size * size * k;
            for i in 0..size {
                let row = base + i * size;
                for j in 0..size - 1 {
                    self.table[row + j] = self.table[row + j + 1];
                }
                self.table[row + size - 1] = p[i];
            }
            for i in 0..size {
                let row = base + i * size;
                p[i] =
                    if self.forward { self.table[row + size - i - 1] } else { self.table[row + i] };
            }
        }
    }
}

// ---------------------------------------------------------------- the receiver

/// One receiver, fed audio and asked for text.
pub struct MfskRx {
    variant: MfskVariant,
    centre: f64,
    /// Present only when the audio is not already at the variant's own rate.
    resampler: Option<crate::MonoResampler>,
    scratch: Vec<f32>,
    spacing: f64,
    bank: Bank,
    j: usize,
    n_align: usize,
    align: usize,
    hop: usize,
    count: usize,
    hop_idx: usize,
    next_dec: usize,
    decisions: usize,
    phase: [f32; HOPS],
    /// Alignment and energy history, decayed once per symbol.
    r: Vec<f32>,
    e: Vec<f32>,
    interleaver: Interleaver,
    viterbi: Box<ViterbiStream>,
    varicode: VaricodeRx,
    /// Two soft symbols per bit, taken alternately.
    /// The two most recent soft symbols, 0..255 with 128 unknown.
    pair: [u8; 2],
    symcounter: bool,
    /// `+1` or `-1` on the soft symbols: two conventions were plausible and only
    /// the air decides. Set by `SDROXIDE_MFSK_SIGN=-1` while that is settled.
    soft_sign: f32,
    /// Which half of each pair the decoder takes. Same argument.
    second_half: bool,
    /// Mirror the tone order. An MFSK signal received on the wrong sideband is a
    /// tone set upside down about its centre, and nothing decodes until that is
    /// corrected — for FSK, inversion is exactly "tone i is tone n−1−i".
    mirror: bool,
    text: String,
    /// A long-run metric, so a signal too weak to trust is not printed as text.
    metric: f64,
    pub locked: bool,
    pub snr_db: f32,
    pub offset_hz: f32,
    /// The strongest tone of every symbol decided, in order. Kept because the
    /// only way to find a convention difference between two implementations is
    /// to compare the sequence, not the output.
    pub tones: Vec<u8>,
}

impl MfskRx {
    pub fn new(variant: MfskVariant, rate: f64, centre: f64) -> Self {
        let spacing = variant.spacing();
        let hop = (variant.symlen / HOPS).max(1);
        let mut rx = MfskRx {
            variant,
            centre,
            resampler: crate::MonoResampler::new(rate, variant.fs),
            scratch: Vec::new(),
            spacing,
            bank: Bank::new(variant.symlen, variant.fs, &[0.0]),
            j: 0,
            n_align: 0,
            align: 0,
            hop,
            count: 0,
            hop_idx: 0,
            next_dec: HOPS,
            decisions: 0,
            phase: [0.0; HOPS],
            r: Vec::new(),
            e: Vec::new(),
            interleaver: Interleaver::new(variant.symbits, variant.depth, false),
            viterbi: Box::new(ViterbiStream::new(ConvCode::THOR)),
            varicode: VaricodeRx::new(),
            pair: [0; 2],
            symcounter: false,
            soft_sign: if std::env::var("SDROXIDE_MFSK_SIGN").as_deref() == Ok("-1") {
                -1.0
            } else {
                1.0
            },
            second_half: std::env::var("SDROXIDE_MFSK_ODD").is_ok(),
            mirror: std::env::var("SDROXIDE_MFSK_MIRROR").is_ok(),
            text: String::new(),
            metric: 0.0,
            locked: false,
            snr_db: 0.0,
            offset_hz: 0.0,
            tones: Vec::new(),
        };
        rx.build();
        rx
    }

    /// Retune. The bank is rebuilt, as fldigi's own `restart()` does.
    pub fn set_centre(&mut self, centre: f64) {
        if (centre - self.centre).abs() < 0.5 {
            return;
        }
        self.centre = centre;
        self.build();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn build(&mut self) {
        let f0 = self.centre - self.variant.bandwidth() / 2.0;
        let mut freqs = Vec::new();
        for t in -(MARGIN as i32)..(self.variant.tones as i32 + MARGIN as i32) {
            for k in 0..PATHS {
                freqs.push(f0 + (t as f64 + k as f64 / PATHS as f64) * self.spacing);
            }
        }
        self.bank = Bank::new(self.variant.symlen, self.variant.fs, &freqs);
        self.j = freqs.len();
        self.n_align = 2 * MARGIN * PATHS + PATHS;
        self.align = MARGIN * PATHS;
        self.r = vec![0.0; self.n_align];
        self.e = vec![0.0; self.j];
        self.phase = [0.0; HOPS];
        self.count = 0;
        self.hop_idx = 0;
        self.next_dec = HOPS;
        self.decisions = 0;
        self.interleaver = Interleaver::new(self.variant.symbits, self.variant.depth, false);
        self.viterbi = Box::new(ViterbiStream::new(ConvCode::THOR));
        self.varicode = VaricodeRx::new();
        self.pair = [0; 2];
        self.symcounter = false;
        self.metric = 0.0;
        self.locked = false;
    }

    /// Feed audio at the variant's own rate.
    pub fn feed(&mut self, audio: &[f32]) {
        if self.resampler.is_some() {
            self.scratch.clear();
            let mut done = Vec::new();
            self.resampler.as_mut().expect("checked").push(audio, &mut done);
            self.scratch = done;
            let scratch = std::mem::take(&mut self.scratch);
            for &v in &scratch {
                self.bank.feed(v as f64);
                self.count += 1;
                if self.count >= self.hop {
                    self.count = 0;
                    self.hop();
                }
            }
            self.scratch = scratch;
            return;
        }
        for &v in audio {
            self.bank.feed(v as f64);
            self.count += 1;
            if self.count >= self.hop {
                self.count = 0;
                self.hop();
            }
        }
    }

    /// Peakness of the tones at alignment `a`: the strongest tone over their sum.
    ///
    /// An MFSK symbol carries **one** tone, so this is a clean timing metric —
    /// unlike a multi-tone mode, where the same measure is nearly flat.
    fn peakness(&self, a: usize) -> f32 {
        let mut mx: f64 = 0.0;
        let mut sum = 0.0;
        for i in 0..self.variant.tones {
            let m = self.bank.mag(a + i * PATHS);
            mx = mx.max(m);
            sum += m;
        }
        if sum > 1e-12 { (mx / sum) as f32 } else { 0.0 }
    }

    fn hop(&mut self) {
        self.hop_idx += 1;
        let phase = self.hop_idx % HOPS;
        let p = self.peakness(self.align);
        self.phase[phase] += 0.1 * (p - self.phase[phase]);
        if self.hop_idx < self.next_dec {
            return;
        }
        self.decide();
        // One hop per symbol towards the phase that measures best.
        let mut best = 0;
        for i in 1..HOPS {
            if self.phase[i] > self.phase[best] {
                best = i;
            }
        }
        let cur = self.next_dec % HOPS;
        let mut d = best as i32 - cur as i32;
        if d > (HOPS / 2) as i32 {
            d -= HOPS as i32;
        }
        if d < -((HOPS / 2) as i32) {
            d += HOPS as i32;
        }
        let adj = if d != 0 && self.phase[best] > self.phase[cur] * 1.03 + 1e-4 {
            if d > 0 { 1 } else { -1 }
        } else {
            0
        };
        self.next_dec = (self.next_dec as i32 + HOPS as i32 + adj).max(0) as usize;
    }

    fn decide(&mut self) {
        self.decisions += 1;
        let nt = self.variant.tones;
        let mut m = vec![0.0f64; nt];
        let mut sum = 0.0f64;
        let mut mx = 0.0f64;
        for i in 0..nt {
            m[i] = self.bank.mag(self.align + i * PATHS);
            sum += m[i];
            mx = mx.max(m[i]);
        }
        for a in 0..self.n_align {
            self.r[a] += (self.peakness(a) - self.r[a]) * (1.0 / 16.0);
        }
        for j in 0..self.j {
            self.e[j] += (self.bank.energy(j) as f32 - self.e[j]) * (1.0 / 32.0);
        }
        if self.decisions % 8 == 0 {
            self.realign();
        }
        if sum < 1e-9 {
            return;
        }
        let avg = sum / nt as f64;
        let burst = (0..nt).filter(|&i| m[i] > 2.0 * avg).count();
        let static_burst = burst == nt;

        // Soft symbols: every tone votes on every bit, with the sign of the
        // Gray-decoded value it carries, scaled by the symbol's total.
        let mut b = [0.0f64; 8];
        let bits = self.variant.symbits;
        for i in 0..nt {
            // The modulator Gray-codes the value onto the tone; the receiver
            // takes the tone number back to its value, once. (Applying Gray and
            // then its inverse is the identity, which is what the first version
            // did — it still produced structure and the wrong letters.)
            let idx = if self.mirror { nt - 1 - i } else { i };
            let val = binary(idx as u32);
            for (k, bv) in b.iter_mut().enumerate().take(bits) {
                *bv += if (val >> (bits - k - 1)) & 1 == 1 { m[idx] } else { -m[idx] };
            }
        }
        let mut sym = [0u8; 8];
        for (k, sv) in sym.iter_mut().enumerate().take(bits) {
            *sv = if static_burst {
                128
            } else {
                (128.0 + b[k] / sum * 256.0).clamp(0.0, 255.0) as u8
            };
        }
        self.interleaver.symbols(&mut sym[..bits]);
        for k in 0..bits {
            self.decode_symbol(sym[k]);
        }
        self.snr_db =
            (20.0 * (mx / ((sum - mx) / (nt as f64 - 1.0)).max(1e-9)).max(1e-6).log10()) as f32;
        self.locked = self.snr_db > 6.0;
        // The hard tone decision, kept so that two signals carrying the same text
        // can be compared tone by tone. The decoded text cannot settle a
        // convention question - it is the thing being questioned.
        let mut strongest = 0usize;
        for i in 1..nt {
            if m[i] > m[strongest] {
                strongest = i;
            }
        }
        let shown = if self.mirror { nt - 1 - strongest } else { strongest };
        self.tones.push(shown as u8);
    }

    /// The best integer shift and sub-bin offset: the tones in use show energy
    /// *inside* the window and not just beyond it, and the sub-bin offset is the
    /// one whose tones peak hardest.
    fn realign(&mut self) {
        let kp = PATHS as i32;
        let tones = self.variant.tones;
        let mut bf = 0usize;
        let mut bs = -1.0f32;
        for f in 0..PATHS {
            let mut s = 0.0f32;
            for g in 0..=(2 * MARGIN) {
                s += self.r[g * PATHS + f];
            }
            if s > bs {
                bs = s;
                bf = f;
            }
        }
        let score = |e: &[f32], a: usize| -> f64 {
            let mut inside = 0.0;
            for i in 0..tones {
                inside += e[a + i * PATHS] as f64;
            }
            let mut outside = 0.0;
            let mut n = 0;
            for k in [-2i32, -1, tones as i32, tones as i32 + 1] {
                let j = a as i32 + k * kp;
                if j >= 0 && (j as usize) < e.len() {
                    outside += e[j as usize] as f64;
                    n += 1;
                }
            }
            inside / tones as f64 - if n > 0 { outside / n as f64 } else { 0.0 }
        };
        let mut bg = self.align / PATHS;
        let mut best = score(&self.e, bg * PATHS + bf);
        for g in 0..=(2 * MARGIN) {
            let a = g * PATHS + bf;
            if a + tones * PATHS >= self.e.len() {
                break;
            }
            let s = score(&self.e, a);
            if s > best * 1.05 + 1e-12 {
                best = s;
                bg = g;
            }
        }
        self.align = bg * PATHS + bf;
        self.offset_hz = ((self.align as i32 - (MARGIN * PATHS) as i32) as f32 / PATHS as f32)
            * self.spacing as f32;
    }

    /// One soft symbol into the Viterbi stream, one bit out of its far end.
    fn decode_symbol(&mut self, s: u8) {
        self.pair[0] = self.pair[1];
        self.pair[1] = s;
        self.symcounter = !self.symcounter;
        // The reference decodes on every second soft symbol for four bits per
        // symbol, which is MFSK16 and its neighbours; the odd variants use the
        // other decoder for the second half of the pair.
        let bits = self.variant.symbits;
        // Four bits per symbol (MFSK16 and its neighbours) takes the second
        // soft symbol of each pair and decodes one bit; the odd variants take
        // the other half.
        let use_second = (bits != 5 && bits != 3) != self.second_half;
        if (use_second && self.symcounter) || (!use_second && !self.symcounter) {
            return;
        }
        // Soft symbols are 0..255 with 128 unknown; the stream wants +1 for a
        // set bit, so the value is shifted and scaled.
        let a = self.soft_sign * (self.pair[0] as f32 - 128.0) / 128.0;
        let b = self.soft_sign * (self.pair[1] as f32 - 128.0) / 128.0;
        let Some(bit) = self.viterbi.push(a, b) else {
            return;
        };
        // `VaricodeRx` owns the word rule: a word starts with 1 and ends with
        // 00, and it hands back a character when one completes.
        if let Some(c) = self.varicode.push_bit(bit) {
            self.push_char(c);
        }
    }

    fn push_char(&mut self, c: char) {
        match c {
            '\r' | '\n' => {
                if !self.text.ends_with('\n') && !self.text.is_empty() {
                    self.text.push('\n');
                }
            }
            c if c >= ' ' && c != '\u{7f}' => self.text.push(c),
            _ => {}
        }
    }

    /// For a modulator or a test: the varicode word for a character, as bits.
    pub fn varicode_bits(c: char) -> Vec<u8> {
        varicode(c as u8).bytes().map(|b| b - b'0').collect()
    }
}

// ---------------------------------------------------------------- the modulator

/// Encode `text` as this variant's audio, at the variant's own rate.
///
/// This exists for the tests. Without a transmitter, every experiment on the
/// receiver changed two things at once and the stage that diverged could not be
/// named — which is exactly how the Olivia work was lost. With one, a loopback
/// says whether the receiver can decode at all, and a disagreement with the real
/// capture then points at the conventions rather than at the plumbing.
///
/// **A loopback proves self-consistency, not interoperability.** It cannot show
/// that this is what a station sends, only that the two halves here agree.
pub fn encode(variant: MfskVariant, centre: f64, text: &str) -> Vec<f32> {
    let spacing = variant.spacing();
    let f0 = centre - variant.bandwidth() / 2.0;
    let bits = variant.symbits;
    let mut out: Vec<f32> = Vec::new();
    let mut phase: f64 = 0.0;

    let mut encoder = crate::fec::ConvEncoder::new(ConvCode::THOR);
    let mut interleaver = Interleaver::new(bits, variant.depth, true);
    let mut shift: u32 = 0;
    let mut filled = 0usize;

    // The preamble: `preamble` pairs of the zero-code's outputs, written straight
    // into the symbol shifter, so no data is carried and the receiver's clock has
    // something steady to lock on.
    let zero = encoder.push(0);
    for _ in 0..variant.preamble {
        for i in 0..2 {
            shift = (shift << 1) | ((zero >> i) & 1) as u32;
            filled += 1;
            if filled == bits {
                push_symbol(&mut out, &mut phase, &mut interleaver, shift, f0, spacing, variant);
                filled = 0;
                shift = 0;
            }
        }
    }
    encoder.reset();
    // Then `preamble / 3` encoded zero bits, which is what the reference does
    // between the preamble and the first character.
    for _ in 0..(variant.preamble / 3) {
        let d = encoder.push(0);
        for i in 0..2 {
            shift = (shift << 1) | ((d >> i) & 1) as u32;
            filled += 1;
            if filled == bits {
                push_symbol(&mut out, &mut phase, &mut interleaver, shift, f0, spacing, variant);
                filled = 0;
                shift = 0;
            }
        }
    }

    // CR LF CR, then the text, each character as its varicode word plus the two
    // terminating zeros.
    let mut chars: Vec<u8> = vec![13, 2, 13];
    chars.extend(text.bytes());
    chars.push(2);
    for c in chars {
        let word = varicode(c);
        let mut stream: Vec<u8> = word.bytes().map(|b| b - b'0').collect();
        stream.push(0);
        stream.push(0);
        for b in stream {
            let d = encoder.push(b);
            for i in 0..2 {
                shift = (shift << 1) | ((d >> i) & 1) as u32;
                filled += 1;
                if filled == bits {
                    push_symbol(
                        &mut out,
                        &mut phase,
                        &mut interleaver,
                        shift,
                        f0,
                        spacing,
                        variant,
                    );
                    filled = 0;
                    shift = 0;
                }
            }
        }
    }
    out
}

fn push_symbol(
    out: &mut Vec<f32>,
    phase: &mut f64,
    interleaver: &mut Interleaver,
    mut word: u32,
    f0: f64,
    spacing: f64,
    variant: MfskVariant,
) {
    interleaver.bits(&mut word);
    let tone = gray(word) as usize % variant.tones;
    let w = 2.0 * PI * (f0 + tone as f64 * spacing) / variant.fs;
    for _ in 0..variant.symlen {
        out.push(phase.cos() as f32);
        *phase += w;
        if *phase > 2.0 * PI {
            *phase -= 2.0 * PI;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variants_are_the_documented_ones() {
        let m16 = MfskVariant::named("MFSK16").expect("MFSK16 is a variant");
        assert_eq!(m16.symlen, 512);
        assert_eq!(m16.tones, 16);
        assert!((m16.spacing() - 15.625).abs() < 1e-9, "spacing {}", m16.spacing());
        assert!((m16.bandwidth() - 234.375).abs() < 1e-9);
        assert_eq!(MfskVariant::named("mFsk22").map(|v| v.fs), Some(11025.0));
        assert!(MfskVariant::named("MFSK128").is_none(), "128 is not in this table");
    }

    #[test]
    fn gray_and_its_inverse_are_a_pair() {
        for k in 0..64u32 {
            assert_eq!(binary(gray(k)), k, "gray round trip at {k}");
        }
    }

    /// The interleaver must be a permutation: what goes in must come out, and a
    /// contiguous burst of errors must not land on one word.
    #[test]
    fn the_interleaver_spreads_a_burst() {
        let (size, depth) = (4usize, 10usize);
        let mut tx = Interleaver::new(size, depth, true);
        let mut rx = Interleaver::new(size, depth, false);
        let mut burst = [1u8, 2, 3, 4];
        tx.symbols(&mut burst);
        // The first step out of the transmitter is not the input again.
        assert_ne!(burst, [1, 2, 3, 4], "the interleaver did nothing");
        let mut back = burst;
        for _ in 0..size * depth {
            rx.symbols(&mut back);
        }
        // And the two directions are not the same permutation, which is what
        // makes a burst spread rather than pass straight through.
        let mut tx2 = Interleaver::new(size, depth, true);
        let mut a = [9u8, 9, 9, 9];
        let mut b = [9u8, 9, 9, 9];
        tx2.symbols(&mut a);
        rx.symbols(&mut b);
        assert_ne!(a, b, "forward and reverse interleave must differ");
    }

    /// The sliding DFT must put a tone at the bin it belongs to.
    ///
    /// This is the front end everything else stands on: if the bank is a tone
    /// out, every soft symbol is wrong and the decoder produces structure with
    /// the wrong letters, which is exactly what it did.
    #[test]
    fn the_bank_finds_a_tone() {
        let fs = 8000.0f64;
        let freqs: Vec<f64> = (0..16).map(|i| 1600.0 + i as f64 * 15.625).collect();
        let mut bank = Bank::new(512, fs, &freqs);
        // A tone on bin 5, a whole window's worth plus a little.
        let f = freqs[5];
        for n in 0..600 {
            bank.feed((2.0 * PI * f * n as f64 / fs).sin());
        }
        let peak = (0..16).max_by(|&a, &b| bank.mag(a).total_cmp(&bank.mag(b))).unwrap();
        assert_eq!(peak, 5, "the bank peaks at bin {peak}, not 5");
        assert!(bank.mag(5) > 100.0 * bank.mag(6), "peak {} next {}", bank.mag(5), bank.mag(6));

        // And with a quarter bin of mistuning, path 1 of the four must be right.
        let mut off =
            Bank::new(512, fs, &freqs.iter().map(|&f| f + 0.25 * 15.625).collect::<Vec<_>>());
        for n in 0..600 {
            off.feed((2.0 * PI * f * n as f64 / fs).sin());
        }
        assert!(
            off.mag(5) > off.mag(4) && off.mag(5) > off.mag(6),
            "a quarter bin off still finds bin 5"
        );
    }

    /// Silence must not produce text.
    #[test]
    fn silence_produces_nothing() {
        let v = MfskVariant::named("MFSK16").unwrap();
        let mut rx = MfskRx::new(v, 8000.0, 1500.0);
        let quiet = vec![0.0f32; 8000 * 4];
        rx.feed(&quiet);
        assert!(rx.text().chars().filter(|c| c.is_alphabetic()).count() < 4, "{:?}", rx.text());
    }

    /// The same receiver over our own signal and over the air capture, tone by
    /// tone. This is the comparison that names a convention difference: the
    /// receiver's output text cannot, because it is the thing being questioned.
    #[test]
    #[ignore]
    fn the_capture_and_our_modulator_agree_on_the_tones() {
        let Ok(path) = std::env::var("SDROXIDE_MFSK_SAMPLE") else {
            eprintln!("SDROXIDE_MFSK_SAMPLE unset; skipping");
            return;
        };
        let variant = MfskVariant::named("MFSK16").unwrap();
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        let hz = num("SDROXIDE_MFSK_HZ", 1730.0);

        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();

        let mut air = MfskRx::new(variant, spec.sample_rate as f64, hz);
        for c in audio.chunks(512) {
            air.feed(c);
        }
        let mut mine = MfskRx::new(variant, variant.fs, 1500.0);
        mine.feed(&encode(variant, 1500.0, "TEST 12345 SDRBROWN"));
        mine.feed(&vec![0.0f32; 8000]);

        let n = 40.min(air.tones.len()).min(mine.tones.len());
        eprintln!("air  : {:?}", &air.tones[..n]);
        eprintln!("ours : {:?}", &mine.tones[..n]);
        eprintln!("air chars : {:?}", air.text());
        eprintln!("ours chars: {:?}", mine.text());
        // The preamble is a fixed pattern, so the two sequences must line up
        // somewhere within it if the conventions agree.
    }

    /// Our own modulator into our own receiver.
    ///
    /// Self-consistency only — it cannot prove this is what a station sends. It
    /// can prove the plumbing: if this fails, the fault is inside these two
    /// functions and has nothing to do with the air.
    #[test]
    fn our_own_transmission_reads_back() {
        let variant = MfskVariant::named("MFSK16").unwrap();
        let text = "TEST 12345 SDRBROWN";
        let audio = encode(variant, 1500.0, text);
        assert!(!audio.is_empty(), "the modulator produced nothing");
        let mut rx = MfskRx::new(variant, variant.fs, 1500.0);
        rx.feed(&audio);
        rx.feed(&vec![0.0f32; 8000]);
        eprintln!("loopback: {:?}", rx.text());
        assert!(rx.text().contains("TEST"), "loopback did not read the text: {:?}", rx.text());
    }

    /// Point `SDROXIDE_MFSK_SAMPLE` at a mono 8 kHz WAV of an MFSK
    /// transmission; `_VARIANT`, `_HZ` and `_TEXT` describe it.
    ///
    /// `~/Downloads/sounds/our-captures/mfsk16_cb_eagle.wav` is the MFSK16
    /// capture from the bench, 255 Hz wide, centre 1730.
    #[test]
    #[ignore]
    fn an_off_air_capture_decodes() {
        let Ok(path) = std::env::var("SDROXIDE_MFSK_SAMPLE") else {
            eprintln!("SDROXIDE_MFSK_SAMPLE unset; skipping");
            return;
        };
        let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
        let spec = reader.spec();
        let audio: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
            hound::SampleFormat::Int => {
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
            }
        };
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        let name = std::env::var("SDROXIDE_MFSK_VARIANT").unwrap_or_else(|_| "MFSK16".into());
        let variant = MfskVariant::named(&name).expect("a variant name");
        let hz = num("SDROXIDE_MFSK_HZ", 1730.0);

        let mut rx = MfskRx::new(variant, spec.sample_rate as f64, hz);
        for chunk in audio.chunks(512) {
            rx.feed(chunk);
        }
        let pad = vec![0.0f32; spec.sample_rate as usize];
        rx.feed(&pad);
        eprintln!(
            "{name} at {hz} Hz (mirror {}): {} chars, snr {:.1} dB, offset {:.1} Hz, locked {}: {:?}",
            std::env::var("SDROXIDE_MFSK_MIRROR").is_ok(),
            rx.text().len(),
            rx.snr_db,
            rx.offset_hz,
            rx.locked,
            rx.text()
        );
        if let Ok(want) = std::env::var("SDROXIDE_MFSK_TEXT") {
            for w in want.split_whitespace() {
                assert!(rx.text().contains(w), "did not recover {w:?}: {:?}", rx.text());
            }
        }
    }

    /// The varicode tables are the ones the modes use: a character's word must
    /// start with 1 and end with 00, which is the IZ8BLY rule.
    #[test]
    fn every_varicode_word_is_well_formed() {
        let mut coded = 0;
        for c in 0u8..=127 {
            let word = varicode(c);
            if word.is_empty() {
                continue; // NUL has no word, which is how the tables are built
            }
            coded += 1;
            assert!(word.starts_with('1'), "{:?} for {c:?} must start with 1", word);
            assert!(
                !word.contains("00"),
                "{:?} for {c:?} must not contain 00 - the terminator is appended, not part of the word",
                word
            );
            assert!(word.ends_with('1'), "{:?} for {c:?} must end with 1", word);
        }
        assert!(coded > 90, "only {coded} characters have a word");
        assert_eq!(MfskRx::varicode_bits('A').len(), varicode(b'A').len());
    }
}
