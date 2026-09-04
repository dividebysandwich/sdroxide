//! Decimators: fast half-band /2 stages plus a generic windowed-sinc
//! FIR decimator for the residual integer factor.

use std::f64::consts::PI;

use crate::Complex32;

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 { 1.0 } else { (PI * x).sin() / (PI * x) }
}

fn blackman_harris_f64(n: usize, i: usize) -> f64 {
    const A: [f64; 4] = [0.35875, 0.48829, 0.14128, 0.01168];
    let x = std::f64::consts::TAU * i as f64 / (n as f64 - 1.0);
    A[0] - A[1] * x.cos() + A[2] * (2.0 * x).cos() - A[3] * (3.0 * x).cos()
}

/// Windowed-sinc lowpass, DC gain 1. `cutoff` is normalized to the input
/// sample rate (0.5 = Nyquist).
pub fn lowpass_taps(ntaps: usize, cutoff: f64) -> Vec<f32> {
    let center = (ntaps - 1) as f64 / 2.0;
    let mut taps: Vec<f64> = (0..ntaps)
        .map(|i| {
            2.0 * cutoff * sinc(2.0 * cutoff * (i as f64 - center)) * blackman_harris_f64(ntaps, i)
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    taps.iter_mut().for_each(|t| *t /= sum);
    taps.into_iter().map(|t| t as f32).collect()
}

/// Taps in the half-band prototype. Fixed, because the filter is: a cutoff of
/// exactly 0.25 is what zeroes every other tap, and 23 is where the stopband is
/// deep enough for a /2 stage without the dot product growing.
const HB_TAPS: usize = 23;

/// Non-zero taps below the centre one, and so the number of symmetric pairs the
/// dot product below is written out as. `(HB_TAPS / 2 + 1) / 2` for a 23-tap
/// prototype: indices 0, 2, 4, 6, 8, 10.
const HB_PAIRS: usize = 6;

/// Half-band decimator (factor 2), in polyphase form.
///
/// This is the busiest filter in the tree — a receive chain on a wide front end
/// runs a ladder of these at the device rate, 16.2 Msps on the RX-888 geometry
/// these numbers were taken at, and it is the largest single symbol in a
/// profile of the running receiver. Three properties of the prototype are spent
/// here rather than left on the table:
///
/// * Every second tap is zero (that is what a half-band *is*), so half the
///   products never existed.
/// * The rest are symmetric about the centre, so a pair of samples can share
///   one multiply: `x[a]·t + x[b]·t` is `(x[a] + x[b])·t`. That is a complex
///   add standing in for a complex multiply — seven multiplies where the
///   straightforward loop does thirteen.
/// * The surviving taps are all at *even* offsets, save the lone centre one at
///   an odd offset. So the filter never mixes the two phases of its input:
///   split `x` into `even[m] = x[2m]` and `odd[m] = x[2m+1]` and one output is
///
///   ```text
///   y[n] = h₁₁·odd[n+5] + Σⱼ tⱼ·(even[n+j] + even[n+11−j])
///   ```
///
///   which reads both arrays at unit stride.
///
/// That last point is the whole reason for the split, and it is worth being
/// plain about what it buys. The direct form indexes its window with a stride
/// of two, and a stride the compiler cannot flatten means one output at a time
/// however wide the registers are: measured against `-C target-cpu=native`, the
/// direct form went from 779 to 974 Msps while this one went from 893 to 1816.
/// Under the AVX2 copy this crate actually dispatches (see [`crate::simd`]) it
/// is 1.78× the direct form.
///
/// The split costs nothing extra: the direct form copied every input sample
/// into a history buffer anyway, and this spends the same copy putting the two
/// phases in separate arrays.
pub struct HalfbandDecim {
    /// The six surviving tap values, `taps[j]` being the prototype's tap `2j`
    /// and its mirror. Built from the prototype rather than written down, so
    /// the numbers stay derived.
    taps: [f32; HB_PAIRS],
    /// The centre tap, which has no partner and sits on the odd phase.
    center: f32,
    /// Input split by phase, `even[m] = x[2m]` and `odd[m] = x[2m+1]`.
    even: Vec<Complex32>,
    odd: Vec<Complex32>,
    /// Set when a block ended on an even sample, so the next one opens with
    /// that pair's odd half. Losing this is what would slide the decimation
    /// phase by one sample on any block of odd length.
    odd_next: bool,
}

crate::simd::kernel! {
    /// Split a block into its two sample phases.
    fn split_phases / split_phases_portable / split_phases_avx2 / split_phases_avx512 (
        even: &mut [Complex32],
        odd: &mut [Complex32],
        src: &[Complex32],
    ) {
        for ((e, o), p) in even.iter_mut().zip(odd).zip(src.chunks_exact(2)) {
            *e = p[0];
            *o = p[1];
        }
    }
}

crate::simd::kernel! {
    /// One half-band stage's dot products, one per element of `dst`.
    ///
    /// Written against a `&mut [Complex32]` rather than pushing into the
    /// caller's `Vec` so the loop has a length the compiler knows before it
    /// starts, and against slices cut to exactly what it reads so the bounds
    /// checks hoist out. Both are what let the AVX2 copy compute four outputs
    /// at a time.
    fn halfband / halfband_portable / halfband_avx2 / halfband_avx512 (
        dst: &mut [Complex32],
        even: &[Complex32],
        odd: &[Complex32],
        taps: &[f32; HB_PAIRS],
        center: f32,
    ) {
        let t = *taps;
        for (k, d) in dst.iter_mut().enumerate() {
            // Two chains of three rather than six terms in sequence: summed in
            // one order that is a dependency chain the length of the filter.
            let a = (even[k] + even[k + 11]) * t[0]
                + (even[k + 1] + even[k + 10]) * t[1]
                + (even[k + 2] + even[k + 9]) * t[2];
            let b = (even[k + 3] + even[k + 8]) * t[3]
                + (even[k + 4] + even[k + 7]) * t[4]
                + (even[k + 5] + even[k + 6]) * t[5];
            *d = odd[k + 5] * center + (a + b);
        }
    }
}

/// Even-phase samples one output reads: the six pairs span `even[k ..= k+11]`.
const HB_SPAN: usize = 2 * HB_PAIRS;

impl HalfbandDecim {
    pub fn new() -> Self {
        let proto = lowpass_taps(HB_TAPS, 0.25);
        let center = HB_TAPS / 2;
        let mut taps = [0.0f32; HB_PAIRS];
        let mut found = 0;
        // The zeros are what the cutoff put there; recognising them rather than
        // assuming their positions keeps this honest if the prototype is ever
        // re-cut. The polyphase form needs more than their count, though — it
        // needs them at the even offsets, which is what makes the surviving
        // taps one phase and the centre the other.
        for (k, &tap) in proto.iter().enumerate().take(center).filter(|(_, t)| t.abs() > 1e-12) {
            assert_eq!(k, 2 * found, "the half-band prototype's zeros moved off the odd taps");
            taps[found] = tap;
            found += 1;
        }
        assert_eq!(found, HB_PAIRS, "the half-band prototype changed shape");
        assert_eq!(center % 2, 1, "the centre tap must sit on the odd phase");
        HalfbandDecim {
            taps,
            center: proto[center],
            even: Vec::new(),
            odd: Vec::new(),
            odd_next: false,
        }
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        let mut rest = input;
        // A block that ended mid-pair: its partner opens this one.
        if self.odd_next
            && let Some((first, tail)) = rest.split_first()
        {
            self.odd.push(*first);
            rest = tail;
            self.odd_next = false;
        }
        let pairs = rest.len() / 2;
        let (e0, o0) = (self.even.len(), self.odd.len());
        self.even.resize(e0 + pairs, Complex32::default());
        self.odd.resize(o0 + pairs, Complex32::default());
        split_phases(&mut self.even[e0..], &mut self.odd[o0..], &rest[..pairs * 2]);
        if let Some(&last) = rest.get(pairs * 2) {
            self.even.push(last);
            self.odd_next = true;
        }

        // One output per even-phase sample that has a whole window behind it.
        // `odd` is the shorter of the two whenever a pair is half-arrived, so
        // it is what bounds the count.
        let m = self.odd.len();
        if m < HB_SPAN {
            return;
        }
        let count = m - (HB_SPAN - 1);
        let start = out.len();
        out.resize(start + count, Complex32::default());
        halfband(
            &mut out[start..],
            &self.even[..count + HB_SPAN - 1],
            &self.odd[..count + HB_PAIRS - 1],
            &self.taps,
            self.center,
        );
        self.even.drain(..count);
        self.odd.drain(..count);
    }
}

/// Cascade of half-band stages: decimation by any power of two, keeping the
/// span centred where it already was.
///
/// This is the front-end decimator the operator sets from the RX box, not a
/// downconverter — there is no NCO, so DC in is DC out and the hardware LO
/// stays the middle of the span. What it buys is everything downstream running
/// at a fraction of the device rate.
///
/// Half-band stages are what make a deep factor affordable: each costs about a
/// dozen multiply-accumulates per *output* sample, and every stage halves the
/// rate the next one is fed at, so the whole cascade costs less than twice its
/// first stage no matter how far down it goes.
pub struct Decimator {
    stages: Vec<HalfbandDecim>,
    tmp_a: Vec<Complex32>,
    tmp_b: Vec<Complex32>,
}

impl Decimator {
    /// `factor` is rounded *down* to a power of two; 1 (or 0) builds a
    /// pass-through, so a caller does not have to special-case "off".
    pub fn new(factor: u32) -> Self {
        let stages = (0..factor.max(1).ilog2()).map(|_| HalfbandDecim::new()).collect();
        Decimator { stages, tmp_a: Vec::new(), tmp_b: Vec::new() }
    }

    /// The factor actually built — `1 << stages`, which is [`Decimator::new`]'s
    /// argument rounded down to a power of two.
    pub fn factor(&self) -> u32 {
        1 << self.stages.len()
    }

    /// Appends the decimated samples to `out`.
    ///
    /// Each stage keeps its own tail of unconsumed samples, so a block whose
    /// length is not a multiple of the factor is carried over rather than
    /// dropped — the output is continuous across calls.
    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        let Some((last, rest)) = self.stages.split_last_mut() else {
            out.extend_from_slice(input);
            return;
        };
        // The first stage reads the caller's buffer and the last writes into
        // `out`, so only the middle ones ever touch the scratch pair — a /2 and
        // a /4 copy nothing at all.
        let mut middle = rest.iter_mut();
        let Some(first) = middle.next() else {
            last.process(input, out);
            return;
        };
        self.tmp_a.clear();
        first.process(input, &mut self.tmp_a);
        for hb in middle {
            self.tmp_b.clear();
            hb.process(&self.tmp_a, &mut self.tmp_b);
            std::mem::swap(&mut self.tmp_a, &mut self.tmp_b);
        }
        last.process(&self.tmp_a, out);
    }
}

/// Independent running sums a filter window is walked with.
///
/// A dot product written as one accumulator is a chain: every
/// multiply-accumulate waits on the one before it, so a filter costs its tap
/// count times the *latency* of an add and the machine's other pipelines stand
/// idle. Splitting the window across several accumulators that are summed at
/// the end computes exactly the same thing with the chain cut into that many
/// independent pieces.
///
/// Eight is measured, not assumed: on the 97-tap /8 stage a zoom lane builds at
/// 32.4 Msps, two accumulators buy 1.1×, four 1.3×, eight 1.7×, and twelve
/// falls off a cliff (more live values than there are registers to hold them).
const FIR_ACCUMULATORS: usize = 8;

/// `window · taps`, walked with [`FIR_ACCUMULATORS`] independent sums.
///
/// `inline(always)`, not `inline`: this is called from inside a dispatched
/// kernel, and a copy left standing on its own would be compiled once for the
/// baseline and then *called* by the AVX2 copy — which is how the whole dot
/// product silently stayed on 128-bit registers while the loop around it went
/// wide. It showed up in a profile as a `decim::dot` with no `_avx2` suffix.
#[inline(always)]
fn dot(window: &[Complex32], taps: &[f32]) -> Complex32 {
    let mut acc = [Complex32::default(); FIR_ACCUMULATORS];
    let mut w = window.chunks_exact(FIR_ACCUMULATORS);
    let mut t = taps.chunks_exact(FIR_ACCUMULATORS);
    for (xs, ts) in (&mut w).zip(&mut t) {
        for i in 0..FIR_ACCUMULATORS {
            acc[i] += xs[i] * ts[i];
        }
    }
    let mut sum = Complex32::default();
    for a in acc {
        sum += a;
    }
    // The taps are an odd count by construction, so there is always a tail.
    for (x, &tap) in w.remainder().iter().zip(t.remainder()) {
        sum += x * tap;
    }
    sum
}

/// Generic FIR decimator by an integer factor.
pub struct FirDecim {
    taps: Vec<f32>,
    factor: usize,
    buf: Vec<Complex32>,
}

crate::simd::kernel! {
    /// [`dot`] once per kept output — see [`FirDecim`].
    fn fir_decim / fir_decim_portable / fir_decim_avx2 / fir_decim_avx512 (
        dst: &mut [Complex32],
        buf: &[Complex32],
        taps: &[f32],
        factor: usize,
    ) {
        let n = taps.len();
        for (o, d) in dst.iter_mut().enumerate() {
            let base = o * factor;
            *d = dot(&buf[base..base + n], taps);
        }
    }
}

impl FirDecim {
    pub fn new(factor: usize) -> Self {
        assert!(factor >= 1);
        let ntaps = (12 * factor).clamp(24, 768) | 1; // odd
        let taps = lowpass_taps(ntaps, 0.45 / factor as f64);
        FirDecim { taps, factor, buf: Vec::new() }
    }

    /// The same dot product with taps the caller chose.
    ///
    /// [`FirDecim::new`] cuts at the widest frequency that will not alias at
    /// `factor`, which is what a *rate change* wants. A caller filtering to a
    /// signal narrower than the rate it is running at — a channel inside a
    /// stream ten times its width — wants a cutoff of its own, and `factor` 1
    /// makes this a plain band-limiting filter with no rate change at all.
    ///
    /// Real taps, unlike [`crate::ComplexFir`]'s: half the multiplies for a
    /// filter that is not shifting anything sideways.
    pub fn with_taps(taps: Vec<f32>, factor: usize) -> Self {
        assert!(factor >= 1 && taps.len() > factor);
        FirDecim { taps, factor, buf: Vec::new() }
    }

    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.buf.extend_from_slice(input);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = (self.buf.len() - n) / self.factor + 1;
        let start = out.len();
        out.resize(start + count, Complex32::default());
        fir_decim(&mut out[start..], &self.buf, &self.taps, self.factor);
        self.buf.drain(..count * self.factor);
    }
}

