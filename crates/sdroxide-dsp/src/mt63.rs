//! MT63 — the 64-carrier Walsh-spread OFDM mode, receive only.
//!
//! Written from fldigi's `src/mt63/mt63base.cxx`, `mt63intl.dat` and
//! `symbol.dat` (GPL-3.0, Dave Freese W1HKJ, after Pawel Jalocha SP9VRC's
//! MT63ASC). fldigi and OnAir are credited in the README's Acknowledgements;
//! nothing here is copied by name, and the two data tables (the symbol shape and
//! the interleave patterns) are protocol constants.
//!
//! Structure, which is what makes this a day rather than a week once the
//! reference is read:
//!
//! * **64 carriers**, `DataCarrSepar = 4` FFT bins apart, in a 512-point FFT.
//!   The processing rate is twice the mode's bandwidth, so a carrier is
//!   `bw / 64` from its neighbour and the bank is exactly `bw` wide.
//! * **One character is one 64-bit Walsh codeword**: code `c < 64` is a positive
//!   impulse at `c`, `c >= 64` a negative impulse at `c - 64`. The inverse Walsh
//!   transform spreads it over all 64 carriers.
//! * **Differential phase**: a `0` bit flips the carrier's phase between
//!   symbols, a `1` leaves it. Nothing absolute is needed, so a mistuned
//!   receiver still reads it.
//! * **A block interleaver** of 32 (short) or 64 (long) characters spreads a
//!   burst across the Walsh codewords. `L` in the mode name is that long
//!   interleave.
//!
//! The carrier bank alternates between two FFT slices half a symbol apart
//! (`WindowLen * (i & 1)` on transmit): even carriers are in one, odd in the
//! other. The receiver therefore keeps both and takes carrier `i` from the slice
//! that carried it, which is what `decode_at` does.
//!
//! **Receiver only.** There is no modulator here; the corpus is the gate and
//! fldigi is the transmitter.

use num_complex::Complex32;
use rustfft::FftPlanner;

const CARRIERS: usize = 64;
/// The reference's FFT length and symbol hop at its own processing rate of
/// `2 · bandwidth`. Both are scaled to the incoming rate in `Mt63Rx::new`, and
/// then a carrier is always exactly four bins from its neighbour.
const WINDOW: usize = 512;
const SYMBOL: usize = 200;
const CARR_SEP: usize = 4;
/// Carriers looked at either side of the bank by the decoder, as fldigi does.
const MARGIN: usize = 8;
/// FFT bins scanned: the 64 carriers plus the margin on both sides.
const SCAN: usize = CARRIERS + 2 * MARGIN;
/// Time offsets the deinterleaver scores, `2 * MARGIN + 1`.
const SCAN_LEN: usize = 2 * MARGIN + 1;

