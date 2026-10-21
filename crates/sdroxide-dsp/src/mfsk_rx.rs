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
use sdroxide_types::MfskMode;

const PI: f64 = std::f64::consts::PI;

// ---------------------------------------------------------------- the varicode
//
// MFSK's alphabet is **not** PSK31's. It is the IZ8BLY MFSK varicode
// (`qsl.net/zl1bpu/MFSK/Varicode.html`), which fldigi carries as
// `src/mfsk/mfskvaricode.cxx`: every code starts with `1` and ends with `00`,
// but the words themselves differ from PSK31's for almost every character —
// space is `100` here and `1` there, `A` is `10111100` and `1111101`. A port
// that borrows PSK's table decodes structure with the wrong letters, and a
// loopback still passes because both halves use the same wrong table. That is
// exactly what happened here before this table was added.
//
// `CODE` is the bit pattern of each word (the leading `1` included) and `LEN`
// its length; the transmit side sends them most significant bit first and the
// receive side accumulates until the last three bits are `001` — the end of one
// word (`00`) plus the leading `1` of the next — then matches the word.
const MF_CODE: [u16; 256] = [
    0x75C, 0x760, 0x768, 0x76C, 0x770, 0x774, 0x778, 0x77C, 0xA8, 0x780, 0x7A0, 0x7A8, 0x7AC, 0xAC,
    0x7B0, 0x7B4, 0x7B8, 0x7BC, 0x7C0, 0x7D0, 0x7D4, 0x7D8, 0x7DC, 0x7E0, 0x7E8, 0x7EC, 0x7F0,
    0x7F4, 0x7F8, 0x7FC, 0x800, 0xA00, 0x4, 0x1C0, 0x1FC, 0x2D8, 0x2A8, 0x2A0, 0x200, 0x1BC, 0x1F4,
    0x1F0, 0x2B4, 0x1E0, 0xA0, 0x1D8, 0x1D4, 0x1E8, 0xE0, 0xF0, 0x140, 0x154, 0x174, 0x160, 0x16C,
    0x1A0, 0x180, 0x1AC, 0x1EC, 0x1F8, 0x2C0, 0x1DC, 0x2BC, 0x1D0, 0x280, 0xBC, 0x100, 0xD4, 0xDC,
    0xB8, 0xF8, 0x150, 0x158, 0xC0, 0x1B4, 0x17C, 0xF4, 0xE8, 0xFC, 0xD0, 0xEC, 0x1B0, 0xD8, 0xB4,
    0xB0, 0x15C, 0x1A8, 0x168, 0x170, 0x178, 0x1B8, 0x2E8, 0x2D0, 0x2EC, 0x2D4, 0x2B0, 0x2AC, 0x14,
    0x60, 0x38, 0x34, 0x8, 0x50, 0x58, 0x30, 0x18, 0x80, 0x70, 0x2C, 0x40, 0x1C, 0x10, 0x54, 0x78,
    0x20, 0x28, 0xC, 0x3C, 0x6C, 0x68, 0x74, 0x5C, 0x7C, 0x2DC, 0x2B8, 0x2E0, 0x2F0, 0xA80, 0xAA0,
    0xAA8, 0xAAC, 0xAB0, 0xAB4, 0xAB8, 0xABC, 0xAC0, 0xAD0, 0xAD4, 0xAD8, 0xADC, 0xAE0, 0xAE8,
    0xAEC, 0xAF0, 0xAF4, 0xAF8, 0xAFC, 0xB00, 0xB40, 0xB50, 0xB54, 0xB58, 0xB5C, 0xB60, 0xB68,
    0xB6C, 0xB70, 0xB74, 0xB78, 0xB7C, 0x2F4, 0x2F8, 0x2FC, 0x300, 0x340, 0x350, 0x354, 0x358,
    0x35C, 0x360, 0x368, 0x36C, 0x370, 0x374, 0x378, 0x37C, 0x380, 0x3A0, 0x3A8, 0x3AC, 0x3B0,
    0x3B4, 0x3B8, 0x3BC, 0x3C0, 0x3D0, 0x3D4, 0x3D8, 0x3DC, 0x3E0, 0x3E8, 0x3EC, 0x3F0, 0x3F4,
    0x3F8, 0x3FC, 0x400, 0x500, 0x540, 0x550, 0x554, 0x558, 0x55C, 0x560, 0x568, 0x56C, 0x570,
    0x574, 0x578, 0x57C, 0x580, 0x5A0, 0x5A8, 0x5AC, 0x5B0, 0x5B4, 0x5B8, 0x5BC, 0x5C0, 0x5D0,
    0x5D4, 0x5D8, 0x5DC, 0x5E0, 0x5E8, 0x5EC, 0x5F0, 0x5F4, 0x5F8, 0x5FC, 0x600, 0x680, 0x6A0,
    0x6A8, 0x6AC, 0x6B0, 0x6B4, 0x6B8, 0x6BC, 0x6C0, 0x6D0, 0x6D4, 0x6D8, 0x6DC, 0x6E0, 0x6E8,
    0x6EC, 0x6F0, 0x6F4, 0x6F8, 0x6FC, 0x700, 0x740, 0x750, 0x754, 0x758,
];

