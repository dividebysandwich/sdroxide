//! THOR and DominoEX — the incremental-frequency-keying family, receive only.
//!
//! Written from the keyboard-mode family in OnAir (`core/src/hfdig/hfdig_kbd_ifk.cpp`,
//! GPL-3.0, after fldigi's `src/dominoex` and `src/thor`), which is credited in
//! the README's Acknowledgements. Nothing is copied by name; the two data
//! tables (the DominoEX nibbles and the MFSK varicode, which THOR borrows) are
//! protocol constants.
//!
//! IFK+ is the whole trick: the data is the **step** between successive tones,
//! `2 + symbol (mod 18)`, so a receiver with no absolute frequency reference
//! still reads it — a tuning error shifts every tone and cancels in the step.
//!
//! * **DominoEX** sends each character as one to three nibbles of its own
//!   alphabet; a nibble with bit 3 set continues the character. No FEC.
//! * **THOR** sends the IZ8BLY word bits through a K = 7 convolutional code
//!   (K = 15 in 25x4 and above), a 4 × 4 × depth interleaver, four soft bits per
//!   symbol, and the same varicode receiver the MFSK modes use.
//!
//! The front end is the reference's: a sliding DFT over the 18 tones plus nine
//! tones either side, at five sub-bin offsets for a one-bin tone spacing and
//! three for a wider one, with the symbol clock taken from the peak sharpness
//! at sixteen hop phases.

use crate::MonoResampler;
use crate::mfsk_rx::{Bank, Interleaver, MfskVaricodeRx};

const TONES: usize = 18;
const HOPS: usize = 16;
const EXTRA: usize = 9;
/// The peak ratio below which a symbol is treated as noise and punctured.
const LOCK_RATIO: f64 = 6.0;