/// The MT63 symbol shape, from fldigi's `mt63/symbol.dat` (`SymbolShape`,
/// 512 points). It is a raised-cosine-like pulse, zero at both ends.
#[rustfmt::skip]
const SYMBOL_SHAPE: [f32; WINDOW] = [
    -0.000000000, 0.000006650, 0.000026570, 0.000059750, 0.000106130, 0.000165620, 0.000238100, 0.000323410,
    0.000421340, 0.000531620, 0.000653890, 0.000787730, 0.000932610, 0.001087890, 0.001252830, 0.001426530,
    0.001607980, 0.001795990, 0.001989260, 0.002186280, 0.002385420, 0.002584870, 0.002782640, 0.002976620,
    0.003164520, 0.003343940, 0.003512320, 0.003667010, 0.003805260, 0.003924240, 0.004021090, 0.004092880,
    0.004136710, 0.004149690, 0.004128980, 0.004071820, 0.003975550, 0.003837640, 0.003655740, 0.003427670,
    0.003151450, 0.002825340, 0.002447870, 0.002017810, 0.001534240, 0.000996530, 0.000404350, -0.000242310,
    -0.000943140, -0.001697530, -0.002504530, -0.003362930, -0.004271180, -0.005227490, -0.006229770, -0.007275690,
    -0.008362720, -0.009488090, -0.010648860, -0.011841930, -0.013064050, -0.014311890, -0.015581980, -0.016870830,
    -0.018174860, -0.019490510, -0.020814160, -0.022142230, -0.023471130, -0.024797330, -0.026117280, -0.027427520,
    -0.028724570, -0.030005040, -0.031265510, -0.032502620, -0.033712980, -0.034893200, -0.036039880, -0.037149540,
    -0.038218680, -0.039243670, -0.040220790, -0.041146200, -0.042015890, -0.042825700, -0.043571260, -0.044248010,
    -0.044851180, -0.045375750, -0.045816480, -0.046167870, -0.046424210, -0.046579550, -0.046627690, -0.046562250,
    -0.046376650, -0.046064140, -0.045617860, -0.045030820, -0.044295990, -0.043406310, -0.042354750, -0.041134360,
    -0.039738340, -0.038160060, -0.036393160, -0.034431550, -0.032269560, -0.029901920, -0.027323850, -0.024531120,
    -0.021520120, -0.018287890, -0.014832160, -0.011151460, -0.007245080, -0.003113170, 0.001243280, 0.005822360,
    0.010621270, 0.015636270, 0.020862730, 0.026295040, 0.031926740, 0.037750430, 0.043757870, 0.049939950,
    0.056286810, 0.062787800, 0.069431590, 0.076206210, 0.083099140, 0.090097320, 0.097187300, 0.104355260,
    0.111587150, 0.118868700, 0.126185600, 0.133523510, 0.140868190, 0.148205610, 0.155521980, 0.162803890,
    0.170038410, 0.177213110, 0.184316200, 0.191336610, 0.198264010, 0.205088960, 0.211802890, 0.218398230,
    0.224868450, 0.231208060, 0.237412700, 0.243479190, 0.249405490, 0.255190790, 0.260835470, 0.266341160,
    0.271710670, 0.276948070, 0.282058570, 0.287048600, 0.291925710, 0.296698550, 0.301376840, 0.305971300,
    0.310493620, 0.314956360, 0.319372920, 0.323757410, 0.328124650, 0.332490010, 0.336869360, 0.341278980,
    0.345735450, 0.350255540, 0.354856130, 0.359554120, 0.364366270, 0.369309150, 0.374399020, 0.379651700,
    0.385082500, 0.390706090, 0.396536420, 0.402586620, 0.408868900, 0.415394460, 0.422173410, 0.429214700,
    0.436526030, 0.444113830, 0.451983110, 0.460137530, 0.468579250, 0.477308960, 0.486325850, 0.495627560,
    0.505210210, 0.515068400, 0.525195200, 0.535582200, 0.546219500, 0.557095820, 0.568198490, 0.579513510,
    0.591025680, 0.602718600, 0.614574780, 0.626575740, 0.638702100, 0.650933660, 0.663249510, 0.675628170,
    0.688047630, 0.700485530, 0.712919220, 0.725325900, 0.737682720, 0.749966880, 0.762155720, 0.774226870,
    0.786158280, 0.797928360, 0.809516020, 0.820900790, 0.832062870, 0.842983150, 0.853643350, 0.864025980,
    0.874114430, 0.883892960, 0.893346770, 0.902461950, 0.911225530, 0.919625470, 0.927650620, 0.935290730,
    0.942536420, 0.949379160, 0.955811220, 0.961825620, 0.967416160, 0.972577280, 0.977304100, 0.981592330,
    0.985438250, 0.988838640, 0.991790790, 0.994292410, 0.996341630, 0.997936960, 0.999077280, 0.999761780,
    0.999990000, 0.999761780, 0.999077280, 0.997936960, 0.996341630, 0.994292410, 0.991790790, 0.988838640,
    0.985438250, 0.981592330, 0.977304100, 0.972577280, 0.967416160, 0.961825620, 0.955811220, 0.949379160,
    0.942536420, 0.935290730, 0.927650620, 0.919625470, 0.911225530, 0.902461950, 0.893346770, 0.883892960,
    0.874114430, 0.864025980, 0.853643350, 0.842983150, 0.832062870, 0.820900790, 0.809516020, 0.797928360,
    0.786158280, 0.774226870, 0.762155720, 0.749966880, 0.737682720, 0.725325900, 0.712919220, 0.700485530,
    0.688047630, 0.675628170, 0.663249510, 0.650933660, 0.638702100, 0.626575740, 0.614574780, 0.602718600,
    0.591025680, 0.579513510, 0.568198490, 0.557095820, 0.546219500, 0.535582200, 0.525195200, 0.515068400,
    0.505210210, 0.495627560, 0.486325850, 0.477308960, 0.468579250, 0.460137530, 0.451983110, 0.444113830,
    0.436526030, 0.429214700, 0.422173410, 0.415394460, 0.408868900, 0.402586620, 0.396536420, 0.390706090,
    0.385082500, 0.379651700, 0.374399020, 0.369309150, 0.364366270, 0.359554120, 0.354856130, 0.350255540,
    0.345735450, 0.341278980, 0.336869360, 0.332490010, 0.328124650, 0.323757410, 0.319372920, 0.314956360,
    0.310493620, 0.305971300, 0.301376840, 0.296698550, 0.291925710, 0.287048600, 0.282058570, 0.276948070,
    0.271710670, 0.266341160, 0.260835470, 0.255190790, 0.249405490, 0.243479190, 0.237412700, 0.231208060,
    0.224868450, 0.218398230, 0.211802890, 0.205088960, 0.198264010, 0.191336610, 0.184316200, 0.177213110,
    0.170038410, 0.162803890, 0.155521980, 0.148205610, 0.140868190, 0.133523510, 0.126185600, 0.118868700,
    0.111587150, 0.104355260, 0.097187300, 0.090097320, 0.083099140, 0.076206210, 0.069431590, 0.062787800,
    0.056286810, 0.049939950, 0.043757870, 0.037750430, 0.031926740, 0.026295040, 0.020862730, 0.015636270,
    0.010621270, 0.005822360, 0.001243280, -0.003113170, -0.007245080, -0.011151460, -0.014832160, -0.018287890,
    -0.021520120, -0.024531120, -0.027323850, -0.029901920, -0.032269560, -0.034431550, -0.036393160, -0.038160060,
    -0.039738340, -0.041134360, -0.042354750, -0.043406310, -0.044295990, -0.045030820, -0.045617860, -0.046064140,
    -0.046376650, -0.046562250, -0.046627690, -0.046579550, -0.046424210, -0.046167870, -0.045816480, -0.045375750,
    -0.044851180, -0.044248010, -0.043571260, -0.042825700, -0.042015890, -0.041146200, -0.040220790, -0.039243670,
    -0.038218680, -0.037149540, -0.036039880, -0.034893200, -0.033712980, -0.032502620, -0.031265510, -0.030005040,
    -0.028724570, -0.027427520, -0.026117280, -0.024797330, -0.023471130, -0.022142230, -0.020814160, -0.019490510,
    -0.018174860, -0.016870830, -0.015581980, -0.014311890, -0.013064050, -0.011841930, -0.010648860, -0.009488090,
    -0.008362720, -0.007275690, -0.006229770, -0.005227490, -0.004271180, -0.003362930, -0.002504530, -0.001697530,
    -0.000943140, -0.000242310, 0.000404350, 0.000996530, 0.001534240, 0.002017810, 0.002447870, 0.002825340,
    0.003151450, 0.003427670, 0.003655740, 0.003837640, 0.003975550, 0.004071820, 0.004128980, 0.004149690,
    0.004136710, 0.004092880, 0.004021090, 0.003924240, 0.003805260, 0.003667010, 0.003512320, 0.003343940,
    0.003164520, 0.002976620, 0.002782640, 0.002584870, 0.002385420, 0.002186280, 0.001989260, 0.001795990,
    0.001607980, 0.001426530, 0.001252830, 0.001087890, 0.000932610, 0.000787730, 0.000653890, 0.000531620,
    0.000421340, 0.000323410, 0.000238100, 0.000165620, 0.000106130, 0.000059750, 0.000026570, 0.000006650,
];