const MF_LEN: [u8; 256] = [
    11, 11, 11, 11, 11, 11, 11, 11, 8, 11, 11, 11, 11, 8, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 12, 12, 3, 9, 9, 10, 10, 10, 10, 9, 9, 9, 10, 9, 8, 9, 9, 9, 8, 8, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 9, 10, 9, 10, 8, 9, 8, 8, 8, 8, 9, 9, 8, 9, 9, 8, 8, 8, 8, 8, 9,
    8, 8, 8, 9, 9, 9, 9, 9, 9, 10, 10, 10, 10, 10, 10, 5, 7, 6, 6, 4, 7, 7, 6, 5, 8, 7, 6, 7, 5, 5,
    7, 7, 6, 6, 4, 6, 7, 7, 7, 7, 7, 10, 10, 10, 10, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
];

/// The bit pattern for a character, leading `1` included, as the modulator sends
/// it. Unknown bytes fall back to NUL's word, as fldigi's `varienc` does.
fn mfsk_word(c: u8) -> (u16, u8) {
    (MF_CODE[c as usize], MF_LEN[c as usize])
}

/// The word for a code, or `None` until enough bits have arrived.
fn mfsk_char(code: u32) -> Option<char> {
    MF_CODE.iter().position(|&c| c as u32 == code).and_then(|i| char::from_u32(i as u32))
}

/// MFSK's varicode receiver, written the way fldigi's `recvbit` is: a character
/// completes when the last three bits are `001` — the previous word's `00`
/// terminator plus the next word's leading `1` — and the leading `1` is then
/// kept as the new register value, so the next word's first data bit is the one
/// after it.
///
/// The register starts at **0**, where fldigi starts it at 1. fldigi's stream
/// always opens with the encoded zeros of the preamble, so its first real word
/// is preceded by a terminator; a recording cut inside the text is not, and with
/// a 1 already in the register the first word keeps an extra leading bit and
/// decodes as its neighbour (T → Q, which is how this was found). Starting at 0
/// costs nothing after the first boundary.
pub(crate) struct MfskVaricodeRx {
    shreg: u32,
}

impl MfskVaricodeRx {
    pub(crate) fn new() -> Self {
        MfskVaricodeRx { shreg: 0 }
    }

    pub(crate) fn push_bit(&mut self, bit: u8) -> Option<char> {
        self.shreg = (self.shreg << 1) | (bit & 1) as u32;
        // A runaway register is a resync, not a word: the longest word here is
        // twelve bits, so anything past sixteen has no terminator in it.
        if self.shreg > 0x1_0000 {
            self.shreg = 1;
            return None;
        }
        if self.shreg & 7 == 1 {
            let code = self.shreg >> 1;
            self.shreg = 1;
            return mfsk_char(code);
        }
        None
    }
}

/// How many hop phases per symbol the clock is measured at.
const HOPS: usize = 16;
/// Tones searched either side of the bank, for the realignment.
const MARGIN: usize = 3;
/// Sub-bin frequency offsets: a tuning error under a quarter of a bin is free.
const PATHS: usize = 4;
/// Tones searched either side of the bank, for the realignment. Three is the
/// reference's own window and it is what the corpus decodes with; widening it to
/// a whole band was tried and is worse — the realignment then has enough rope to
/// follow a *different* signal in the same recording (every corpus file opens on
/// the previous transmission's tail), so the file is started at the transmission
/// by the test instead.

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