/// [`dot`] for a real-valued window — the audio counterpart. `inline(always)`
/// for the reason [`dot`] is.
#[inline(always)]
fn real_dot(window: &[f32], taps: &[f32]) -> f32 {
    let mut acc = [0.0f32; FIR_ACCUMULATORS];
    let mut w = window.chunks_exact(FIR_ACCUMULATORS);
    let mut t = taps.chunks_exact(FIR_ACCUMULATORS);
    for (xs, ts) in (&mut w).zip(&mut t) {
        for i in 0..FIR_ACCUMULATORS {
            acc[i] += xs[i] * ts[i];
        }
    }
    let mut sum = 0.0;
    for a in acc {
        sum += a;
    }
    for (x, &tap) in w.remainder().iter().zip(t.remainder()) {
        sum += x * tap;
    }
    sum
}

/// Real-valued decimating FIR: the counterpart to [`FirDecim`] for audio.
///
/// Only the kept outputs are computed, so a long anti-alias filter costs
/// `ntaps / factor` multiply-accumulates per output sample — which is what
/// makes the two 15 kHz low-passes of the WFM stereo decoder cheaper than the
/// single non-decimating one they replace.
///
/// The cutoff is given in Hz at the *input* rate; the caller is responsible for
/// keeping it below the output Nyquist.
pub struct RealFirDecim {
    taps: Vec<f32>,
    factor: usize,
    buf: Vec<f32>,
}