/// fldigi's short interleave pattern: four carriers at a time, sixteen times.
const SHORT_PATTERN: [u8; CARRIERS] = [4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7, 4, 5, 6, 7];
/// fldigi's long interleave pattern: one carrier at a time.
const LONG_PATTERN: [u8; CARRIERS] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 0,
];

/// One MT63 mode: bandwidth in Hz and whether the long interleave is in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mt63Variant {
    pub bandwidth: u32,
    pub long: bool,
}

impl Mt63Variant {
    /// The mode names fldigi uses: `MT63-500S`, `MT63-1KL`, …
    pub fn named(name: &str) -> Option<Self> {
        let n = name.to_ascii_uppercase();
        let bandwidth = if n.contains("500") {
            500
        } else if n.contains("2K") || n.contains("2000") {
            2000
        } else if n.contains("1K") || n.contains("1000") {
            1000
        } else {
            return None;
        };
        let long = n.contains("L");
        Some(Mt63Variant { bandwidth, long })
    }

    pub fn label(&self) -> String {
        let bw = match self.bandwidth {
            500 => "500",
            2000 => "2K",
            _ => "1K",
        };
        format!("MT63-{bw}{}", if self.long { "L" } else { "S" })
    }

    /// The processing rate: twice the bandwidth, so a carrier is four bins of a
    /// 512-point FFT from its neighbour and the bank is exactly `bandwidth`.
    pub fn rate(&self) -> f64 {
        2.0 * self.bandwidth as f64
    }
}