/// The DominoEX alphabet: one to three nibbles per character; a nibble with bit
/// 3 set continues. `[0, 0, 0]` is unused.
#[rustfmt::skip]
/// The DominoEX alphabet: **fldigi's own table** (`src/dominoex/dominovar.cxx`,
/// `varicode[][3]`, the primary 256 rows), transcribed 2026-10-22. One to three
/// nibbles per character, the *first* with bit 3 clear and the ones after it with
/// bit 3 set — which is the opposite way round from how it reads at first, and the
/// reason `decodeDomino` completes a character when the *next* one starts.
///
/// The table this replaces came from the OnAir port and is not fldigi's: its rows
/// 33 onwards look alike for a few entries and then diverge, so every character
/// decoded to nothing while the tone decisions were right.
#[rustfmt::skip]
const DOMINO_CODE: [[u8; 3]; 256] = [
    [1, 15, 9], [1, 15, 10], [1, 15, 11], [1, 15, 12], [1, 15, 13], [1, 15, 14], [1, 15, 15], [2, 8, 8],
    [2, 12, 0], [2, 8, 9], [2, 8, 10], [2, 8, 11], [2, 8, 12], [2, 13, 0], [2, 8, 13], [2, 8, 14],
    [2, 8, 15], [2, 9, 8], [2, 9, 9], [2, 9, 10], [2, 9, 11], [2, 9, 12], [2, 9, 13], [2, 9, 14],
    [2, 9, 15], [2, 10, 8], [2, 10, 9], [2, 10, 10], [2, 10, 11], [2, 10, 12], [2, 10, 13], [2, 10, 14],
    [0, 0, 0], [7, 11, 0], [0, 8, 14], [0, 10, 11], [0, 9, 10], [0, 9, 9], [0, 8, 15], [7, 10, 0],
    [0, 8, 12], [0, 8, 11], [0, 9, 13], [0, 8, 8], [2, 11, 0], [7, 14, 0], [7, 13, 0], [0, 8, 9],
    [3, 15, 0], [4, 10, 0], [4, 15, 0], [5, 9, 0], [6, 8, 0], [5, 12, 0], [5, 14, 0], [6, 12, 0],
    [6, 11, 0], [6, 14, 0], [0, 8, 10], [0, 8, 13], [0, 10, 8], [7, 15, 0], [0, 9, 15], [7, 12, 0],
    [0, 9, 8], [3, 9, 0], [4, 14, 0], [3, 12, 0], [3, 14, 0], [3, 8, 0], [4, 12, 0], [5, 8, 0],
    [5, 10, 0], [3, 10, 0], [7, 8, 0], [6, 10, 0], [4, 11, 0], [4, 8, 0], [4, 13, 0], [3, 11, 0],
    [4, 9, 0], [6, 15, 0], [3, 13, 0], [2, 15, 0], [2, 14, 0], [5, 11, 0], [6, 13, 0], [5, 13, 0],
    [5, 15, 0], [6, 9, 0], [7, 9, 0], [0, 10, 14], [0, 10, 9], [0, 10, 15], [0, 10, 10], [0, 9, 12],
    [0, 9, 11], [4, 0, 0], [1, 11, 0], [0, 12, 0], [0, 11, 0], [1, 0, 0], [0, 15, 0], [1, 9, 0],
    [0, 10, 0], [5, 0, 0], [2, 10, 0], [1, 14, 0], [0, 9, 0], [0, 14, 0], [6, 0, 0], [3, 0, 0],
    [1, 8, 0], [2, 8, 0], [7, 0, 0], [0, 8, 0], [2, 0, 0], [0, 13, 0], [1, 13, 0], [1, 12, 0],
    [1, 15, 0], [1, 10, 0], [2, 9, 0], [0, 10, 12], [0, 9, 14], [0, 10, 13], [0, 11, 8], [2, 10, 15],
    [2, 11, 8], [2, 11, 9], [2, 11, 10], [2, 11, 11], [2, 11, 12], [2, 11, 13], [2, 11, 14], [2, 11, 15],
    [2, 12, 8], [2, 12, 9], [2, 12, 10], [2, 12, 11], [2, 12, 12], [2, 12, 13], [2, 12, 14], [2, 12, 15],
    [2, 13, 8], [2, 13, 9], [2, 13, 10], [2, 13, 11], [2, 13, 12], [2, 13, 13], [2, 13, 14], [2, 13, 15],
    [2, 14, 8], [2, 14, 9], [2, 14, 10], [2, 14, 11], [2, 14, 12], [2, 14, 13], [2, 14, 14], [2, 14, 15],
    [0, 11, 9], [0, 11, 10], [0, 11, 11], [0, 11, 12], [0, 11, 13], [0, 11, 14], [0, 11, 15], [0, 12, 8],
    [0, 12, 9], [0, 12, 10], [0, 12, 11], [0, 12, 12], [0, 12, 13], [0, 12, 14], [0, 12, 15], [0, 13, 8],
    [0, 13, 9], [0, 13, 10], [0, 13, 11], [0, 13, 12], [0, 13, 13], [0, 13, 14], [0, 13, 15], [0, 14, 8],
    [0, 14, 9], [0, 14, 10], [0, 14, 11], [0, 14, 12], [0, 14, 13], [0, 14, 14], [0, 14, 15], [0, 15, 8],
    [0, 15, 9], [0, 15, 10], [0, 15, 11], [0, 15, 12], [0, 15, 13], [0, 15, 14], [0, 15, 15], [1, 8, 8],
    [1, 8, 9], [1, 8, 10], [1, 8, 11], [1, 8, 12], [1, 8, 13], [1, 8, 14], [1, 8, 15], [1, 9, 8],
    [1, 9, 9], [1, 9, 10], [1, 9, 11], [1, 9, 12], [1, 9, 13], [1, 9, 14], [1, 9, 15], [1, 10, 8],
    [1, 10, 9], [1, 10, 10], [1, 10, 11], [1, 10, 12], [1, 10, 13], [1, 10, 14], [1, 10, 15], [1, 11, 8],
    [1, 11, 9], [1, 11, 10], [1, 11, 11], [1, 11, 12], [1, 11, 13], [1, 11, 14], [1, 11, 15], [1, 12, 8],
    [1, 12, 9], [1, 12, 10], [1, 12, 11], [1, 12, 12], [1, 12, 13], [1, 12, 14], [1, 12, 15], [1, 13, 8],
    [1, 13, 9], [1, 13, 10], [1, 13, 11], [1, 13, 12], [1, 13, 13], [1, 13, 14], [1, 13, 15], [1, 14, 8],
    [1, 14, 9], [1, 14, 10], [1, 14, 11], [1, 14, 12], [1, 14, 13], [1, 14, 14], [1, 14, 15], [1, 15, 8],
];

/// The symbol a tone step carries.
///
/// IFK+ sends `step = 2 + symbol (mod 18)` and the tones sit at *absolute*
/// positions, so a step that wraps around the bank arrives as a negative
/// difference: `-16` is symbol 0 and `+1` is symbol 17. Anything outside
/// `-16..=19` is not a step at all.
///
/// fldigi's `dominoex::decodesymbol` is this arithmetic (`c = round(fdiff) - 2`,
/// `if (c < 0) c += NUMTONES`). An earlier version here instead declared the raw
/// differences 0 and 1 "out of range", which threw away symbols 16 and 17 — one
/// symbol in nine, on every transmission.
fn step_symbol(raw: i32) -> Option<u8> {
    let mut c = raw - 2;
    if c < 0 {
        c += TONES as i32;
    }
    (0..TONES as i32).contains(&c).then_some(c as u8)
}