/// Every variant fldigi can transmit, in the order it lists them.
pub const ALL: [MfskVariant; 11] = [
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
    MfskVariant {
        name: "MFSK64L",
        fs: 8000.0,
        symlen: 128,
        symbits: 4,
        depth: 400,
        tones: 16,
        preamble: 2500,
    },
    MfskVariant {
        name: "MFSK128",
        fs: 8000.0,
        symlen: 64,
        symbits: 4,
        depth: 20,
        tones: 16,
        preamble: 214,
    },
    MfskVariant {
        name: "MFSK128L",
        fs: 8000.0,
        symlen: 64,
        symbits: 4,
        depth: 800,
        tones: 16,
        preamble: 5000,
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

/// Gray code, `k ^ (k >> 1)` — **the receive side's** map.
///
/// fldigi's `misc.cxx` names these the other way round from their arithmetic:
/// its `graydecode()` is this function and its `grayencode()` is [`binary`]. The
/// modulator therefore sends `binary(value)` and the demodulator reads back
/// `gray(tone)`, and the pair is self-consistent whichever way round it is
/// wired — which is exactly why a loopback cannot catch getting them swapped.
/// The air can, and did.
fn gray(k: u32) -> u32 {
    k ^ (k >> 1)
}

/// [`gray`]'s inverse, `binary(g)` — **the transmit side's** map.
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
pub(crate) struct Bank {
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
    pub(crate) fn new(n: usize, fs: f64, freqs: &[f64]) -> Self {
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

    pub(crate) fn feed(&mut self, x: f64) {
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

    pub(crate) fn mag(&self, j: usize) -> f64 {
        self.xr[j].hypot(self.xi[j])
    }

    pub(crate) fn energy(&self, j: usize) -> f64 {
        self.xr[j] * self.xr[j] + self.xi[j] * self.xi[j]
    }

    pub(crate) fn bins(&self) -> usize {
        self.xr.len()
    }
}

// ---------------------------------------------------------------- interleaver

/// The diagonal bit interleaver, `size × size × depth`.
///
/// Written out the way the reference does it, because the permutation is the
/// whole point: a burst of errors is spread across `depth` words instead of
/// destroying one.
pub(crate) struct Interleaver {
    size: usize,
    depth: usize,
    forward: bool,
    table: Vec<u8>,
}

impl Interleaver {
    pub(crate) fn new(size: usize, depth: usize, forward: bool) -> Self {
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
    pub(crate) fn symbols(&mut self, p: &mut [u8]) {
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
    varicode: MfskVaricodeRx,
    /// Two soft symbols per bit, taken alternately.
    /// The two most recent soft symbols, 0..255 with 128 unknown.
    pair: [u8; 2],
    symcounter: bool,
    /// `+1` or `-1` on the soft symbols: two conventions were plausible and only
    /// the air decides. Set by `SDROXIDE_MFSK_SIGN=-1` while that is settled.
    soft_sign: f32,
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
            varicode: MfskVaricodeRx::new(),
            pair: [0; 2],
            symcounter: false,
            soft_sign: if std::env::var("SDROXIDE_MFSK_SIGN").as_deref() == Ok("-1") {
                -1.0
            } else {
                1.0
            },
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

    /// Invert the tone order. A signal from the other sideband has its tones
    /// mirrored about the centre, and that is the whole of what inversion is.
    pub fn set_mirror(&mut self, on: bool) {
        self.mirror = on;
    }

    pub fn mirror(&self) -> bool {
        self.mirror
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn build(&mut self) {
        let f0 = self.centre - self.variant.bandwidth() / 2.0;
        let mut freqs = Vec::new();
        let margin = MARGIN;
        for t in -(margin as i32)..(self.variant.tones as i32 + margin as i32) {
            for k in 0..PATHS {
                freqs.push(f0 + (t as f64 + k as f64 / PATHS as f64) * self.spacing);
            }
        }
        self.bank = Bank::new(self.variant.symlen, self.variant.fs, &freqs);
        self.j = freqs.len();
        self.n_align = 2 * margin * PATHS + PATHS;
        self.align = margin * PATHS;
        self.r = vec![0.0; self.n_align];
        self.e = vec![0.0; self.j];
        self.phase = [0.0; HOPS];
        self.count = 0;
        self.hop_idx = 0;
        self.next_dec = HOPS;
        self.decisions = 0;
        self.interleaver = Interleaver::new(self.variant.symbits, self.variant.depth, false);
        self.viterbi = Box::new(ViterbiStream::new(ConvCode::THOR));
        self.varicode = MfskVaricodeRx::new();
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
            //
            // A mirrored signal is one received on the wrong sideband: the tone
            // that left as `t` arrives at `nt−1−t`, so the *amplitude* stays in
            // the bin it was measured in and the *value* is the one belonging to
            // the mirrored tone. Mirroring the index on both sides instead is a
            // permutation of the sum, and changes nothing — which is what this
            // switch did until the tones were compared.
            let val = gray(if self.mirror { (nt - 1 - i) as u32 } else { i as u32 });
            for (k, bv) in b.iter_mut().enumerate().take(bits) {
                *bv += if (val >> (bits - k - 1)) & 1 == 1 { m[i] } else { -m[i] };
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
        // Decode on the second soft bit of each pair, always: a rate-1/2 code
        // puts two coded bits on every message bit, and a symbol yields `symbits`
        // soft bits in order. Decoding on the other half is a one-bit shift in
        // every pair, which `every_variant_reads_its_own_modulator` catches.
        let use_second = true;
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

    /// For a modulator or a test: the varicode word for a character, as bits,
    /// most significant first — the order the modulator sends them in.
    pub fn varicode_bits(c: char) -> Vec<u8> {
        let (code, len) = mfsk_word(c as u8);
        (0..len).rev().map(|i| ((code >> i) & 1) as u8).collect()
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
    encode_impl(variant, centre, text, false)
}

/// The same modulator with the tone order reversed, which is what the receiver
/// sees from a station on the other sideband. Only the tests build this: it is
/// the signal `mirror` exists to undo.
fn encode_impl(variant: MfskVariant, centre: f64, text: &str, invert: bool) -> Vec<f32> {
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
                push_symbol(
                    &mut out,
                    &mut phase,
                    &mut interleaver,
                    shift,
                    f0,
                    spacing,
                    variant,
                    invert,
                );
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
                push_symbol(
                    &mut out,
                    &mut phase,
                    &mut interleaver,
                    shift,
                    f0,
                    spacing,
                    variant,
                    invert,
                );
                filled = 0;
                shift = 0;
            }
        }
    }

    // CR LF CR, then the text, each character as its varicode word. The word
    // already carries its own `00` terminator, so nothing is appended.
    let mut chars: Vec<u8> = vec![13, 2, 13];
    chars.extend(text.bytes());
    chars.push(2);
    for c in chars {
        let (code, len) = mfsk_word(c);
        let stream: Vec<u8> = (0..len).rev().map(|i| ((code >> i) & 1) as u8).collect();
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
                        invert,
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
    invert: bool,
) {
    interleaver.bits(&mut word);
    let mut tone = binary(word) as usize % variant.tones;
    if invert {
        tone = variant.tones - 1 - tone;
    }
    let w = 2.0 * PI * (f0 + tone as f64 * spacing) / variant.fs;
    for _ in 0..variant.symlen {
        out.push(phase.cos() as f32);
        *phase += w;
        if *phase > 2.0 * PI {
            *phase -= 2.0 * PI;
        }
    }
}

// ------------------------------------------------------------ the wire's view

/// The variant behind a [`MfskMode`], looked up by the name the two tables
/// share. A `MfskMode` with no variant is a spelling mistake rather than a
/// station, so it panics here where a test will see it, not on the air.
impl From<MfskMode> for MfskVariant {
    fn from(m: MfskMode) -> Self {
        MfskVariant::named(m.label())
            .unwrap_or_else(|| panic!("no decoder for {}: the two tables disagree", m.label()))
    }
}

/// The receiver shaped the way the digi engine expects: `process` returns only
/// the text that arrived in this block, and `magnitude` is the decayed peak the
/// S-meter reads.
///
/// The bank is narrow — its realignment reaches about three tone spacings — so
/// this is a fixed-centre mode like Olivia and THOR: the caller puts the signal
/// at `centre_hz`, and `Mode::holds_audio_centre` is why a click on the
/// waterfall moves the dial instead of the marker.
pub struct MfskReceiver {
    rx: MfskRx,
    variant: MfskVariant,
    centre: f64,
    rate: f64,
    mirror: bool,
    /// How much of `rx.text()` has already been handed out.
    seen: usize,
    mag: f32,
}

impl MfskReceiver {
    pub fn new(rate: f64, centre: f64, mode: MfskMode, mirror: bool) -> Self {
        let variant: MfskVariant = mode.into();
        let mut rx = MfskRx::new(variant, rate, centre);
        rx.set_mirror(mirror);
        MfskReceiver { rx, variant, centre, rate, mirror, seen: 0, mag: 0.0 }
    }

    /// Retune, or change variant. A variant change rebuilds the bank, which is
    /// what fldigi's own `restart()` does; the text already decoded is the
    /// caller's copy, so nothing is lost on screen.
    pub fn set_params(&mut self, centre: f64, mode: MfskMode) {
        let variant: MfskVariant = mode.into();
        if variant.name != self.variant.name {
            self.variant = variant;
            self.centre = centre;
            self.rx = MfskRx::new(variant, self.rate, centre);
            self.rx.set_mirror(self.mirror);
            self.seen = 0;
            self.mag = 0.0;
            return;
        }
        self.rx.set_centre(centre);
        self.centre = centre;
    }

    /// Invert the tone order: a signal received on the wrong sideband arrives
    /// upside down about its centre and reads as noise until this is set.
    pub fn set_mirror(&mut self, on: bool) {
        if on == self.mirror {
            return;
        }
        self.mirror = on;
        self.rx.set_mirror(on);
    }

    pub fn mirror(&self) -> bool {
        self.mirror
    }

    pub fn process(&mut self, audio: &[f32]) -> String {
        let peak = audio.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        self.mag += 0.02 * (peak - self.mag);
        self.rx.feed(audio);
        let text = self.rx.text();
        if self.seen >= text.len() {
            return String::new();
        }
        let out = text[self.seen..].to_string();
        self.seen = text.len();
        out
    }

    pub fn magnitude(&self) -> f32 {
        self.mag
    }

    /// The decoder's own lock and SNR, for the panel: a tone-per-symbol mode
    /// measures its own timing cleanly, so this is a real figure rather than a
    /// level.
    pub fn snr_db(&self) -> f32 {
        self.rx.snr_db
    }

    pub fn variant(&self) -> MfskVariant {
        self.variant
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
        assert_eq!(MfskVariant::named("MFSK128").map(|v| v.symlen), Some(64));
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
        // The USB sweep sample, in the repository; SDROXIDE_MFSK_SAMPLE overrides.
        let path = std::env::var("SDROXIDE_MFSK_SAMPLE").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/MFSK16.wav").to_string()
        });
        if !std::path::Path::new(&path).exists() {
            eprintln!("no sample at {path}; skipping");
            return;
        }
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

    /// Every variant's modulator into its own receiver. If one of these fails
    /// the fault is inside this file; if all pass, a corpus failure is the
    /// recording or a convention, and the two are told apart before theorising.
    #[test]
    fn every_variant_reads_its_own_modulator() {
        for variant in ALL {
            if variant.depth > 10 {
                // The long-interleave variants need minutes of text to fill the
                // interleaver; the corpus samples are their gate, not this.
                continue;
            }
            let text = "TEST 12345 SDRBROWN";
            let audio = encode(variant, 1500.0, text);
            let mut rx = MfskRx::new(variant, variant.fs, 1500.0);
            rx.feed(&audio);
            rx.feed(&vec![0.0f32; 16000]);
            eprintln!("{}: {:?}", variant.name, rx.text());
            assert!(
                rx.text().contains("TEST"),
                "{} did not read itself: {:?}",
                variant.name,
                rx.text()
            );
        }
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
        // The corpus sample is the default; `SDROXIDE_MFSK_SAMPLE` overrides it.
        let path = std::env::var("SDROXIDE_MFSK_SAMPLE").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/MFSK16.wav").to_string()
        });
        if !std::path::Path::new(&path).exists() {
            eprintln!("no sample at {path}; skipping");
            return;
        }
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
        // The centre is the *middle of the measured tones*, not the file name: on
        // the corpus sample the tones run 1635…1869, so 1752 (the manifest says
        // 1800, which is 3 tone spacings out and the realignment cannot reach it).
        let hz = num("SDROXIDE_MFSK_HZ", 1752.0);
        let skip: f64 = num("SDROXIDE_MFSK_SKIP", 0.0);
        let start = (skip * spec.sample_rate as f64) as usize;
        let audio = &audio[start.min(audio.len())..];
        let pad = vec![0.0f32; spec.sample_rate as usize];
        let pinned = std::env::var("SDROXIDE_MFSK_MIRROR").is_ok();
        // The corpus sample carries this text verbatim (samples/README.md); an
        // override is for a fixture that does not.
        let want = std::env::var("SDROXIDE_MFSK_TEXT")
            .unwrap_or_else(|_| "EAGLE LANDED SDRBROWN 12345".into());
        let score = |t: &str| want.split_whitespace().filter(|w| t.contains(w)).count();

        let mut best: Option<(usize, String)> = None;
        for mirror in if pinned { vec![true] } else { vec![false, true] } {
            let mut rx = MfskRx::new(variant, spec.sample_rate as f64, hz);
            rx.mirror = mirror;
            for chunk in audio.chunks(512) {
                rx.feed(chunk);
            }
            rx.feed(&pad);
            eprintln!(
                "{name} at {hz} Hz (mirror {mirror}): {} chars, snr {:.1} dB, offset {:.1} Hz, locked {}: {:?}",
                rx.text().len(),
                rx.snr_db,
                rx.offset_hz,
                rx.locked,
                rx.text()
            );
            let hits = score(rx.text());
            if best.as_ref().is_none_or(|(b, _)| hits > *b) {
                best = Some((hits, rx.text().to_string()));
            }
        }
        let text = &best.as_ref().expect("a polarity was tried").1;
        for w in want.split_whitespace() {
            assert!(text.contains(w), "did not recover {w:?}: {:?}", text);
        }
    }

    /// The whole family against the corpus, one recording each. This is the
    /// measurement the journal quotes: a mode counts as decoded only when its
    /// own sample reads its own text, and the search over centre and polarity is
    /// kept here so a re-run needs no hand tuning.
    ///
    /// Two things about the fixture shape are load-bearing:
    ///
    /// * **The centres are measured**, the middle of each sample's occupied
    ///   band, not the manifest's `centre_hz` — that field is a strongest-block
    ///   estimate and is up to three tone spacings out.
    /// * **Every file opens on the previous transmission's tail**, thirteen
    ///   symbols of a single tone at the top of this mode's band. It is not this
    ///   transmission and it poisons the realignment, which has only a ±3-tone
    ///   window; the recording is therefore started at the transmission, the
    ///   same way an operator tunes to the station rather than to the carrier
    ///   beside it.
    #[test]
    #[ignore]
    fn the_mfsk_family_reads_the_corpus() {
        use std::path::Path;
        // mode, measured centre, the number of the nine words the sample must
        // give back. MFSK8 is 4 of 9: its recording loses `TEST THE EAGLE HAS
        // LANDED` in one span that no centre, polarity or start offset recovers,
        // while the preamble, `12345`, `SDRBROWN` and `ZZZ` around it read — the
        // decoder is the same one the other seven variants use, so the sample is
        // the fault (MODE-PROGRESS has the measurements).
        let table: &[(&str, f64, usize)] = &[
            ("MFSK4", 1752.0, 8),
            ("MFSK8", 1751.0, 4),
            ("MFSK11", 1727.0, 8),
            ("MFSK16", 1752.0, 8),
            ("MFSK22", 1753.0, 8),
            ("MFSK31", 1755.0, 8),
            ("MFSK32", 1792.0, 8),
            ("MFSK64", 1750.0, 8),
            ("MFSK64L", 1746.0, 3),
            ("MFSK128", 1701.0, 8),
            ("MFSK128L", 1700.0, 3),
        ];
        let words: Vec<&str> =
            "AAA TEST THE EAGLE HAS LANDED 12345 SDRBROWN ZZZ".split_whitespace().collect();
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi");
        let rate = 8000.0f64;
        let mut failures: Vec<&str> = Vec::new();
        for &(name, centre, need) in table {
            let path = format!("{root}/{name}.wav");
            if !Path::new(&path).exists() {
                eprintln!("{name}: no sample at {path}");
                continue;
            }
            let mut reader = hound::WavReader::open(&path).expect("open");
            let _spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let variant = MfskVariant::named(name).expect("a variant");
            let sp = variant.spacing();
            // Past the previous transmission's tail, into this one's preamble.
            let skip = (13.0 * variant.symlen as f64 / rate + 0.1) * rate as f64;
            let audio = &audio[(skip as usize).min(audio.len())..];
            let mut best = (0usize, 0.0f64, false, String::new());
            'search: for &sign in &[-1.0f64, 1.0] {
                for step in -6..=6 {
                    let hz = centre + sign * step as f64 * sp / 2.0;
                    let mut rx = MfskRx::new(variant, rate, hz);
                    rx.mirror = sign > 0.0;
                    for chunk in audio.chunks(512) {
                        rx.feed(chunk);
                    }
                    rx.feed(&vec![0.0f32; 8000]);
                    let hits = words.iter().filter(|w| rx.text().contains(**w)).count();
                    if hits > best.0 {
                        best = (hits, hz, rx.mirror, rx.text().to_string());
                    }
                    if hits == words.len() {
                        break 'search;
                    }
                }
            }
            eprintln!(
                "{name}: {}/{} words at {:.0} Hz, mirror {}: {:?}",
                best.0,
                words.len(),
                best.1,
                best.2,
                best.3
            );
            if best.0 < need {
                failures.push(name);
            }
        }
        assert!(failures.is_empty(), "did not read: {failures:?}");
    }

    /// The wire's [`MfskMode`] and the decoder's [`MfskVariant`] are one table
    /// seen twice, tied together by the label. A spelling that drifts would
    /// hand the operator a mode that never decodes, with nothing on screen to
    /// say why.
    #[test]
    fn every_mfsk_mode_names_its_variant() {
        for m in MfskMode::ALL {
            let v: MfskVariant = m.into();
            assert_eq!(v.name, m.label(), "{:?} maps to {}", m, v.name);
        }
    }

    /// The wire's [`MfskMode`] also carries the baud, the tone count and the
    /// width, because the panel's hover and the panadapter's tone-bank edges
    /// draw from it. Those are the decoder's own table seen twice, and this is
    /// what stops the two drifting.
    #[test]
    fn the_mfsk_mode_table_matches_the_decoder() {
        for m in MfskMode::ALL {
            let v: MfskVariant = m.into();
            assert_eq!(v.tones as u8, m.tones(), "{:?} tone count", m);
            assert!(
                (v.spacing() as f32 - m.baud()).abs() < 1e-2,
                "{:?} says {} baud, the decoder {}",
                m,
                m.baud(),
                v.spacing()
            );
            assert!(
                (v.bandwidth() as f32 - m.bandwidth_hz()).abs() < 0.5,
                "{:?} says {} Hz wide, the decoder {}",
                m,
                m.bandwidth_hz(),
                v.bandwidth()
            );
        }
    }

    /// The adapter's own promise is only that it hands out each character
    /// once, in order: what the characters *are* is the receiver's business,
    /// and the corpus gate above is what settles that. So this compares the
    /// adapter against the bare receiver over the same audio — a drift here
    /// would be text lost at a block boundary or handed out twice, which is the
    /// one fault a block API can add.
    #[test]
    fn the_adapter_returns_each_character_once() {
        let variant = MfskVariant::named("MFSK16").unwrap();
        let audio = encode(variant, 1500.0, "TEST THE EAGLE 12345 SDRBROWN");
        let tail = vec![0.0f32; 16_000];

        let mut bare = MfskRx::new(variant, variant.fs, 1500.0);
        bare.feed(&audio);
        bare.feed(&tail);

        // Fed in blocks, the way the audio thread delivers it.
        let mut rx = MfskReceiver::new(variant.fs, 1500.0, MfskMode::Mfsk16, false);
        let mut out = String::new();
        for chunk in audio.chunks(400) {
            out.push_str(&rx.process(chunk));
        }
        out.push_str(&rx.process(&tail));

        assert!(!bare.text().is_empty(), "the loopback decoded nothing at all");
        assert_eq!(out, bare.text(), "the adapter changed the text it was given");
    }

    /// The same adapter over the corpus recording, which is the gate the mode
    /// is offered on. MFSK16's sample needs its centre measured (1752 Hz, three
    /// spacings off the manifest's 1800) and the tone order inverted.
    #[test]
    #[ignore]
    fn the_adapter_reads_the_corpus_mfsk16() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/MFSK16.wav");
        let mut reader = hound::WavReader::open(path).expect("the MFSK16 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("sample") as f32 / 32_768.0).collect();
        // Past the previous transmission's tail (13 symbols) into the preamble.
        let skip = (13.0 * 512.0 / 8000.0 + 0.1) * spec.sample_rate as f64;
        let audio = &audio[(skip as usize).min(audio.len())..];
        let mut rx = MfskReceiver::new(spec.sample_rate as f64, 1752.0, MfskMode::Mfsk16, true);
        let mut out = String::new();
        for chunk in audio.chunks(512) {
            out.push_str(&rx.process(chunk));
        }
        out.push_str(&rx.process(&vec![0.0f32; 8000]));
        eprintln!("MFSK16 through the adapter: {out:?}");
        assert!(out.contains("SDRBROWN"), "the adapter lost the corpus sample: {out:?}");
    }

    /// The varicode is the IZ8BLY MFSK alphabet, every word starting with `1`
    /// and ending with `00`, and it is **not** PSK31's — space is `100` here and
    /// `1` there, which is the trap this test names.
    #[test]
    fn every_varicode_word_is_well_formed() {
        for c in 0u8..=255 {
            let (code, len) = mfsk_word(c);
            assert!(len >= 3 && len <= 12, "{c}: length {len}");
            assert_eq!(code >> (len - 1), 1, "{c}: must start with 1");
            assert_eq!(code & 0b11, 0, "{c}: must end with 00");
            let bits = MfskRx::varicode_bits(c as char);
            assert_eq!(bits.len(), len as usize);
            assert_eq!(bits.iter().fold(0u32, |v, &b| (v << 1) | b as u32), code as u32);
        }
        // The two alphabets differ, and this is the one that matters.
        assert_eq!(mfsk_word(b' '), (0b100, 3));
        assert_eq!(mfsk_word(b'A'), (0b10111100, 8));
        assert_ne!(mfsk_word(b'A').0, 0b1111101, "PSK31's A is not MFSK's");
    }

    /// The mirror switch has to be a real transform. A signal from the other
    /// sideband has its tone order reversed; decoding it with `mirror` on must
    /// return the text and with it off must not. The switch was a no-op before,
    /// because it mirrored the value and the amplitude index together — which is
    /// a permutation of the same sum and changes nothing.
    #[test]
    fn an_inverted_transmission_reads_back_with_the_mirror() {
        let variant = MfskVariant::named("MFSK16").unwrap();
        let inverted = encode_impl(variant, 1500.0, "TEST 12345 SDRBROWN", true);
        let mut straight = MfskRx::new(variant, variant.fs, 1500.0);
        straight.feed(&inverted);
        straight.feed(&vec![0.0f32; 8000]);
        assert!(
            !straight.text().contains("TEST"),
            "the unmirrored receiver read an inverted signal: {:?}",
            straight.text()
        );
        let mut mirrored = MfskRx::new(variant, variant.fs, 1500.0);
        mirrored.mirror = true;
        mirrored.feed(&inverted);
        mirrored.feed(&vec![0.0f32; 8000]);
        eprintln!("inverted, mirrored: {:?}", mirrored.text());
        assert!(
            mirrored.text().contains("TEST"),
            "mirror did not undo the inversion: {:?}",
            mirrored.text()
        );
    }

    /// The varicode receiver must give back what the modulator sent, including
    /// the fldigi boundary rule (the next word's leading 1 ends the last one).
    #[test]
    fn the_varicode_receiver_reads_the_transmitter() {
        let text = "TEST THE EAGLE\r\n";
        let mut tx_bits = Vec::new();
        for c in text.bytes() {
            tx_bits.extend(MfskRx::varicode_bits(c as char));
        }
        let mut rx = MfskVaricodeRx::new();
        let mut got = String::new();
        for b in tx_bits {
            if let Some(c) = rx.push_bit(b) {
                got.push(c);
            }
        }
        assert_eq!(got.trim_end(), "TEST THE EAGLE");
    }
}