crate::simd::kernel! {
    /// [`fir_decim`]'s real-valued counterpart.
    fn real_fir_decim / real_fir_decim_portable / real_fir_decim_avx2 / real_fir_decim_avx512 (
        dst: &mut [f32],
        buf: &[f32],
        taps: &[f32],
        factor: usize,
    ) {
        let n = taps.len();
        for (o, d) in dst.iter_mut().enumerate() {
            let base = o * factor;
            *d = real_dot(&buf[base..base + n], taps);
        }
    }
}

impl RealFirDecim {
    pub fn new(ntaps: usize, cutoff_hz: f64, sample_rate: f64, factor: usize) -> Self {
        assert!(factor >= 1);
        RealFirDecim { taps: lowpass_taps(ntaps, cutoff_hz / sample_rate), factor, buf: Vec::new() }
    }

    /// Group delay in samples at the *output* rate.
    pub fn delay(&self) -> f64 {
        (self.taps.len() - 1) as f64 / 2.0 / self.factor as f64
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        let n = self.taps.len();
        if self.buf.len() < n {
            return;
        }
        let count = (self.buf.len() - n) / self.factor + 1;
        let start = out.len();
        out.resize(start + count, 0.0);
        real_fir_decim(&mut out[start..], &self.buf, &self.taps, self.factor);
        self.buf.drain(..count * self.factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// A unit-amplitude complex tone at `f` cycles per sample.
    fn tone(n: usize, f: f32) -> Vec<Complex32> {
        (0..n)
            .map(|i| {
                let p = TAU * f * i as f32;
                Complex32::new(p.cos(), p.sin())
            })
            .collect()
    }

    /// Level at `f` cycles per sample, in dB relative to a unit-amplitude tone.
    fn tone_db(x: &[Complex32], f: f32) -> f32 {
        let mut acc = Complex32::default();
        for (i, s) in x.iter().enumerate() {
            let p = -TAU * f * i as f32;
            acc += *s * Complex32::new(p.cos(), p.sin());
        }
        20.0 * (acc.norm() / x.len() as f32).max(1e-12).log10()
    }

    /// Decimate one long tone in blocks, the way the engine feeds it, and hand
    /// back the steady state (the first samples cover the filters' warm-up).
    fn decimate_tone(factor: u32, f: f32) -> Vec<Complex32> {
        let input = tone(1 << 17, f);
        let mut d = Decimator::new(factor);
        let mut out = Vec::new();
        for block in input.chunks(4096) {
            d.process(block, &mut out);
        }
        out.split_off(1024)
    }

    /// The polyphase stage must compute exactly what the textbook direct form
    /// does, whatever lengths the blocks arrive in.
    ///
    /// The reference is written from the definition — `y[n] = Σ h[k]·x[2n−k]`
    /// over the full 23-tap prototype, no zeros skipped and no symmetry
    /// exploited — so it cannot agree with the implementation by sharing its
    /// reasoning. The ragged block patterns are the other half: every output
    /// consumes exactly two input samples, and a block of odd length is where a
    /// decimator silently slips a phase and mirrors the spectrum from then on.
    #[test]
    fn the_polyphase_stage_is_the_direct_form_it_replaces() {
        let taps = lowpass_taps(HB_TAPS, 0.25);
        let x = tone(1 << 15, 0.037);
        // Direct form over the whole stream at once.
        let mut want = Vec::new();
        let mut o = 0;
        while o * 2 + HB_TAPS <= x.len() {
            let mut acc = Complex32::default();
            for (k, &h) in taps.iter().enumerate() {
                acc += x[o * 2 + k] * h;
            }
            want.push(acc);
            o += 1;
        }

        for pattern in
            [&[4096usize][..], &[1, 2, 3, 5, 7, 11, 13][..], &[23, 1, 46, 2, 1000, 3][..]]
        {
            let mut hb = HalfbandDecim::new();
            let mut got = Vec::new();
            let (mut at, mut i) = (0usize, 0usize);
            while at < x.len() {
                let n = pattern[i % pattern.len()].min(x.len() - at);
                hb.process(&x[at..at + n], &mut got);
                at += n;
                i += 1;
            }
            assert_eq!(got.len(), want.len(), "block pattern {pattern:?}: wrong output count");
            for (k, (g, w)) in got.iter().zip(&want).enumerate() {
                assert!((g - w).norm() < 1e-5, "block pattern {pattern:?}, sample {k}: {g} vs {w}");
            }
        }
    }

    #[test]
    fn a_decimator_rounds_its_factor_down_to_a_power_of_two() {
        assert_eq!(Decimator::new(0).factor(), 1);
        assert_eq!(Decimator::new(1).factor(), 1);
        assert_eq!(Decimator::new(6).factor(), 4);
        assert_eq!(Decimator::new(64).factor(), 64);
    }

    /// Factor 1 is the "decimation off" case the engine leaves in place, and it
    /// has to be a pass-through rather than a filter with a unit factor.
    #[test]
    fn a_pass_through_decimator_hands_the_block_straight_back() {
        let input = tone(64, 0.1);
        let mut out = Vec::new();
        Decimator::new(1).process(&input, &mut out);
        assert_eq!(out, input);
    }

    /// Nothing is lost across the block boundaries: every stage carries its own
    /// tail, so N input samples produce N/factor output samples give or take
    /// the filters' warm-up, however the input is chopped up.
    #[test]
    fn the_output_rate_is_the_input_rate_over_the_factor() {
        for factor in [2u32, 4, 8, 16, 32] {
            let input = tone(1 << 16, 0.001);
            let mut d = Decimator::new(factor);
            let mut out = Vec::new();
            for block in input.chunks(1000) {
                d.process(block, &mut out);
            }
            let want = input.len() / factor as usize;
            let slack = 22 * factor as usize; // 11 input samples of group delay per stage
            assert!(
                want - out.len() < slack,
                "factor {factor}: {} samples out, wanted ~{want}",
                out.len()
            );
        }
    }

    /// The point of the anti-alias filtering: a signal from the part of the span
    /// being thrown away must not reappear inside the part being kept. Unfiltered,
    /// the 0.2 tone would land at -0.4 in the output at full strength and read as
    /// a real signal on the panadapter.
    #[test]
    fn what_falls_outside_the_kept_span_does_not_fold_into_it() {
        // 0.01 of the input rate is 0.08 of the /8 output rate: comfortably
        // inside, and it has to come through at full amplitude.
        let kept = decimate_tone(8, 0.01);
        let db = tone_db(&kept, 0.08);
        assert!(db > -0.2, "kept tone lost {db} dB");

        // 0.2 of the input rate is outside the ±0.0625 that /8 keeps. It would
        // alias to 1.6 cycles/sample at the output rate, i.e. -0.4.
        let folded = decimate_tone(8, 0.2);
        let db = tone_db(&folded, -0.4);
        assert!(db < -70.0, "alias only {db} dB down");
    }
}