/// One IFK variant. `ds` is the tone spacing in FFT bins times the reference's
/// base, `depth` the interleaver depth (THOR only), `k15` the longer code.
#[derive(Clone, Copy, Debug)]
pub struct IfkVariant {
    pub name: &'static str,
    pub thor: bool,
    pub fs: f64,
    pub symlen: usize,
    pub ds: usize,
    pub depth: usize,
    pub k15: bool,
}

pub const THOR: [IfkVariant; 12] = [
    IfkVariant {
        name: "THOR4",
        thor: true,
        fs: 8000.0,
        symlen: 2048,
        ds: 2,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR5",
        thor: true,
        fs: 11025.0,
        symlen: 2048,
        ds: 2,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR8",
        thor: true,
        fs: 8000.0,
        symlen: 1024,
        ds: 2,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR11",
        thor: true,
        fs: 11025.0,
        symlen: 1024,
        ds: 1,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR16",
        thor: true,
        fs: 8000.0,
        symlen: 512,
        ds: 1,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR22",
        thor: true,
        fs: 11025.0,
        symlen: 512,
        ds: 1,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR25x4",
        thor: true,
        fs: 8000.0,
        symlen: 320,
        ds: 4,
        depth: 50,
        k15: true,
    },
    IfkVariant {
        name: "THOR50x1",
        thor: true,
        fs: 8000.0,
        symlen: 160,
        ds: 1,
        depth: 50,
        k15: true,
    },
    IfkVariant {
        name: "THOR50x2",
        thor: true,
        fs: 8000.0,
        symlen: 160,
        ds: 2,
        depth: 50,
        k15: true,
    },
    IfkVariant { name: "THOR100", thor: true, fs: 8000.0, symlen: 80, ds: 1, depth: 50, k15: true },
    // fldigi's micro submode: a 4000-sample symbol at 8 kHz and a short
    // interleave; the corpus's THORM is this one.
    IfkVariant { name: "THORM", thor: true, fs: 8000.0, symlen: 4000, ds: 1, depth: 4, k15: false },
    // The corpus lists THOR25 separately from THOR25x4; fldigi's own table has
    // only 25x4, so the two are the same signal to this receiver.
    IfkVariant { name: "THOR25", thor: true, fs: 8000.0, symlen: 320, ds: 4, depth: 50, k15: true },
];

pub const DOMINO: [IfkVariant; 9] = [
    IfkVariant {
        name: "DOMEX4",
        thor: false,
        fs: 8000.0,
        symlen: 2048,
        ds: 2,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX5",
        thor: false,
        fs: 11025.0,
        symlen: 2048,
        ds: 2,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX8",
        thor: false,
        fs: 8000.0,
        symlen: 1024,
        ds: 2,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX11",
        thor: false,
        fs: 11025.0,
        symlen: 1024,
        ds: 1,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX16",
        thor: false,
        fs: 8000.0,
        symlen: 512,
        ds: 1,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX22",
        thor: false,
        fs: 11025.0,
        symlen: 512,
        ds: 1,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEXM",
        thor: false,
        fs: 8000.0,
        symlen: 4000,
        ds: 1,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX44",
        thor: false,
        fs: 11025.0,
        symlen: 256,
        ds: 2,
        depth: 0,
        k15: false,
    },
    IfkVariant {
        name: "DOMEX88",
        thor: false,
        fs: 11025.0,
        symlen: 128,
        ds: 1,
        depth: 0,
        k15: false,
    },
];

impl IfkVariant {
    pub fn named(name: &str) -> Option<Self> {
        let n = name.to_ascii_uppercase();
        // The sweep names some entries `DOMX11` where fldigi's table says
        // `DOMEX11`; both are the same mode.
        let canonical = |s: &str| {
            let u = s.to_ascii_uppercase();
            match u.strip_prefix("DOMX") {
                Some(rest) => format!("DOMEX{rest}"),
                None => u,
            }
        };
        let want = canonical(&n);
        THOR.iter().chain(DOMINO.iter()).copied().find(|v| canonical(v.name) == want)
    }

    pub fn spacing(&self) -> f64 {
        self.fs * self.ds as f64 / self.symlen as f64
    }
}

/// A streaming soft-decision Viterbi for either constraint length. The K = 7
/// code is the one the MFSK modes use; THOR's fast submodes use K = 15, which
/// is 16 384 states and does not fit the compact `ViterbiStream`.
struct SoftViterbi {
    k: usize,
    n: usize,
    pa: u32,
    pb: u32,
    metric: Vec<f32>,
    /// Predecessor (0/1) per state per step, in a ring.
    hist: Vec<Vec<u8>>,
    ptr: usize,
    depth: usize,
    started: bool,
}

impl SoftViterbi {
    fn new(k: usize, pa: u32, pb: u32, depth: usize) -> Self {
        let n = 1 << (k - 1);
        SoftViterbi {
            k,
            n,
            pa,
            pb,
            metric: vec![f32::NEG_INFINITY; n],
            hist: (0..depth).map(|_| vec![0u8; n]).collect(),
            ptr: 0,
            depth,
            started: false,
        }
    }