/// fldigi's Walsh transform, butterflies included — the second one is
/// `b2 - b1`, not `b1 - b2`, and that sign is part of the code's meaning.
fn walsh_trans(d: &mut [f64]) {
    let len = d.len();
    let mut step = 1;
    while step < len {
        let mut ptr = 0;
        while ptr < len {
            for k in 0..step {
                let a = d[ptr + k];
                let b = d[ptr + k + step];
                d[ptr + k] = a + b;
                d[ptr + k + step] = b - a;
            }
            ptr += 2 * step;
        }
        step *= 2;
    }
}

/// The inverse, as `dspWalshInvTrans` writes it.
pub fn walsh_inv(d: &mut [f64]) {
    let len = d.len();
    let mut step = len / 2;
    while step > 0 {
        let mut ptr = 0;
        while ptr < len {
            for k in 0..step {
                let a = d[ptr + k];
                let b = d[ptr + k + step];
                d[ptr + k] = a - b;
                d[ptr + k + step] = a + b;
            }
            ptr += 2 * step;
        }
        step /= 2;
    }
}

/// The character encoder and block interleaver, ported for the loopback and so
/// that the sign conventions can be settled without the air.
pub struct Mt63Encoder {
    long: bool,
    pipe: Vec<u8>,
    ptr: usize,
    pat: [usize; CARRIERS],
    size: usize,
}

impl Mt63Encoder {
    pub fn new(long: bool) -> Self {
        let pattern = if long { &LONG_PATTERN } else { &SHORT_PATTERN };
        let len = if long { 64 } else { 32 };
        let mut pat = [0usize; CARRIERS];
        let mut p = 0usize;
        for (i, &step) in pattern.iter().enumerate() {
            pat[i] = p * CARRIERS;
            p += step as usize;
            if p >= len {
                p -= len;
            }
        }
        Mt63Encoder { long, pipe: vec![0u8; len * CARRIERS], ptr: 0, pat, size: len * CARRIERS }
    }

