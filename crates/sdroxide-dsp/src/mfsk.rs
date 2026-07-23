//! Shared MFSK / incremental-FSK building blocks used by the Olivia, THOR, and
//! FSQ keyboard modems: a phase-continuous tone generator, a direct-DFT tone-bank
//! detector with symbol-timing recovery, a fast Walsh–Hadamard transform (Olivia
//! coding), and Gray-code helpers.
//!
//! These are deliberately simple and allocation-light; the tone banks here are
//! small (≤64 tones) and the per-symbol windows short (≤256 samples), so a direct
//! DFT per tone is cheap and keeps the frequency grid arbitrary (FSQ's ~8.79 Hz
//! spacing does not divide the internal rate, unlike Olivia's).

/// Phase-continuous MFSK tone source. Successive `emit` calls keep phase, so tone
/// transitions are glitch-free (true continuous-phase FSK).
pub struct ToneGen {
    rate: f64,
    phase: f64,
}

impl ToneGen {
    pub fn new(rate: f64) -> Self {
        ToneGen { rate, phase: 0.0 }
    }

    /// Append `n` samples of a sine at `hz` (phase-continuous) scaled by `amp`.
    /// A raised-cosine ramp of `ramp` samples is applied at the very start/end of
    /// the whole transmission by the caller; within a stream tones just abut.
    pub fn emit(&mut self, hz: f64, n: usize, amp: f32, out: &mut Vec<f32>) {
        let inc = std::f64::consts::TAU * hz / self.rate;
        for _ in 0..n {
            out.push((self.phase.cos() as f32) * amp);
            self.phase += inc;
            if self.phase > std::f64::consts::TAU {
                self.phase -= std::f64::consts::TAU;
            }
        }
    }
}

const SUBPHASES: usize = 16;

/// MFSK symbol-timing recovery over a uniform tone bank. Feed audio; it returns
/// one magnitude vector (one entry per tone) per recovered symbol, sampled at the
/// tracked timing instant (exactly one per symbol period, so a downstream
/// differential/block decoder stays aligned).
pub struct MfskClock {
    rate: f64,
    sps: usize,
    hop: usize,
    base_hz: f64,
    spacing: f64,
    tones: usize,
    buf: Vec<f32>,
    buf_start: usize,
    next_hop: usize,
    sync: [f32; SUBPHASES],
    tphase: usize,
    peak: f32,
}

impl MfskClock {
    pub fn new(rate: f64, base_hz: f64, spacing: f64, tones: usize, sps: usize) -> Self {
        MfskClock {
            rate,
            sps,
            hop: (sps / SUBPHASES).max(1),
            base_hz,
            spacing,
            tones,
            buf: Vec::new(),
            buf_start: 0,
            next_hop: sps,
            sync: [0.0; SUBPHASES],
            tphase: 0,
            peak: 0.0,
        }
    }

    /// Smoothed peak tone magnitude (tuning/quality indicator).
    pub fn peak_mag(&self) -> f32 {
        self.peak
    }

    pub fn feed(&mut self, audio: &[f32]) -> Vec<Vec<f32>> {
        let mut syms = Vec::new();
        self.buf.extend_from_slice(audio);
        let n_total = self.buf_start + self.buf.len();
        while self.next_hop <= n_total {
            let t = self.next_hop;
            if t >= self.sps {
                let a = t - self.sps - self.buf_start;
                let b = t - self.buf_start;
                let mags =
                    tone_bank_mags(&self.buf[a..b], self.base_hz, self.spacing, self.tones, self.rate);
                let peak = mags.iter().copied().fold(0.0f32, f32::max);
                self.peak += 0.02 * (peak - self.peak);
                let phase = (t / self.hop) % SUBPHASES;
                for (i, s) in self.sync.iter_mut().enumerate() {
                    *s *= 0.995;
                    if i == phase {
                        *s += peak;
                    }
                }
                if phase == 0 {
                    self.tphase = self
                        .sync
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.total_cmp(b.1))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
                if phase == self.tphase {
                    syms.push(mags);
                }
            }
            self.next_hop += self.hop;
        }
        let keep = n_total.saturating_sub(self.sps);
        if keep > self.buf_start {
            self.buf.drain(0..keep - self.buf_start);
            self.buf_start = keep;
        }
        syms
    }
}

/// One complex DFT bin of `samples` at `hz` (Goertzel-equivalent, O(len)).
pub fn dft_bin(samples: &[f32], hz: f64, rate: f64) -> num_complex::Complex<f32> {
    let w = std::f64::consts::TAU * hz / rate;
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (n, &s) in samples.iter().enumerate() {
        let a = w * n as f64;
        re += s as f64 * a.cos();
        im -= s as f64 * a.sin();
    }
    num_complex::Complex::new(re as f32, im as f32)
}

/// Magnitudes of a uniform tone bank (`base + k*spacing`, k in 0..n_tones) over a
/// window of samples. Returned vector has one entry per tone.
pub fn tone_bank_mags(
    samples: &[f32],
    base_hz: f64,
    spacing_hz: f64,
    n_tones: usize,
    rate: f64,
) -> Vec<f32> {
    (0..n_tones)
        .map(|k| dft_bin(samples, base_hz + k as f64 * spacing_hz, rate).norm())
        .collect()
}

/// In-place fast Walsh–Hadamard transform (natural order). `data.len()` must be a
/// power of two. Olivia's (64,7) biorthogonal code decodes by taking the FWHT of
/// the 64 soft bits and reading off the largest-magnitude coefficient.
pub fn fwht(data: &mut [f32]) {
    let n = data.len();
    debug_assert!(n.is_power_of_two());
    let mut len = 1;
    while len < n {
        let mut i = 0;
        while i < n {
            for j in i..i + len {
                let a = data[j];
                let b = data[j + len];
                data[j] = a + b;
                data[j + len] = a - b;
            }
            i += len << 1;
        }
        len <<= 1;
    }
}

/// One row of the natural-order 64×64 Hadamard matrix as ±1, i.e. the codeword
/// for Walsh index `row`. `bit(k)` = parity of the AND of `row` and `k`.
pub fn hadamard_bit(row: usize, k: usize) -> i8 {
    if (row & k).count_ones() & 1 == 0 { 1 } else { -1 }
}

/// Binary-reflected Gray code and its inverse (used to map MFSK tone indices so
/// adjacent tones differ in one bit).
pub fn gray(v: u32) -> u32 {
    v ^ (v >> 1)
}

pub fn ungray(mut g: u32) -> u32 {
    let mut v = 0;
    while g != 0 {
        v ^= g;
        g >>= 1;
    }
    v
}