    fn parity(x: u32) -> f32 {
        if x.count_ones() & 1 == 1 { 1.0 } else { -1.0 }
    }

    /// One pair of soft symbols (positive for a received 1); returns the bit the
    /// oldest step has settled on, once the window is full.
    fn push(&mut self, a: f32, b: f32) -> Option<u8> {
        if !self.started {
            self.metric.iter_mut().for_each(|m| *m = f32::NEG_INFINITY);
            self.metric[0] = 0.0;
            self.started = true;
        }
        let mask = (1u32 << self.k) - 1;
        let mut next = vec![f32::NEG_INFINITY; self.n];
        let cur = &mut self.hist[self.ptr];
        for ns in 0..self.n {
            let bit = (ns & 1) as u32;
            let p0 = ns >> 1;
            let p1 = p0 | (self.n / 2);
            let o0 = Self::parity((((p0 as u32) << 1 | bit) & mask) & self.pa) * a
                + Self::parity((((p0 as u32) << 1 | bit) & mask) & self.pb) * b;
            let o1 = Self::parity((((p1 as u32) << 1 | bit) & mask) & self.pa) * a
                + Self::parity((((p1 as u32) << 1 | bit) & mask) & self.pb) * b;
            let m0 = self.metric[p0] + o0;
            let m1 = self.metric[p1] + o1;
            if m0 >= m1 {
                next[ns] = m0;
                cur[ns] = 0;
            } else {
                next[ns] = m1;
                cur[ns] = 1;
            }
        }
        let best = next.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        if best.is_finite() {
            next.iter_mut().for_each(|m| *m -= best);
        }
        self.metric = next;
        self.ptr = (self.ptr + 1) % self.depth;
        if !self.started || self.hist.len() < self.depth {
            return None;
        }
        // Trace back from the best current state through the whole ring.
        let mut s =
            (0..self.n).max_by(|&x, &y| self.metric[x].total_cmp(&self.metric[y])).unwrap_or(0);
        let mut bit = 0u8;
        for step in 0..self.depth {
            let idx = (self.ptr + step) % self.depth;
            bit = (s & 1) as u8;
            s = (s >> 1) | ((self.hist[idx][s] as usize) * (self.n / 2));
        }
        Some(bit)
    }
}

/// The receiver. Feed audio, get text.
pub struct IfkRx {
    variant: IfkVariant,
    centre: f64,
    resampler: Option<MonoResampler>,
    scratch: Vec<f32>,
    spacing: f64,
    paths: usize,
    bank: Bank,
    hop: usize,
    count: usize,
    hop_idx: usize,
    /// Where the next decision falls, in hops — **fractional**, because the timing
    /// loop corrects by a fraction of a hop and rounding it away every decision is
    /// how a loop stalls short of its own target.
    next_dec: f64,
    /// The last two symbols of tone energies, one row per hop: this is the
    /// reference's `pipe`, the thing a transition can be found in.
    hist: Vec<Vec<f32>>,
    hist_pos: usize,
    /// The filtered timing error, in hops.
    sync_err: f64,
    /// The tone of the symbol decided immediately before, if any.
    prev_tone: Option<usize>,
    prev_pos: Option<usize>,
    ratio: f64,
    interleaver: Interleaver,
    viterbi: SoftViterbi,
    pair: [u8; 2],
    symcounter: bool,
    varicode: MfskVaricodeRx,
    decoded: String,
    nib: usize,
    symbuf: [u8; 4],
    pub locked: bool,
    pub snr_db: f32,
    /// The capture was taken on the wrong sideband: the tone steps run the other
    /// way. IFK carries data in the *step*, so this is the only place it matters.
    pub reverse: bool,
    /// Every step symbol decided, in order, with `0xFF` for one the receiver
    /// refused. Kept for the same reason `MfskRx::tones` is: the only way to find
    /// a convention difference between two implementations is to compare the
    /// sequence, because the decoded text cannot say which symbol was wrong.
    pub steps: Vec<u8>,
    /// The raw bank position each step was read from, for the same reason.
    pub positions: Vec<u32>,
}