    /// One character in, 64 differential bits out (`true` = no phase flip).
    pub fn process(&mut self, ch: u8) -> [u8; CARRIERS] {
        let code = (ch & 127) as usize;
        let mut w = [0.0f64; CARRIERS];
        if code < CARRIERS {
            w[code] = 1.0;
        } else {
            w[code - CARRIERS] = -1.0;
        }
        walsh_inv(&mut w);
        let mut out = [0u8; CARRIERS];
        let len = self.size / CARRIERS;
        for i in 0..CARRIERS {
            self.pipe[self.ptr + i] = (w[i] < 0.0) as u8;
        }
        for i in 0..CARRIERS {
            let mut k = self.ptr + self.pat[i];
            if k >= self.size {
                k -= self.size;
            }
            out[i] = self.pipe[k + i];
        }
        self.ptr += CARRIERS;
        if self.ptr >= self.size {
            self.ptr -= self.size;
        }
        let _ = (len, self.long);
        out
    }
}

/// The bank, the deinterleaver and the Walsh decoder.
///
/// One `Mt63Rx` is tuned to a centre and a mode. `decode` walks the recording at
/// a symbol offset and returns the text and the mean decode SNR, so a caller can
/// search the offset the way fldigi's synchroniser does.
pub struct Mt63Rx {
    variant: Mt63Variant,
    centre: f64,
    /// FFT length and symbol hop at the incoming rate.
    window_len: usize,
    hop: usize,
    shape: Vec<f32>,
    sample_rate: f64,
    /// Per-scan-position complex value of the last symbol, for the differential.
    prev_a: Vec<Complex32>,
    prev_b: Vec<Complex32>,
    /// Deinterleaver pipe, `(InterleaveLen + 1) * SCAN`.
    pipe: Vec<f32>,
    pipe_ptr: usize,
    pat: [usize; CARRIERS],
    size: usize,
    decode_pipe: Vec<u8>,
    decode_ptr: usize,
    decode_len: usize,
    snr_out: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl Mt63Rx {
    pub fn new(rate: f64, centre: f64, variant: Mt63Variant) -> Self {
        let len = if variant.long { 64 } else { 32 };
        let pattern = if variant.long { &LONG_PATTERN } else { &SHORT_PATTERN };
        let mut pat = [0usize; CARRIERS];
        let mut p = 0usize;
        for (i, &step) in pattern.iter().enumerate() {
            pat[i] = p * SCAN;
            p += step as usize;
            if p >= len {
                p -= len;
            }
        }
        let size = (len + 1) * SCAN;
        let decode_len = 8;
        // Everything at the reference's rate is scaled by this, so a carrier is
        // still four bins and one symbol still `SYMBOL` hops at 2·bandwidth.
        let scale = rate / variant.rate();
        let window_len = ((WINDOW as f64 * scale).round() as usize).max(WINDOW);
        let hop = ((SYMBOL as f64 * scale).round() as usize).max(SYMBOL);
        let shape: Vec<f32> = (0..window_len)
            .map(|i| {
                let x = i as f64 * WINDOW as f64 / window_len as f64;
                let j = x.floor() as usize;
                let f = (x - j as f64) as f32;
                let a = SYMBOL_SHAPE[j.min(WINDOW - 1)];
                let b = SYMBOL_SHAPE[(j + 1).min(WINDOW - 1)];
                a + (b - a) * f
            })
            .collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(window_len);
        Mt63Rx {
            variant,
            centre,
            window_len,
            hop,
            shape,
            sample_rate: rate,
            prev_a: vec![Complex32::new(0.0, 0.0); SCAN],
            prev_b: vec![Complex32::new(0.0, 0.0); SCAN],
            pipe: vec![0.0; size],
            pipe_ptr: 0,
            pat,
            size,
            decode_pipe: vec![0u8; decode_len * SCAN_LEN],
            decode_ptr: 0,
            decode_len,
            snr_out: vec![0.0; SCAN_LEN],
            fft,
        }
    }

    pub fn variant(&self) -> Mt63Variant {
        self.variant
    }

    /// The FFT bin of the first data carrier: `round((centre - bw/2) * 512/R)`,
    /// where `R = 2·bw`, exactly as `MT63rx::Preset` computes it.
    fn first_carrier(&self) -> i32 {
        ((self.centre - self.variant.bandwidth as f64 / 2.0) * self.window_len as f64
            / self.sample_rate
            + 0.5)
            .floor() as i32
    }

    /// Decode one recording at a given symbol phase, returning the text and the
    /// mean decoded SNR. `phase` is in samples at the processing rate.
    pub fn decode(&mut self, audio: &[f32], phase: usize) -> (String, f32) {
        let sig = audio;
        self.reset();
        let first = self.first_carrier();
        let mut text = String::new();
        let mut snr_sum = 0.0f32;
        let mut n = 0usize;
        let mut t = phase;
        while t + self.window_len + self.hop / 2 < sig.len() {
            let a = self.slice(&sig[t..t + self.window_len]);
            let b = self.slice(&sig[t + self.hop / 2..t + self.hop / 2 + self.window_len]);
            let mut vals = [0.0f32; SCAN];
            for j in 0..SCAN {
                let carrier = j as i32 - MARGIN as i32;
                let c = first + carrier * CARR_SEP as i32;
                if c < 0 || c as usize >= self.window_len {
                    vals[j] = 0.0;
                    continue;
                }
                let c = c as usize;
                // Odd carriers are the second slice, half a symbol later.
                let (cur, prev) =
                    if carrier & 1 == 0 { (a[c], self.prev_a[j]) } else { (b[c], self.prev_b[j]) };
                if carrier & 1 == 0 {
                    self.prev_a[j] = a[c];
                } else {
                    self.prev_b[j] = b[c];
                }
                // The window advanced by SYMBOL samples, so a carrier at bin c
                // has turned by 2πc·SYMBOL/WINDOW. Removing that turn leaves the
                // data's own phase step.
                let turn = -2.0 * std::f32::consts::PI * c as f32 * self.hop as f32
                    / self.window_len as f32;
                let d = cur * prev.conj() * Complex32::new(turn.cos(), turn.sin());
                let norm = (cur.norm() * prev.norm()).max(1e-12);
                vals[j] = (d.re / norm).clamp(-1.0, 1.0);
            }
            let (code, snr) = self.deinterleave(&vals);
            text.push(code as char);
            snr_sum += snr;
            n += 1;
            t += self.hop;
        }
        if n > 0 { snr_sum /= n as f32; }
        (text, snr_sum)
    }

    fn reset(&mut self) {
        self.prev_a.fill(Complex32::new(0.0, 0.0));
        self.prev_b.fill(Complex32::new(0.0, 0.0));
        self.pipe.fill(0.0);
        self.pipe_ptr = 0;
        self.decode_pipe.fill(0);
        self.decode_ptr = 0;
        self.snr_out.fill(0.0);
    }

    fn slice(&self, samples: &[f32]) -> Vec<Complex32> {
        let mut buf: Vec<Complex32> = samples
            .iter()
            .zip(self.shape.iter())
            .map(|(&s, &w)| Complex32::new(s * w, 0.0))
            .collect();
        self.fft.process(&mut buf);
        buf
    }

    /// `MT63decoder::Process`: deinterleave, Walsh-transform, and pick the time
    /// offset whose decode is strongest.
    fn deinterleave(&mut self, data: &[f32; SCAN]) -> (u8, f32) {
        self.pipe[self.pipe_ptr..self.pipe_ptr + SCAN].copy_from_slice(data);
        let mut best_code = 0u8;
        let mut best_snr = 0.0f32;
        for s in 0..SCAN_LEN {
            let mut w = [0.0f64; CARRIERS];
            for i in 0..CARRIERS {
                let mut k = self.pipe_ptr as isize - SCAN as isize - self.pat[i] as isize;
                if (s & 1) != 0 && (i & 1) != 0 {
                    k += SCAN as isize;
                }
                let mut idx = k + s as isize + i as isize;
                idx = idx.rem_euclid(self.size as isize);
                w[i] = self.pipe[idx as usize] as f64;
            }
            walsh_trans(&mut w);
            let mut max = 0.0f64;
            let mut min = 0.0f64;
            let mut max_pos = 0;
            let mut min_pos = 0;
            for (i, &v) in w.iter().enumerate() {
                if v > max {
                    max = v;
                    max_pos = i;
                }
                if v < min {
                    min = v;
                    min_pos = i;
                }
            }
            let (code, sig) = if max.abs() > min.abs() {
                (max_pos + CARRIERS, max.abs())
            } else {
                (min_pos, min.abs())
            };
            w[if max.abs() > min.abs() { max_pos } else { min_pos }] = 0.0;
            let noise = (w.iter().map(|v| v * v).sum::<f64>() / CARRIERS as f64).sqrt();
            let snr = if noise > 0.0 { (sig / noise) as f32 } else { 0.0 };
            self.snr_out[s] = snr;
            self.decode_pipe[self.decode_ptr + s] = code as u8;
            if self.snr_out[s] > best_snr {
                best_snr = self.snr_out[s];
                best_code = code as u8;
            }
        }
        self.pipe_ptr += SCAN;
        if self.pipe_ptr >= self.size {
            self.pipe_ptr = 0;
        }
        self.decode_ptr += SCAN_LEN;
        if self.decode_ptr >= self.decode_len * SCAN_LEN {
            self.decode_ptr = 0;
        }
        (best_code, best_snr)
    }
}

/// Decode a recording, searching the symbol phase the way the receiver's
/// synchroniser would. Returns the best text and its mean SNR.
pub fn decode(audio: &[f32], rate: f64, centre: f64, variant: Mt63Variant) -> (String, f32) {
    let mut best = (String::new(), 0.0f32);
    let hop = (SYMBOL as f64 * rate / variant.rate()).round() as usize;
    let step = (hop / 80).max(1);
    for phase in (0..hop).step_by(step) {
        let mut rx = Mt63Rx::new(rate, centre, variant);
        let (text, snr) = rx.decode(audio, phase);
        // Mean SNR alone can prefer a perfectly-timed slice of the wrong
        // thing; a decode that produces letters is the better answer.
        let score = textiness(&text) as f32 * 10.0 + snr;
        if score > best.1 {
            best = (text, score);
        }
    }
    best
}

/// How much of a decode looks like text: runs of ASCII letters and digits, with
/// spaces, win; control characters and long punctuation runs do not.
fn textiness(text: &str) -> usize {
    let mut score = 0usize;
    let mut run = 0usize;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            run += 1;
        } else {
            if run >= 3 {
                score += run;
            }
            run = 0;
            if c == ' ' {
                score += 1;
            }
        }
    }
    score + if run >= 3 { run } else { 0 }
}

