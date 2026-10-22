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
use crate::fec::{ConvCode, ConvEncoder};
use crate::mfsk_rx::{Bank, Interleaver, MfskRx, MfskVaricodeRx};

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

pub const THOR: [IfkVariant; 15] = [
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
    // The three the corpus has and this table did not (`src/thor/thor.cxx`): 256
    // samples at 8 kHz and at 11.025 kHz, and 290 at 16 kHz.
    IfkVariant {
        name: "THOR32",
        thor: true,
        fs: 8000.0,
        symlen: 256,
        ds: 1,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR44",
        thor: true,
        fs: 11025.0,
        symlen: 256,
        ds: 2,
        depth: 10,
        k15: false,
    },
    IfkVariant {
        name: "THOR56",
        thor: true,
        fs: 16000.0,
        symlen: 290,
        ds: 1,
        depth: 10,
        k15: false,
    },
    // fldigi's micro submode: a 4000-sample symbol at 8 kHz and a short
    // interleave; the corpus's THORM is this one.
    IfkVariant { name: "THORM", thor: true, fs: 8000.0, symlen: 4000, ds: 1, depth: 4, k15: false },
    // fldigi's THOR25 is its own mode — 320 samples, *single* spaced, and a
    // 25-deep interleave (its `idepth = 25; // 1 sec interleave`). It is not
    // THOR25x4 under another name.
    IfkVariant {
        name: "THOR25",
        thor: true,
        fs: 8000.0,
        symlen: 320,
        ds: 1,
        depth: 25,
        k15: false,
    },
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
    /// Steps pushed. The traceback is only worth reading once the ring is full;
    /// before that it walks through slots that were never written.
    pushed: usize,
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
            pushed: 0,
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
        self.pushed += 1;
        if self.pushed < self.depth {
            return None;
        }
        // Trace back from the best current state through the whole ring, **newest
        // step first**, and hand out the bit the oldest step settled on.
        //
        // This walk is the one thing a streaming Viterbi gets wrong silently: the
        // ring is written oldest-slot-first, so stepping through it in write order
        // applies the oldest survivor to the newest state and every bit after the
        // first comes out of a chain that does not exist. THOR read no characters
        // at all on that version, and the DominoEX half — which has no code — was
        // unaffected, which is what made it look like a framing problem.
        let mut s =
            (0..self.n).max_by(|&x, &y| self.metric[x].total_cmp(&self.metric[y])).unwrap_or(0);
        let mut bit = 0u8;
        for step in 0..self.depth {
            let idx = (self.ptr + self.depth - 1 - step) % self.depth;
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
        // **Not `None`.** fldigi starts its receiver with `prev1symbol = 0`, so
        // the very first symbol still produces a step — a bad one, which is
        // punctured, but the symbol *count* is what pairs the code. Skipping it
        // shifts every soft-symbol pair by one for the rest of the over and THOR
        // then decodes noise however clean the signal is.
        self.prev_pos = Some(0);
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
            // The previous symbol's centre, as the energy centroid of its own
            // tone over the older half of the ring. An argmax is quantised to a
            // whole hop and a loop fed a quantised error limit-cycles around it;
            // the centroid is smooth, and a symbol is 16 hops wide so a half-hop
            // of smoothing costs nothing.
            let mut num = 0.0;
            let mut den = 0.0;
            let mut best = (0.0f32, 0usize);
            for k in 0..(2 * HOPS) {
                let idx = (self.hist_pos + k) % (2 * HOPS);
                let v = self.hist[idx][tone.min(TONES - 1)] as f64;
                num += v * k as f64;
                den += v;
                if v as f32 > best.0 {
                    best = (v as f32, k);
                }
            }
            let centre = if den > 1e-12 { num / den } else { best.1 as f64 };
            // The bank is a matched filter over one symbol, so a tone's energy
            // peaks when its window exactly spans that tone — at the symbol's
            // **end**, not its middle. Measured against the middle, the loop pulls
            // itself half a symbol late and every decision lands on a boundary.
            let err = centre - (HOPS - 1) as f64;
            if std::env::var("IFK_DEBUG").is_ok() {
                let row: Vec<u8> = (0..2 * HOPS)
                    .map(|k| {
                        (self.hist[(self.hist_pos + k) % (2 * HOPS)][tone.min(TONES - 1)]
                            .min(255.0)) as u8
                    })
                    .collect();
                eprintln!(
                    "dec hop={} next={:.2} tone={} centre={:.2} err={:.2} sync={:.2} row={:?}",
                    self.hop_idx, self.next_dec, tone, centre, err, self.sync_err, row
                );
            }
            if std::env::var("IFK_DEBUG").is_ok() {
                eprintln!(
                    "dec hop_idx={} next_dec={:.2} prev_tone={} best_idx={} err={:.1} sync={:.2}",
                    self.hop_idx, self.next_dec, tone, best.1, err, self.sync_err
                );
            }
            if err.abs() <= HOPS as f64 / 2.0 {
                // A slow filter: one symbol's measurement is noisy, and a loop that
                // believes each one is a loop that jitters.
                self.sync_err += (err - self.sync_err) * 0.1;
            }
        }
        // The bank carries `EXTRA` tones either side of the alphabet, so the
        // peak's slot has to have them taken off before it names a tone — without
        // it the clock looks at tone 19, 21, 25 (never a real tone) and the
        // timing loop has nothing to measure, which is what it did.
        let slot = pos / (self.variant.ds * self.paths);
        self.prev_tone = Some(slot.saturating_sub(EXTRA).min(TONES - 1));
        // The correction is a fraction of a hop per symbol, so the position has to
        // stay fractional — truncating it every decision stalls the loop a whole
        // hop short, the same fixed-point trap per-column flattening hit.
        // **The sign is the whole loop.** Measured in the ring's own order, a
        // clock running late puts the previous symbol's peak *earlier* in the
        // window, so the correction has to be added: applied with the other sign
        // it is positive feedback, and the clock walks itself off the symbols
        // within a second of audio.
        // Clamped: an unclamped correction can shorten a symbol by half, and a
        // decision taken twice inside one symbol puts a *zero step* in the stream
        // — a symbol that carries no data and shifts everything after it.
        self.next_dec += HOPS as f64 + (self.sync_err * 0.5).clamp(-1.5, 1.5);
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

/// Synthesise `text` as this variant's audio, at the variant's own rate.
///
/// This is fldigi's `sendchar`/`sendsymbol` path: the MFSK varicode bits, the
/// K = 7 or K = 15 convolutional code, the 4 × 4 × depth interleaver, and the
/// IFK+ step `2 + nibble`. It exists for the same reason the MFSK modulator
/// does — a receiver can only be debugged against a signal whose every bit is
/// known, and the corpus recordings cannot say which side of a disagreement is
/// wrong.
///
/// **A loopback proves self-consistency, not interoperability.** What it proves
/// here is that the front end, the clock and the FEC branch agree with each
/// other; that the corpus reads is the separate, real-path measurement.
pub fn encode(variant: IfkVariant, centre: f64, text: &str) -> Vec<f32> {
    encode_with_steps(variant, centre, text).0
}

/// The same, and the step nibble it sent at each symbol — the sequence a
/// receiver has to reproduce, which the text alone cannot name.
pub fn encode_with_steps(variant: IfkVariant, centre: f64, text: &str) -> (Vec<f32>, Vec<u8>) {
    let spacing = variant.spacing();
    let code = if variant.k15 {
        ConvCode { poly_a: 0o44735, poly_b: 0o63057, invert_b: false }
    } else {
        ConvCode { poly_a: 0o155, poly_b: 0o117, invert_b: false }
    };
    let mut tx = Tx {
        variant,
        centre,
        spacing,
        prev_tone: 0,
        phase: 0.0,
        out: Vec::new(),
        enc: ConvEncoder::new(code),
        inlv: Interleaver::new(4, variant.depth.max(1), true),
        sh: [0; 4],
        filled: 0,
        steps: Vec::new(),
    };
    // `Clearbits`: 1400 encoded zeros into the interleaver, not transmitted. It
    // leaves the interleaver holding the same filler the receiver's does.
    for _ in 0..1400 {
        let d = tx.enc.push(0);
        tx.prime_bit(d & 1);
        tx.prime_bit((d >> 1) & 1);
    }
    // Sixteen steps of two tones each: the preamble the receiver's clock locks on.
    for _ in 0..16 {
        tx.symbol(0);
    }
    // The idle character, then the reference's CR STX CR, then the text.
    tx.push_char('\0');
    for c in ['\r', '\u{2}', '\r'] {
        tx.push_char(c);
    }
    for c in text.chars() {
        tx.push_char(c);
    }
    tx.push_char('\r');
    tx.push_char('\u{4}');
    tx.push_char('\r');
    // A tail of idle characters, so the last character drains the interleaver
    // and the decoder's traceback window instead of being cut in half.
    for _ in 0..8 {
        tx.push_char('\0');
    }
    (tx.out, tx.steps)
}

struct Tx {
    variant: IfkVariant,
    centre: f64,
    spacing: f64,
    prev_tone: usize,
    phase: f64,
    out: Vec<f32>,
    enc: ConvEncoder,
    inlv: Interleaver,
    sh: [u8; 4],
    filled: usize,
    steps: Vec<u8>,
}

impl Tx {
    fn prime_bit(&mut self, bit: u8) {
        self.sh[self.filled] = bit;
        self.filled += 1;
        if self.filled == 4 {
            let mut syms = self.sh;
            self.inlv.symbols(&mut syms);
            self.sh = syms;
            self.filled = 0;
        }
    }

    fn push_char(&mut self, c: char) {
        for bit in MfskRx::varicode_bits(c) {
            let d = self.enc.push(bit);
            // `sendchar` writes bit 0 of the code word first.
            self.push_bit(d & 1);
            self.push_bit((d >> 1) & 1);
        }
    }

    fn push_bit(&mut self, bit: u8) {
        self.sh[self.filled] = bit;
        self.filled += 1;
        if self.filled == 4 {
            let mut syms = self.sh;
            self.inlv.symbols(&mut syms);
            let nib = syms.iter().fold(0u8, |v, &b| (v << 1) | (b & 1));
            self.symbol(nib);
            self.filled = 0;
        }
    }

    /// One symbol: the tone is `2 + nibble` above the previous one, and it is
    /// held for `symlen` samples at the variant's rate.
    fn symbol(&mut self, nibble: u8) {
        self.steps.push(nibble);
        let tone = (self.prev_tone + 2 + nibble as usize) % TONES;
        self.prev_tone = tone;
        let f =
            self.centre - TONES as f64 * self.spacing / 2.0 + (tone as f64 + 0.5) * self.spacing;
        let step = 2.0 * std::f64::consts::PI * f / self.variant.fs;
        let n = self.variant.symlen;
        for _ in 0..n {
            self.out.push(self.phase.cos() as f32);
            self.phase += step;
            if self.phase > std::f64::consts::TAU {
                self.phase -= std::f64::consts::TAU;
            }
        }
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
    use crate::fec::ConvCode;

    /// Encode a bit stream with a code, the way the transmitter does: the state's
    /// low bit is the newest input bit and the code word is the parity of the
    /// register against each polynomial (fldigi's `encoder`/`viterbi::output`).
    fn encode_bits(bits: &[u8], code: ConvCode) -> Vec<(u8, u8)> {
        let n = code.states();
        let mut s = 0usize;
        let mut out = Vec::with_capacity(bits.len());
        for &b in bits {
            let reg = ((s as u32) << 1) | b as u32;
            out.push((
                (reg & code.poly_a).count_ones() as u8 & 1,
                ((reg & code.poly_b).count_ones() as u8 & 1) ^ code.invert_b as u8,
            ));
            s = (reg as usize) & (n - 1);
        }
        out
    }

    /// The whole THOR branch, without the air: bits → code → interleaver → soft
    /// symbols → interleaver → paired Viterbi → bits.
    ///
    /// This is the gate for the traceback. The ring is written oldest slot first,
    /// so walking it in write order applies the oldest survivor to the newest state
    /// and every bit after the first is nonsense — which reads exactly like "THOR
    /// has a framing problem" and is why it went unnoticed: the DominoEX half of
    /// this file has no code and decoded perfectly throughout.
    #[test]
    fn the_thor_chain_reads_back_the_bits_it_was_given() {
        // A pseudo-random stream, so a decoder that merely favours zeros cannot pass.
        let mut seed = 0x1234_5678u32;
        let bits: Vec<u8> = (0..1200)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                ((seed >> 16) & 1) as u8
            })
            .collect();
        for (label, code, depth, tb) in [
            (
                "THOR's K = 7",
                ConvCode { poly_a: 0o155, poly_b: 0o117, invert_b: false },
                10usize,
                45usize,
            ),
            (
                "THOR's K = 15",
                ConvCode { poly_a: 0o44735, poly_b: 0o63057, invert_b: false },
                50,
                180,
            ),
        ] {
            let coded = encode_bits(&bits, code);
            // The transmitter's own order: the two outputs of a bit go out as
            // `[o1, o2]`, four of those make one symbol, and the word's most
            // significant bit is the first one sent (`sendchar` → `bits()`).
            let flat: Vec<u8> = coded.iter().flat_map(|&(a, b)| [a, b]).collect();
            let mut tx = Interleaver::new(4, depth, true);
            let mut rx = Interleaver::new(4, depth, false);
            let k = code.k() as usize;
            let mut viterbi = SoftViterbi::new(k, code.poly_a, code.poly_b, tb);
            let mut pair = [128u8; 2];
            let mut counter = false;
            let mut got = Vec::new();
            // The transmitter packs four code words per symbol, most significant
            // first (`decodesymbol`'s own order), and interleaves them.
            for group in flat.chunks(4) {
                let mut syms = [128u8; 4];
                for (i, &bit) in group.iter().enumerate() {
                    syms[i] = if bit == 1 { 255 } else { 0 };
                }
                tx.symbols(&mut syms);
                rx.symbols(&mut syms);
                for sv in syms {
                    pair[0] = pair[1];
                    pair[1] = sv;
                    counter = !counter;
                    if counter {
                        continue;
                    }
                    let a = (pair[0] as f32 - 128.0) / 128.0;
                    let b = (pair[1] as f32 - 128.0) / 128.0;
                    if let Some(bit) = viterbi.push(a, b) {
                        got.push(bit);
                    }
                }
            }
            let lag = (0..got.len().min(400))
                .max_by_key(|&lag| {
                    bits.iter().zip(&got[lag..]).take(600).filter(|(a, b)| *a == *b).count()
                })
                .expect("a lag");
            let same = bits.iter().zip(&got[lag..]).filter(|(a, b)| *a == *b).count();
            let compared = bits.len().min(got.len() - lag);
            assert!(
                same == compared,
                "{label}: {same} of {compared} bits came back (lag {lag}) — the traceback \
                 walks the ring in the wrong order"
            );
        }
    }

    /// The receiver against a signal built from the reference's own transmit
    /// path: varicode → code → interleaver → steps → tones.
    ///
    /// This is the experiment the corpus cannot do. A clean synthetic that reads
    /// says the front end, the clock and the FEC branch agree, and that a
    /// recording which does not read is either marginal or tuned elsewhere; a
    /// synthetic that does not read is a receiver fault with no noise in it to
    /// blame.
    #[test]
    fn a_synthetic_thor_transmission_reads_back() {
        for name in [
            "THOR16", "THOR8", "THOR4", "THOR22", "THOR25", "THOR32", "THOR44", "THOR56",
            "THOR50x2",
        ] {
            let variant = IfkVariant::named(name).expect("a variant");
            let text = "TEST THE EAGLE HAS LANDED 12345 TEST THE EAGLE HAS LANDED 12345 TEST THE EAGLE HAS LANDED 12345";
            let (audio, sent) = encode_with_steps(variant, 1740.0, text);
            let mut rx = IfkRx::new(variant.fs, 1740.0, variant);
            for chunk in audio.chunks(512) {
                rx.feed(chunk);
            }
            rx.feed(&vec![0.0f32; 16_000]);
            eprintln!("{name}: {:?}", rx.text());
            let got = rx.steps();
            eprintln!("  sent[..40] {:?}", &sent[..40.min(sent.len())]);
            eprintln!("  got [..40] {:?}", &got[..40.min(got.len())]);
            let hit = sent.iter().zip(got).filter(|(a, b)| a == b).count();
            eprintln!("  {hit} of {} steps agree", sent.len().min(got.len()));
            for word in ["EAGLE", "LANDED", "12345"] {
                assert!(
                    rx.text().contains(word),
                    "{name} read {:?} from its own transmission, which should carry {text}",
                    rx.text()
                );
            }
        }
    }

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
    /// The longest run of the expected nibbles that appears, in order, in the
    /// trace. A plain sliding window cannot score a trace that drops or doubles a
    /// symbol — one slip at the front destroys every comparison after it — and the
    /// question here is how much of the text the *symbols* carry, not whether they
    /// landed on one grid.
    fn lcs(a: &[u8], b: &[u8]) -> usize {
        let mut prev = vec![0usize; b.len() + 1];
        let mut cur = vec![0usize; b.len() + 1];
        for &x in a {
            for (j, &y) in b.iter().enumerate() {
                cur[j + 1] = if x == y { prev[j] + 1 } else { cur[j].max(prev[j + 1]) };
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[b.len()]
    }

    /// What DominoEX's steps actually are, against the text we know was sent.
    ///
    /// This is the measurement the decoded text cannot make. The alphabet maps
    /// each character to one to three nibbles (`DOMINO_CODE`) and the transmitter
    /// sends each as `2 + nibble (mod 18)`, so the recording must carry the
    /// expected nibble sequence inside the step trace. A receiver that is right
    /// about everything but one convention still reads almost none of the text,
    /// so the figure to watch is **what share of the expected nibbles the trace
    /// carries**, scored as a subsequence: chance is 1/18, and a near-miss shows
    /// up as a fraction in between.
    #[test]
    #[ignore]
    fn the_dominoex_steps_carry_the_known_text() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi/DOMX11.wav");
        let mut reader = hound::WavReader::open(path).expect("the DOMX11 sample");
        let spec = reader.spec();
        let audio: Vec<f32> =
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
        let variant = IfkVariant::named("DOMX11").expect("a variant");
        let expected: Vec<u8> = "TEST THE EAGLE"
            .chars()
            .flat_map(|c| {
                let code = DOMINO_CODE[c as usize];
                code.iter().copied().take_while(|&n| n != 0).collect::<Vec<u8>>()
            })
            .collect();
        let mirror: Vec<u8> = expected.iter().map(|n| 17 - n).collect();
        let mut best = (0.0f32, 0.0f64, false, Vec::new());
        for hz in [1670.0f64, 1700.0, 1730.0, 1760.0, 1790.0] {
            for reverse in [false, true] {
                let mut rx = IfkRx::new(spec.sample_rate as f64, hz, variant);
                rx.reverse = reverse;
                for chunk in audio.chunks(512) {
                    rx.feed(chunk);
                }
                rx.feed(&vec![0.0f32; 16000]);
                let steps = rx.steps();
                for want in [&expected, &mirror] {
                    let frac = lcs(want, steps) as f32 / want.len() as f32;
                    if frac > best.0 {
                        best = (frac, hz, reverse, steps.to_vec());
                    }
                }
            }
        }
        eprintln!("  want: {:?}", expected);
        eprintln!("  got : {:?}", &best.3[..best.3.len().min(60)]);
        eprintln!(
            "best: {:.0}% of the expected nibbles at {:.0} Hz, reverse {} (chance is {:.0}%)",
            best.0 * 100.0,
            best.1,
            best.2,
            100.0 / 18.0
        );
        // **The gate.** DOMX11 reads its whole message through the corpus gate, so
        // its own step trace must carry the known text's nibbles; anything under
        // this and the front end has regressed, whatever the text says.
        assert!(
            best.0 > 0.9,
            "the steps carry only {:.0}% of the expected nibbles",
            best.0 * 100.0
        );
    }

    /// What each recording gives, measured, and what must hold.
    ///
    /// State 2026-10-22, after the four faults in "THOR" below were fixed: **eleven
    /// of the fifteen THOR variants read their whole text** (THOR4's sample stops
    /// 56 s in, mid-transmission) and **six of nine DominoEX** do. `IFK_ONLY=a,b`
    /// runs a subset; the full sweep is fourteen minutes because the wide variants
    /// are searched over 46 centres in both polarities.
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
            if let Ok(only) = std::env::var("IFK_ONLY") {
                if !only.split(',').any(|o| o.eq_ignore_ascii_case(name)) {
                    continue;
                }
            }
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
            // The corpus gives each file a *block* centre, and these recordings
            // were tuned by hand across a wide spread — the wide variants read at
            // 1150 Hz where the manifest says 1704. The search is the measurement,
            // so it is wide.
            for reverse in [false, true] {
                let mut hz = 1100.0f64;
                while hz <= 2000.0 {
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
                    hz += 20.0;
                }
            }
            eprintln!("{name}: {}/4 at {}: {:?}", best.0, best.2, best.1);
            // What each recording gives, measured 2026-10-22 after the FEC branch
            // was fixed. Eleven of the fifteen THOR variants read their whole
            // text; the exceptions say why they do not.
            let want = match name {
                "THOR5" | "THOR8" | "THOR11" | "THOR16" | "THOR22" | "THOR25x4" | "THOR32"
                | "THOR50x1" | "THOR50x2" | "THOR56" | "THOR100" => Some(4usize),
                // DominoEX reads six of its nine whole once the timing loop was
                // fixed (it was pulling half a symbol late, and the three that
                // read nothing before — DOMEX8, DOMX16, DOMX44 — all read now).
                "DOMX11" | "DOMEX8" | "DOMX16" | "DOMX22" | "DOMX44" | "DOMX88" => Some(4usize),
                "DOMEX5" => Some(2),
                "DOMEX4" => Some(1),
                // THOR4 runs at 3.9 symbols a second: its sample stops 56 s in,
                // mid-transmission, just after `LANDED`. Its own loopback reads the
                // whole text, so this is the length of the recording.
                "THOR4" => Some(2),
                // THOR25 and THOR44 read nothing at any centre or polarity while
                // their own transmissions read 439 of 440 steps — the recording is
                // the suspect, and a fresh one is in the journal's "Needs me".
                _ => None,
            };
            if let Some(want) = want {
                assert_eq!(
                    best.0, want,
                    "{name} read {}/4 words, expected {want}: {:?}",
                    best.0, best.1
                );
            }
        }
    }
}