impl IfkRx {
    pub fn new(rate: f64, centre: f64, variant: IfkVariant) -> Self {
        let spacing = variant.spacing();
        // fldigi's `paths` is **5 at every variant** (3 only on its slow-CPU path),
        // with `extones` nine tones either side. This used to pick 3 for a
        // double-spaced tone, which changes the bank's sub-bin resolution and the
        // `fdiff / paths` divisor together and mis-decided about one symbol in nine.
        let paths = 5;
        let hop = (variant.symlen / HOPS).max(1);
        let interleaver = Interleaver::new(4, variant.depth.max(1), false);
        let viterbi = SoftViterbi::new(
            if variant.k15 { 15 } else { 7 },
            if variant.k15 { 0o44735 } else { 0o155 },
            if variant.k15 { 0o63057 } else { 0o117 },
            if variant.k15 { 180 } else { 45 },
        );
        let mut rx = IfkRx {
            variant,
            centre,
            resampler: MonoResampler::new(rate, variant.fs),
            scratch: Vec::new(),
            spacing,
            paths,
            bank: Bank::new(1, variant.fs, &[0.0]),
            hop,
            count: 0,
            hop_idx: 0,
            next_dec: HOPS as f64,
            hist: vec![vec![0.0; TONES]; 2 * HOPS],
            hist_pos: 0,
            sync_err: 0.0,
            prev_tone: None,
            prev_pos: None,
            ratio: 1.0,
            interleaver,
            viterbi,
            pair: [0; 2],
            symcounter: false,
            varicode: MfskVaricodeRx::new(),
            decoded: String::new(),
            nib: 0,
            symbuf: [0; 4],
            locked: false,
            snr_db: 0.0,
            reverse: false,
            steps: Vec::new(),
            positions: Vec::new(),
        };
        rx.build();
        rx
    }

    pub fn variant(&self) -> IfkVariant {
        self.variant
    }

    fn build(&mut self) {
        let bin = self.variant.fs / self.variant.symlen as f64;
        let f0 = self.centre - TONES as f64 * self.spacing / 2.0;
        let start = f0 - EXTRA as f64 * self.spacing;
        let total = ((TONES + 2 * EXTRA) * self.variant.ds * self.paths) as usize;
        let mut freqs = Vec::with_capacity(total);
        for m in 0..total {
            let f = start + m as f64 * bin / self.paths as f64;
            if f > 80.0 && f < self.variant.fs / 2.0 - 100.0 {
                freqs.push(f);
            }
        }
        self.bank = Bank::new(self.variant.symlen, self.variant.fs, &freqs);
        self.count = 0;
        self.hop_idx = 0;
        self.next_dec = HOPS as f64;
        self.hist_pos = 0;
        self.sync_err = 0.0;
        self.prev_tone = None;
        for row in self.hist.iter_mut() {
            row.iter_mut().for_each(|v| *v = 0.0);
        }
        self.prev_pos = None;
        self.ratio = 1.0;
        self.interleaver = Interleaver::new(4, self.variant.depth.max(1), false);
        let k = if self.variant.k15 { 15 } else { 7 };
        self.viterbi = SoftViterbi::new(
            k,
            if self.variant.k15 { 0o44735 } else { 0o155 },
            if self.variant.k15 { 0o63057 } else { 0o117 },
            if self.variant.k15 { 180 } else { 45 },
        );
        self.pair = [0; 2];
        self.symcounter = false;
        self.varicode = MfskVaricodeRx::new();
        self.decoded.clear();
        self.steps.clear();
        self.positions.clear();
        self.nib = 0;
    }

    fn bins(&self) -> usize {
        self.bank.bins()
    }

    pub fn feed(&mut self, audio: &[f32]) {
        if self.resampler.is_some() {
            self.scratch.clear();
            let mut done = Vec::new();
            self.resampler.as_mut().expect("checked").push(audio, &mut done);
            self.scratch = done;
            let scratch = std::mem::take(&mut self.scratch);
            for &v in &scratch {
                self.sample(v);
            }
            self.scratch = scratch;
        } else {
            for &v in audio {
                self.sample(v);
            }
        }
    }

    fn sample(&mut self, v: f32) {
        self.bank.feed(v as f64);
        self.count += 1;
        if self.count >= self.hop {
            self.count = 0;
            self.hop_step();
        }
    }

    fn peak(&self) -> (usize, f64, f64) {
        let mut mx = 0.0;
        let mut sum = 0.0;
        let mut pos = 0;
        for m in 0..self.bins() {
            let a = self.bank.mag(m);
            sum += a;
            if a > mx {
                mx = a;
                pos = m;
            }
        }
        (pos, mx, sum)
    }