/// The printable part of a decode, for comparing against a known text.
pub fn printable(text: &str) -> String {
    text.chars()
        .filter(|&c| c == '\n' || (' '..='~').contains(&c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The inverse and forward transforms as fldigi writes them are a pair up
    /// to the scale and sign that the decoder never depends on; what matters is
    /// that a codeword's peak is at its own index and that its polarity survives.
    #[test]
    fn the_walsh_pair_round_trips() {
        for code in [0usize, 1, 7, 31, 63] {
            let mut w = [0.0f64; CARRIERS];
            w[code] = 1.0;
            walsh_inv(&mut w);
            walsh_trans(&mut w);
            let peak = (0..CARRIERS).max_by(|&a, &b| w[a].abs().total_cmp(&w[b].abs())).unwrap();
            assert_eq!(peak, code, "codeword {code} peaked at {peak}");
        }
    }

    #[test]
    fn the_encoder_makes_one_bit_per_carrier() {
        let mut enc = Mt63Encoder::new(false);
        let out = enc.process(b'A');
        assert_eq!(out.len(), CARRIERS);
        assert!(out.iter().any(|&b| b == 1) && out.iter().any(|&b| b == 0));
    }

    /// The corpus gate: each MT63 recording must read its own text. Centres are
    /// the measured plateau centre (all six are 1739–1742 Hz), and the text is
    /// `samples/README.md`'s. The phase is searched because the fixture has no
    /// sync preamble this decoder can use.
    #[test]
    #[ignore]
    fn the_corpus_mt63_samples_decode() {
        use std::path::Path;
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi");
        let table = [
            ("MT63-500S", "EAGLE LANDED SDRBROWN"),
            ("MT63-1KS", "EAGLE LANDED SDRBROWN"),
            ("MT63-2KS", "EAGLE LANDED SDRBROWN"),
            ("MT63-500L", "SDRBROWN TEST"),
            ("MT63-1KL", "SDRBROWN TEST"),
            ("MT63-2KL", "SDRBROWN TEST"),
        ];
        let mut failures = Vec::new();
        for (name, want) in table {
            let path = format!("{root}/{name}.wav");
            if !Path::new(&path).exists() {
                eprintln!("{name}: no sample");
                continue;
            }
            let mut reader = hound::WavReader::open(&path).expect("open");
            let spec = reader.spec();
            let audio: Vec<f32> =
                reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect();
            let variant = Mt63Variant::named(name).expect("a variant");
            let hz: f64 = std::env::var("MT63_HZ").ok().and_then(|v| v.parse().ok()).unwrap_or(1740.0);
            let (text, snr) = decode(&audio, spec.sample_rate as f64, hz, variant);
            let p = printable(&text);
            let hits = want.split_whitespace().filter(|w| p.contains(w)).count();
            eprintln!("{name}: {hits}/{} words, snr {snr:.1}: {:?}", want.split_whitespace().count(), p);
            if hits != want.split_whitespace().count() {
                failures.push(name);
            }
        }
        assert!(failures.is_empty(), "did not read: {failures:?}");
    }

    /// The interleaver and the Walsh decode, without audio: the encoder's bits
    /// as ±1 must come back as the character, through the same deinterleaver the
    /// receiver uses. A failure here is a table or pointer, not a demodulator.
    #[test]
    fn the_interleaver_round_trips_a_message() {
        for long in [false, true] {
            let mut enc = Mt63Encoder::new(long);
            let mut rx = Mt63Rx::new(8000.0, 1500.0, Mt63Variant { bandwidth: 1000, long });
            rx.reset();
            let text: Vec<u8> = b"THE EAGLE HAS LANDED 12345".to_vec();
            let mut decoded = Vec::new();
            for &ch in &text {
                let bits = enc.process(ch);
                let vals: [f32; SCAN] = std::array::from_fn(|j| {
                    if (MARGIN..MARGIN + CARRIERS).contains(&j) {
                        if bits[j - MARGIN] == 1 { 1.0 } else { -1.0 }
                    } else {
                        0.0
                    }
                });
                let (code, _) = rx.deinterleave(&vals);
                decoded.push(code);
            }
            let tail: String = decoded.iter().map(|&c| c as char).collect();
            eprintln!("long {long}: {tail:?}");
            assert!(tail.contains("THE EAGLE"), "long {long}: {tail:?}");
        }
    }

    #[test]
    fn the_variant_names_are_fldigis() {
        assert_eq!(
            Mt63Variant::named("MT63-1KS"),
            Some(Mt63Variant { bandwidth: 1000, long: false })
        );
        assert_eq!(
            Mt63Variant::named("MT63-2KL"),
            Some(Mt63Variant { bandwidth: 2000, long: true })
        );
        assert_eq!(Mt63Variant::named("MT63-500S").map(|v| v.bandwidth), Some(500));
    }
}