    /// One hop of the bank: record the tone energies, and decide a symbol when the
    /// clock says so.
    ///
    /// The timing loop is the reference's, not a "which sample looks sharpest"
    /// search. `dominoex::synchronize()` keeps two symbols of FFT vectors and, when
    /// the tone changes, finds the offset at which the **previous** tone's energy is
    /// greatest — the middle of the symbol that has just gone by. That offset is the
    /// error signal, filtered and applied slowly. Chasing peak sharpness instead
    /// gives a loop with nothing to lock to: it walks, corrects, and walks back,
    /// which is exactly the periodic slip this receiver used to show.
    fn hop_step(&mut self) {
        self.hop_idx += 1;
        // Record this hop's eighteen tone energies.
        {
            let dbins = self.variant.ds * self.paths;
            let base = EXTRA * self.paths * self.variant.ds;
            let bins = self.bins();
            let hp = self.hist_pos;
            for t in 0..TONES {
                let lo = base + t * dbins;
                let mut best = 0.0f64;
                for m in lo..(lo + dbins).min(bins) {
                    let a = self.bank.mag(m);
                    if a > best {
                        best = a;
                    }
                }
                self.hist[hp][t] = best as f32;
            }
            self.hist_pos = (self.hist_pos + 1) % (2 * HOPS);
        }
        if (self.hop_idx as f64) < self.next_dec {
            return;
        }
        let (pos, mx, sum) = self.peak();
        self.decide(pos, mx, sum);

        // Where did the symbol before this one actually peak? The ring is oldest
        // first once rotated by `hist_pos`, and the previous symbol occupies its
        // first half, so its middle sits at HOPS/2.
        if let Some(tone) = self.prev_tone {
            let mut best = (0.0f32, 0usize);
            for k in 0..(2 * HOPS) {
                let idx = (self.hist_pos + k) % (2 * HOPS);
                let v = self.hist[idx][tone.min(TONES - 1)];
                if v > best.0 {
                    best = (v, k);
                }
            }
            let err = best.1 as f64 - (HOPS / 2) as f64;
            if err.abs() <= HOPS as f64 / 2.0 {
                // A slow filter: one symbol's measurement is noisy, and a loop that
                // believes each one is a loop that jitters.
                self.sync_err += (err - self.sync_err) * 0.1;
            }
        }
        self.prev_tone = Some(pos / (self.variant.ds * self.paths));
        // The correction is a fraction of a hop per symbol, so the position has to
        // stay fractional — truncating it every decision stalls the loop a whole
        // hop short, the same fixed-point trap per-column flattening hit.
        self.next_dec += HOPS as f64 - self.sync_err * 0.5;
        if self.next_dec < self.hop_idx as f64 {
            self.next_dec = self.hop_idx as f64 + 1.0;
        }
    }

    fn decide(&mut self, pos: usize, mx: f64, sum: f64) {
        let bins = self.bins().max(1);
        let others = (sum - mx) / (bins - 1) as f64;
        let r = if others > 1e-12 { mx / others } else { 1.0 };
        self.ratio += (r - self.ratio) / 12.0;
        self.snr_db = (20.0 * self.ratio.max(1.0).log10()) as f32;
        let burst = mx < 1.2 * sum / bins as f64;
        let Some(prev) = self.prev_pos else {
            self.prev_pos = Some(pos);
            return;
        };
        if sum < 1e-9 {
            self.prev_pos = Some(pos);
            return;
        }
        let mut fdiff = (pos as f64 - prev as f64) / self.paths as f64 / self.variant.ds as f64;
        if self.reverse {
            fdiff = -fdiff;
        }
        self.prev_pos = Some(pos);
        let raw = fdiff.round() as i32;
        let symbol = step_symbol(raw);
        let out_of_range = symbol.is_none();
        let c = symbol.unwrap_or(0) as i32;
        self.steps.push(symbol.unwrap_or(0xFF));
        self.positions.push(pos as u32);
        let sig = self.ratio >= LOCK_RATIO;
        if self.variant.thor {
            let mut s = [0u8; 4];
            if burst || out_of_range || !sig {
                s = [128; 4];
            } else {
                let mut v = c as u32 & 0xF;
                for (i, sv) in s.iter_mut().enumerate() {
                    let bit = (v >> (3 - i)) & 1;
                    *sv = if bit == 1 { 255 } else { 0 };
                }
                v = 0;
                let _ = v;
            }
            self.interleaver.symbols(&mut s);
            for sv in s {
                self.pairs(sv);
            }
        } else {
            self.locked = sig;
            if c & 8 == 0 {
                if self.nib > 0 && self.nib <= 3 {
                    let mut key = 0usize;
                    for i in 0..self.nib {
                        key |= (self.symbuf[i] as usize) << (4 * i);
                    }
                    if let Some(ch) = domino_char(key) {
                        if sig && !burst && !out_of_range {
                            self.push_char(ch);
                        }
                    }
                }
                self.nib = 0;
            }
            for i in (1..4).rev() {
                self.symbuf[i] = self.symbuf[i - 1];
            }
            self.symbuf[0] = if out_of_range || c > 15 { 0 } else { c as u8 };
            self.nib = (self.nib + 1).min(4);
        }
    }

    fn pairs(&mut self, s: u8) {
        self.pair[0] = self.pair[1];
        self.pair[1] = s;
        self.symcounter = !self.symcounter;
        if self.symcounter {
            return;
        }
        let a = (self.pair[0] as f32 - 128.0) / 128.0;
        let b = (self.pair[1] as f32 - 128.0) / 128.0;
        let Some(bit) = self.viterbi.push(a, b) else {
            return;
        };
        self.locked = self.ratio >= LOCK_RATIO;
        if !self.locked {
            return;
        }
        if let Some(c) = self.varicode.push_bit(bit) {
            self.push_char(c);
        }
    }

    fn push_char(&mut self, c: char) {
        match c {
            '\r' | '\n' => {
                if !self.decoded.ends_with('\n') && !self.decoded.is_empty() {
                    self.decoded.push('\n');
                }
            }
            c if c >= ' ' && c != '\u{7f}' => self.decoded.push(c),
            _ => {}
        }
    }

    pub fn text(&self) -> &str {
        &self.decoded
    }

    /// The step symbols this receiver decided, in order.
    pub fn steps(&self) -> &[u8] {
        &self.steps
    }

    /// The raw bank positions the steps came from.
    pub fn positions(&self) -> &[u32] {
        &self.positions
    }
}

/// The character whose nibble key is `key` — the last nibble lowest, as the
/// reference builds it.
fn domino_char(key: usize) -> Option<char> {
    (0..256u32).find_map(|c| {
        let n = DOMINO_CODE[c as usize];
        let mut len = 1;
        while len < 3 && n[len] & 8 != 0 {
            len += 1;
        }
        let mut k = 0usize;
        for i in 0..len {
            k |= (n[len - 1 - i] as usize) << (4 * i);
        }
        if k == key { char::from_u32(c) } else { None }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_names_resolve() {
        assert_eq!(IfkVariant::named("THOR16").map(|v| v.symlen), Some(512));
        assert_eq!(IfkVariant::named("domex8").map(|v| v.ds), Some(2));
        assert_eq!(IfkVariant::named("THOR25x4").map(|v| v.ds), Some(4));
        assert_eq!(IfkVariant::named("THOR25").map(|v| v.symlen), Some(320));
    }

    /// Every THOR recording, measured. The variant table is the reference's; the
    /// corpus has two names it does not (THOR25, THOR44, THOR56, THORM), which
    /// Every tone step maps to its symbol, including the one that wraps.
    ///
    /// The wrap is the half of the alphabet that was missing: with the raw
    /// differences 0 and 1 called "out of range", symbols 16 and 17 were thrown
    /// away on every transmission, and this fails on that version.
    #[test]
    fn every_step_maps_to_its_symbol_and_the_wrap_is_kept() {
        for s in 0..TONES as i32 {
            assert_eq!(step_symbol(2 + s), Some(s as u8), "step {}", 2 + s);
            assert_eq!(
                step_symbol(2 + s - TONES as i32),
                Some(s as u8),
                "wrapped step {}",
                2 + s - TONES as i32
            );
        }
        // A bin and a half outside the bank is not a step.
        assert_eq!(step_symbol(20), None);
        assert_eq!(step_symbol(-17), None);
    }

    /// What DominoEX's steps actually are, against the text we know was sent.
    ///
    /// This is the measurement the decoded text cannot make. The alphabet maps
    /// each character to one to three nibbles (`DOMINO_CODE`) and the transmitter
    /// sends each as `2 + nibble (mod 18)`, so the recording must carry the
    /// expected nibble sequence inside the step trace. A receiver that is right
    /// about everything but one convention still reads almost none of the text,
    /// so the figure to watch is not "does it decode" but **what fraction of the
    /// expected nibbles are found at the best alignment** — chance is 1/18, and a
    /// near-miss shows up as a fraction in between.
    #[test]
    #[ignore]
    fn the_dominoex_steps_carry_the_known_text() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/DOMX16.wav");
        let mut reader = hound::WavReader::open(path).expect("the DOMX16 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let variant = IfkVariant::named("DOMX16").expect("a variant");
        let expected: Vec<u8> = "TEST THE EAGLE"
            .chars()
            .flat_map(|c| {
                let code = DOMINO_CODE[c as usize];
                code.iter().copied().take_while(|&n| n != 0).collect::<Vec<u8>>()
            })
            .collect();
        let mirror: Vec<u8> = expected.iter().map(|n| 17 - n).collect();
        eprintln!("expected nibbles ({}): {:?}", expected.len(), expected);
        let mut best = (0.0f32, 0.0f64, false, 0usize);
        for hz in [1670.0f64, 1700.0, 1730.0, 1760.0, 1790.0] {
            for reverse in [false, true] {
                let mut rx = IfkRx::new(spec.sample_rate as f64, hz, variant);
                rx.reverse = reverse;
                for chunk in audio.chunks(512) {
                    rx.feed(chunk);
                }
                rx.feed(&vec![0.0f32; 16000]);
                let steps = rx.steps();
                if steps.len() < expected.len() {
                    continue;
                }
                for want in [&expected, &mirror] {
                    for at in 0..=steps.len() - want.len() {
                        let hit = steps[at..at + want.len()]
                            .iter()
                            .zip(want)
                            .filter(|(a, b)| a == b)
                            .count();
                        let frac = hit as f32 / want.len() as f32;
                        if frac > best.0 {
                            best = (frac, hz, reverse, at);
                        }
                    }
                }
            }
        }
        // The pattern of the mismatches is what names the fault, so print the
        // two sequences side by side at the best alignment.
        for hz in [best.1] {
            let mut rx = IfkRx::new(spec.sample_rate as f64, hz, variant);
            rx.reverse = best.2;
            for chunk in audio.chunks(512) {
                rx.feed(chunk);
            }
            rx.feed(&vec![0.0f32; 16000]);
            let st = rx.steps();
            if best.3 + expected.len() <= st.len() {
                eprintln!("  want: {:?}", expected);
                eprintln!("  got : {:?}", &st[best.3..best.3 + expected.len()]);
                let ps = rx.positions();
                eprintln!("  pos : {:?}", &ps[best.3..(best.3 + 20).min(ps.len())]);
                let d: Vec<i64> = ps
                    .windows(2)
                    .skip(best.3)
                    .take(20)
                    .map(|w| w[1] as i64 - w[0] as i64)
                    .collect();
                eprintln!("  diffs: {d:?}  (a step of s is {} sub-bins)", variant.ds * 5);
            }
        }
        eprintln!(
            "best alignment: {:.0}% of the expected nibbles at {:.0} Hz, reverse {}, step {} \
             (chance is {:.0}%)",
            best.0 * 100.0,
            best.1,
            best.2,
            best.3,
            100.0 / 18.0
        );
        // Where this stands. When the fraction climbs past about a third, the
        // convention has been found: make this the gate, with the centre and
        // polarity it printed.
        assert!(
            best.0 < 0.35,
            "the steps are mostly right now ({:.0}%) — make this the gate",
            best.0 * 100.0
        );
    }

    /// What each recording gives, measured, and what must hold.
    ///
    /// State 2026-10-22, after fldigi's own alphabet and `paths = 5`: the DominoEX
    /// half **reads text** — `DOMX11` its whole message, and five more read most of
    /// theirs — and **every THOR variant still reads nothing**. THOR is the FEC
    /// branch (interleaver, K = 7 Viterbi, MFSK varicode) and a THOR recording
    /// produces no characters at all, which is one fault in that branch rather than
    /// something in the audio front end it shares with DominoEX.
    #[test]
    #[ignore]
    fn the_corpus_ifk_samples_are_measured() {
        use std::path::Path;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi");
        let words = ["EAGLE", "LANDED", "SDRBROWN", "12345"];
        for name in [
            "THOR4", "THOR5", "THOR8", "THOR11", "THOR16", "THOR22", "THOR25", "THOR25x4",
            "THOR32", "THOR44", "THOR50x1", "THOR50x2", "THOR56", "THOR100", "THORM", "DOMEX4",
            "DOMEX5", "DOMEX8", "DOMEXM", "DOMX11", "DOMX16", "DOMX22", "DOMX44", "DOMX88",
        ] {
            let Some(variant) = IfkVariant::named(name) else {
                eprintln!("{name}: no reference variant");
                continue;
            };
            let path = format!("{root}/{name}.wav");
            if !Path::new(&path).exists() {
                eprintln!("{name}: no sample");
                continue;
            }
            let mut reader = hound::WavReader::open(&path).expect("open");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let mut best = (0usize, String::new(), 0.0f64);
            for reverse in [false, true] {
                for hz in [1670.0f64, 1700.0, 1730.0, 1760.0, 1790.0] {
                    let mut rx = IfkRx::new(spec.sample_rate as f64, hz, variant);
                    rx.reverse = reverse;
                    for chunk in audio.chunks(512) {
                        rx.feed(chunk);
                    }
                    rx.feed(&vec![0.0f32; 16000]);
                    let hits = words.iter().filter(|w| rx.text().contains(**w)).count();
                    if hits > best.0 {
                        best = (hits, rx.text().to_string(), hz);
                    }
                }
            }
            eprintln!("{name}: {}/4 at {}: {:?}", best.0, best.2, best.1);
            if name == "DOMX11" {
                assert!(
                    best.0 == words.len(),
                    "DOMX11 read {}/4 — the DominoEX half of this family decodes, and this \
                     recording is what pins it",
                    best.0
                );
            }
            if name.starts_with("THOR") {
                assert!(
                    best.0 == 0,
                    "{name} read {}/4 — if THOR has started working, say so in \
                     MODE-PROGRESS.md and take this assertion out",
                    best.0
                );
            }
        }
    }
}
