use std::sync::Arc;

use rustfft::{Fft, FftPlanner};
use sdroxide_types::SpectrumFrame;

use crate::{
    Complex32,
    window::{blackman_harris, blackman_harris_derivative},
};

/// Overlapped windowed-FFT power spectrum with exponential averaging.
///
/// Feed raw IQ with [`process`](Self::process); it runs an FFT every
/// `fft_size / 2` samples (50 % overlap) and folds the result into a running
/// power average. Emit display frames at any rate with
/// [`make_frame`](Self::make_frame).
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    fft_size: usize,
    /// How many samples one transform actually looks at. Equal to `fft_size`
    /// unless a caller asked for less, in which case the rest of the transform
    /// is zeros — see [`SpectrumAnalyzer::with_window`].
    window_len: usize,
    hop: usize,
    window: Vec<f32>,
    /// Normalization so a full-scale coherent sine reads ~0 dBFS.
    coherent_gain: f32,

    pending: Vec<Complex32>,
    scratch: Vec<Complex32>,
    work: Vec<Complex32>,

    /// Averaged linear power per bin, natural FFT order.
    avg_power: Vec<f32>,
    /// [`Self::avg_power`] (or [`Self::hold`]) rotated into display order and
    /// DC-patched, rebuilt at the top of every [`Self::make_frame`].
    ///
    /// Kept rather than recomputed per bin because the rotation is `(i + half)
    /// % n` and `n` is not a constant, so that `%` is a real integer division —
    /// twenty-odd cycles, once per FFT bin, on every frame *and* every
    /// waterfall row. At a 65536-point transform with a frame at 79 Hz and rows
    /// at 112 Hz that was twelve million divisions a second, and `make_frame`
    /// was the largest single symbol in a profile of the running receiver.
    view: Vec<f32>,
    /// Peak of [`Self::avg_power`] since the last waterfall row was taken —
    /// empty while nobody is clocking rows.
    ///
    /// The waterfall's time resolution is bounded by how often rows are drawn,
    /// not by how often transforms land: a front end streaming megahertz runs
    /// hundreds of transforms a second and a screen draws at most ninety. With
    /// only the latest one kept, everything between two rows is discarded, and
    /// a CW dot shorter than a row interval can fall between two frames and
    /// never be seen at all. Holding the maximum instead means a row is
    /// "the strongest thing that happened in this slice of time", which is
    /// what a waterfall is *for*.
    hold: Vec<f32>,
    /// Average instead of peak: [`Self::hold`] sums the transforms since the
    /// last row and [`Self::hold_n`] counts them, and columns are pooled as a
    /// mean. See [`Self::set_smooth`].
    smooth: bool,
    hold_n: u32,
    /// Reassigned rows (Auger & Flandrin): [`Self::hold`] sums each
    /// transform's power moved to the frequency it is centred on, and is read
    /// as a mean like [`Self::smooth`]. See [`Self::set_reassign`].
    reassign: bool,
    /// A cell is moved only where its power stands this factor over the
    /// transform's median; below that it stays where it is.
    reassign_floor: f32,
    /// The share of its power a cell keeps when it is not moved: below one,
    /// the background is drawn darker than the lines. See
    /// [`Self::set_reassign_dim`].
    reassign_keep: f32,
    /// The window's derivative, per sample, for the second transform.
    dwindow: Vec<f32>,
    /// That second transform; empty until reassignment is first asked for.
    work_d: Vec<Complex32>,
    /// Scratch for the median, the same length as the transform.
    rank: Vec<f32>,
    /// Points per transform bin in a reassigned row, which [`Self::hold`]
    /// then holds in display order: a power of two up to 16, chosen at each
    /// row from how many columns one bin is drawn across, so a line can be as
    /// thin as a column where a bin is wider than one.
    ra_up: usize,
    /// Whether [`Self::make_frame`] pools from [`Self::hold`] instead of
    /// [`Self::avg_power`]. Set for the length of one row build — see
    /// [`Self::set_read_hold`].
    read_hold: bool,
    /// Whether the last [`Self::make_frame`] was a row build — see
    /// [`Self::took_row`].
    hold_read: bool,
    /// Whether the last [`Self::make_frame`] came out of an analyser that had
    /// not yet run a transform — see [`Self::drew_empty`].
    drew_empty_flag: bool,
    alpha: f32,
    primed: bool,
    peak_abs: f32,
    /// Whether [`Self::process`] scans every sample for the input peak.
    ///
    /// Off by default. It is a pass over the whole stream — on an RX-888 at
    /// 32.4 Msps, thirty-two million squared magnitudes a second — kept for one
    /// caller: the terminal waterfall prints a dBFS column beside each line.
    /// Nothing in the GUI or the server reads it, and a lane that nobody asks
    /// should not pay for it. See [`Self::take_peak_dbfs`].
    peak_track: bool,
    seq: u32,
    /// Hide the hardware DC/LO-leakage spike in emitted frames.
    dc_suppress: bool,
    /// Transforms folded into the average since this analyser was built.
    ///
    /// The rate this climbs at is the *real* update rate of anything drawn
    /// from it: emitted frames carry a fresh `seq` whether or not a new
    /// transform landed between them, so a lane can publish at 30 fps while
    /// showing the same numbers for a tenth of a second. Diagnostic only.
    transforms: u64,
}

/// `10·log10(p)` for a display column, without libm.
///
/// A power spectrum drawn as `u8` over the operator's floor-to-ceiling range
/// quantises to about a third of a dB per level, and `log10f` is a ~40-cycle
/// call the vectoriser has to break the loop for — one per column, on every
/// frame and every waterfall row. This reads the exponent off the float and
/// fits the mantissa with a degree-5 polynomial: worst case under 1e-4 dB over
/// `[1, 2)`, four thousand times finer than one level of the picture it draws,
/// and it inlines into a vector loop.
///
/// `p` must be positive and normal; every caller adds `1e-20` first, which is
/// eleven orders of magnitude above `f32`'s smallest normal.
#[inline(always)]
fn db10(p: f32) -> f32 {
    const C: [f32; 6] =
        [0.043_428_365, -0.404_862_3, 1.593_884_6, -3.492_466, 5.046_853, -2.786_805_6];
    let bits = p.to_bits();
    let exp = (((bits >> 23) & 0xff) as i32 - 127) as f32;
    // The mantissa on its own, as a float in `[1, 2)`.
    let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    let log2m =
        C[0].mul_add(m, C[1]).mul_add(m, C[2]).mul_add(m, C[3]).mul_add(m, C[4]).mul_add(m, C[5]);
    // 10·log10(x) = 10·log10(2)·log2(x).
    3.010_3 * (exp + log2m)
}

crate::simd::kernel! {
    /// [`pool_to_bins`] where the columns divide the transform exactly.
    /// `mean` pools the column as its mean power rather than its peak.
    ///
    /// The whole-window view — no zoom — is the common case and it is a
    /// fixed-stride reduction: every column is the same number of bins, taken
    /// from a slice that starts at zero. Saying so lets the compiler unroll the
    /// reduction and overlap the dB polynomial of one column with the maximum
    /// of the next, where the general form below re-derives two `f64` bounds
    /// and builds a fresh sub-slice for every column.
    fn pool_to_bins_uniform / pool_to_bins_uniform_portable / pool_to_bins_uniform_avx2 / pool_to_bins_uniform_avx512 (
        out: &mut [u8],
        view: &[f32],
        per: usize,
        db_floor: f32,
        scale: f32,
        mean: bool,
    ) {
        for (o, w) in out.iter_mut().zip(view.chunks_exact(per)) {
            let mut m = 0.0f32;
            if mean {
                for &p in w {
                    m += p;
                }
                m /= per as f32;
            } else {
                for &p in w {
                    m = m.max(p);
                }
            }
            *o = ((db10(m + 1e-20) - db_floor) * scale).clamp(0.0, 255.0) as u8;
        }
    }
}

crate::simd::kernel! {
    /// Pool the spectrum into display columns and map each to `u8`.
    ///
    /// One column is the strongest bin in its slice — a maximum, not a mean, so
    /// a carrier narrower than a column still reaches the screen at its own
    /// height. Fused with the dB conversion and the level mapping so the whole
    /// frame is one pass with no libm call in it: at a 65536-point transform
    /// with a frame clock at 79 Hz and rows at 112 Hz this runs over twelve
    /// million bins a second, and it was the largest single symbol in a profile
    /// of the running receiver.
    fn pool_to_bins / pool_to_bins_portable / pool_to_bins_avx2 / pool_to_bins_avx512 (
        out: &mut [u8],
        view: &[f32],
        lo_bin: f64,
        step: f64,
        db_floor: f32,
        scale: f32,
        mean: bool,
    ) {
        let n = view.len();
        for (b, o) in out.iter_mut().enumerate() {
            let lo = ((lo_bin + b as f64 * step) as usize).min(n.saturating_sub(1));
            let hi = ((lo_bin + (b + 1) as f64 * step) as usize).clamp(lo + 1, n);
            let mut m = 0.0f32;
            if mean {
                for &p in &view[lo..hi] {
                    m += p;
                }
                m /= (hi - lo) as f32;
            } else {
                for &p in &view[lo..hi] {
                    m = m.max(p);
                }
            }
            *o = ((db10(m + 1e-20) - db_floor) * scale).clamp(0.0, 255.0) as u8;
        }
    }
}

crate::simd::kernel! {
    /// Window one frame into the transform's working buffer.
    fn apply_window / apply_window_portable / apply_window_avx2 / apply_window_avx512 (
        dst: &mut [Complex32],
        src: &[Complex32],
        win: &[f32],
    ) {
        for (w, (x, k)) in dst.iter_mut().zip(src.iter().zip(win)) {
            *w = x * k;
        }
    }
}

crate::simd::kernel! {
    /// Fold one transform's power into the running average.
    fn fold_power / fold_power_portable / fold_power_avx2 / fold_power_avx512 (
        avg: &mut [f32],
        work: &[Complex32],
        norm: f32,
        alpha: f32,
    ) {
        for (a, x) in avg.iter_mut().zip(work) {
            let p = x.norm_sqr() * norm;
            *a += alpha * (p - *a);
        }
    }
}

crate::simd::kernel! {
    /// [`fold_power`] for a lane summing the transforms between rows, for
    /// their mean (see [`SpectrumAnalyzer::set_smooth`]).
    fn fold_power_sum / fold_power_sum_portable / fold_power_sum_avx2 / fold_power_sum_avx512 (
        avg: &mut [f32],
        sum: &mut [f32],
        work: &[Complex32],
        norm: f32,
        alpha: f32,
    ) {
        for ((a, h), x) in avg.iter_mut().zip(sum).zip(work) {
            let p = x.norm_sqr() * norm;
            *a += alpha * (p - *a);
            *h += *a;
        }
    }
}

crate::simd::kernel! {
    /// [`fold_power`] for a lane that is also holding the peak between rows.
    fn fold_power_hold / fold_power_hold_portable / fold_power_hold_avx2 / fold_power_hold_avx512 (
        avg: &mut [f32],
        hold: &mut [f32],
        work: &[Complex32],
        norm: f32,
        alpha: f32,
    ) {
        for ((a, h), x) in avg.iter_mut().zip(hold).zip(work) {
            let p = x.norm_sqr() * norm;
            *a += alpha * (p - *a);
            if *a > *h {
                *h = *a;
            }
        }
    }
}

/// Add one transform's power to `hold`, each cell moved to the frequency it
/// is centred on (reassignment, Auger & Flandrin 1995):
///   k̂ = k − (N / 2π) · Im(X_dh · X_h*) / |X_h|²
/// with `x` = X_h under the window and `xd` = X_dh under its derivative, N the
/// transform size.
///
/// `hold` is in display order (frequency ascending) on a grid `up` times finer
/// than the transform, `up = hold.len() / N`: point `i` covers bin coordinate
/// `[i/up − ½, (i+1)/up − ½)`. A cell that stays where it is fills its `up`
/// points at its power; a moved one puts `up` times its power, split between
/// the two points either side of k̂ — the same energy, concentrated, so the
/// mean over a column reads the same level in both cases. A cell that stays
/// is drawn at `keep` times its power. A cell at or under
/// `floor` times the transform's median, or one that would move further than
/// `reach` bins or past a band edge, stays.
#[allow(clippy::too_many_arguments)]
fn scatter_reassigned(
    hold: &mut [f32],
    rank: &mut Vec<f32>,
    x: &[Complex32],
    xd: &[Complex32],
    norm: f32,
    floor: f32,
    reach: f32,
    keep: f32,
) {
    let n = x.len();
    if n < 4 || xd.len() != n || hold.len() < n || !hold.len().is_multiple_of(n) {
        return;
    }
    let up = hold.len() / n;
    let last = (hold.len() - 1) as f32;
    rank.clear();
    rank.extend(x.iter().map(|v| v.norm_sqr()));
    let (_, med, _) = rank.select_nth_unstable_by(n / 2, |a, b| a.total_cmp(b));
    let gate = *med * floor;
    let half = n / 2;
    let to_bins = n as f32 / std::f32::consts::TAU;
    for k in 0..n {
        let q = x[k].norm_sqr();
        let p = q * norm;
        // Display index: k counted from the bottom edge of the band.
        let d0 = if k >= half { k - half } else { k + half };
        let off = if q > gate && q > 0.0 { (xd[k] * x[k].conj()).im / q * to_bins } else { 0.0 };
        let f = (d0 as f32 - off + 0.5) * up as f32 - 0.5;
        if off == 0.0 || !off.is_finite() || off.abs() > reach || !(0.0..=last).contains(&f) {
            let p = p * keep;
            hold[d0 * up..(d0 + 1) * up].iter_mut().for_each(|h| *h += p);
            continue;
        }
        let e = p * up as f32;
        let j = f.floor() as usize;
        let t = f - j as f32;
        hold[j] += e * (1.0 - t);
        if j + 1 < hold.len() {
            hold[j + 1] += e * t;
        }
    }
}

impl SpectrumAnalyzer {
    pub fn new(fft_size: usize, sample_rate: f64, avg_tc_secs: f32) -> Self {
        Self::with_hop_div(fft_size, sample_rate, avg_tc_secs, 2)
    }

    /// [`Self::new`] with the overlap named: the analyser steps
    /// `fft_size / hop_div` samples between transforms, so `2` is `new`'s 50 %.
    ///
    /// Worth raising where one transform takes a long time to fill. Resolution
    /// and update rate are the same trade on any analyser — a transform cannot
    /// resolve finer than the reciprocal of the time it covers — but the rate
    /// rows reach the waterfall need not be the rate whole windows arrive at.
    /// A zoomed-in lane running at a few kilohertz fills a 4096-point window
    /// only a couple of times a second; stepping an eighth of it instead of a
    /// half puts four times as many rows on screen for the same resolution and
    /// a few hundred extra microseconds of FFT a second.
    pub fn with_hop_div(
        fft_size: usize,
        sample_rate: f64,
        avg_tc_secs: f32,
        hop_div: usize,
    ) -> Self {
        Self::with_window(fft_size, fft_size, sample_rate, avg_tc_secs, hop_div)
    }

    /// [`Self::with_hop_div`] with the *analysis window* named separately from
    /// the transform: `window_len` samples are windowed and the remaining
    /// `fft_size - window_len` points are zero.
    ///
    /// The two are usually the same and there is nothing to choose. They part
    /// company wherever a display is finer than the signal is worth resolving,
    /// which is every zoomed-in view: a 3.7 kHz window across a 2560-column
    /// panel is 1.4 Hz a column, and a transform with a true bin for each of
    /// them is looking at **five and a half seconds** of signal at the rate a
    /// deep zoom runs at. Nothing survives that. FT8's tones are 160 ms long
    /// and 6.25 Hz apart, and a window that covers thirty-four of them draws
    /// them as one continuous bar — the "rough and boxed" waterfall of issue
    /// #302, where turning the FFT size *up* made it worse.
    ///
    /// Zero-padding separates the two things that were tangled together.
    /// `window_len` sets the resolution and the time smear, which is a property
    /// of the signal; `fft_size` sets how many points the answer is drawn on,
    /// which is a property of the screen. The padded transform interpolates
    /// between the true bins rather than inventing detail, so the trace is
    /// smooth — the stair-stepping of issue #195 — while each row still covers
    /// a fifth of a second of band.
    pub fn with_window(
        fft_size: usize,
        window_len: usize,
        sample_rate: f64,
        avg_tc_secs: f32,
        hop_div: usize,
    ) -> Self {
        let window_len = window_len.clamp(1, fft_size);
        let fft = FftPlanner::new().plan_fft_forward(fft_size);
        let window = blackman_harris(window_len);
        let coherent_gain: f32 = window.iter().sum();
        let hop = (window_len / hop_div.max(1)).max(1);
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];

        let mut analyzer = SpectrumAnalyzer {
            fft,
            fft_size,
            window_len,
            hop,
            window,
            coherent_gain,
            pending: Vec::with_capacity(fft_size * 2),
            scratch,
            work: vec![Complex32::default(); fft_size],
            avg_power: vec![0.0; fft_size],
            view: Vec::new(),
            hold: Vec::new(),
            smooth: false,
            hold_n: 0,
            reassign: false,
            reassign_floor: 4.0,
            reassign_keep: 1.0,
            dwindow: blackman_harris_derivative(window_len),
            work_d: Vec::new(),
            rank: Vec::new(),
            ra_up: 1,
            read_hold: false,
            hold_read: false,
            drew_empty_flag: false,
            alpha: 1.0,
            primed: false,
            peak_abs: 0.0,
            peak_track: false,
            seq: 0,
            transforms: 0,
            dc_suppress: true,
        };
        analyzer.set_avg_tc(avg_tc_secs, sample_rate);
        analyzer
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// How many samples one transform looks at — the figure that sets both the
    /// true resolution and how much time each row of the waterfall smears over.
    /// See [`Self::with_window`].
    pub fn window_len(&self) -> usize {
        self.window_len
    }

    /// Whether the bins either side of DC are replaced by their neighbours.
    ///
    /// On by default, because on a front end feeding this directly DC is the
    /// hardware's own LO leakage and a spike there is an artefact. Off for an
    /// analyser looking at a *mixed-down* window, where DC is the middle of
    /// whatever the operator is pointed at: blanking it would punch a hole
    /// through the signal they are looking at.
    pub fn set_dc_suppress(&mut self, on: bool) {
        self.dc_suppress = on;
    }

    /// Clear the overlap/averaging state (e.g. across a TX→RX transition so
    /// transmit samples don't contaminate the first receive frames).
    pub fn reset(&mut self) {
        self.pending.clear();
        self.avg_power.iter_mut().for_each(|p| *p = 0.0);
        self.hold.iter_mut().for_each(|p| *p = 0.0);
        self.primed = false;
        self.peak_abs = 0.0;
    }

    /// Start (or stop) holding the per-bin peak between waterfall rows.
    ///
    /// Costs one `max` per bin per transform, so it is only switched on for a
    /// lane whose rows somebody is actually clocking.
    pub fn set_row_hold(&mut self, on: bool) {
        if on == self.row_hold() {
            return;
        }
        self.hold = if on { self.avg_power.clone() } else { Vec::new() };
        if on && (self.smooth || self.reassign) {
            self.reset_hold();
        }
    }

    /// Whether the peak hold is running.
    pub fn row_hold(&self) -> bool {
        !self.hold.is_empty()
    }

    /// Read the next [`Self::make_frame`] from the held peaks rather than the
    /// running average.
    ///
    /// A flag rather than a second `make_frame`, so a row and the frame it
    /// rides in are pooled by *exactly* the same code over exactly the same
    /// viewport — two copies of that arithmetic would drift apart on the first
    /// change to either. Set it, build the row, clear it; see
    /// `Engine::make_row`.
    ///
    /// Armed even where no peaks are being held yet: such a frame reads the
    /// running average, exactly as every build before the row clock did, and
    /// [`Self::took_row`] then reports that this lane is the one drawing rows
    /// so its caller can switch the hold on. That is how a lane pays for the
    /// hold only while it is the lane somebody is looking at.
    pub fn set_read_hold(&mut self, on: bool) {
        self.read_hold = on;
    }

    /// Whether the last [`Self::make_frame`] was built as a waterfall row, and
    /// clear the flag.
    ///
    /// The caller sets [`Self::set_read_hold`] on every lane that *could* draw
    /// and then asks each of them this, rather than working out in advance
    /// which one will: the choice of lane is a dozen conditions deep in the
    /// engine's frame builder, and a second copy of it here would go wrong the
    /// first time either changed.
    pub fn took_row(&mut self) -> bool {
        std::mem::take(&mut self.hold_read)
    }

    /// Whether the last [`Self::make_frame`] was built out of an average with
    /// nothing in it, and clear the flag.
    ///
    /// An analyser that has not yet run a transform holds zeros, and zero maps
    /// to the display floor in every column — a black band the full width of
    /// the waterfall, lasting as long as it takes `fft_size` samples to arrive.
    /// On a zoom lane running at 62 kHz through a 16384-point window that is a
    /// quarter of a second. Read the same way as [`Self::took_row`], so a
    /// caller that cannot easily say in advance which lane will draw can ask
    /// afterwards whether the picture it got was one.
    pub fn drew_empty(&mut self) -> bool {
        std::mem::take(&mut self.drew_empty_flag)
    }

    /// Whether a transform has ever been folded in, so there is a spectrum here
    /// to draw.
    pub fn primed(&self) -> bool {
        self.primed
    }

    /// Take over the part of `src`'s averaged spectrum this analyser covers,
    /// resampled onto its own transform size, and count as primed.
    ///
    /// `window` is the fraction of `src`'s span this one looks at, ascending in
    /// frequency: `(0.0, 1.0)` for an analyser rebuilt over the same band,
    /// something narrower for a lane mixed down onto part of it.
    ///
    /// A new analyser holds zeros, and zero is the display floor in every
    /// column, so everything drawn from it before its first transform is a
    /// black band the full width of the waterfall — a quarter of a second for a
    /// zoom lane, well over a second for the digital modes' channel analyser.
    /// Seeded, it starts from the picture the wider lane already had of the same
    /// band: coarser than it will settle at, and honest, and it sharpens to its
    /// own resolution as the transforms fold in.
    ///
    /// Ignored where there is nothing to copy — an empty source, or a window
    /// reaching outside the band `src` measured.
    pub fn seed_from(&mut self, src: &SpectrumAnalyzer, window: (f64, f64)) {
        let (n, m) = (self.fft_size, src.fft_size);
        let (lo, hi) = window;
        if !src.primed || n == 0 || m == 0 || !(0.0..hi).contains(&lo) || hi > 1.0 {
            return;
        }
        let (nh, mh) = (n / 2, m / 2);
        // A bin holds the noise in its own width, so a picture resampled onto
        // narrower bins has to be scaled to them or its floor arrives at the
        // wider lane's level. `(hi - lo) · m / n` is exactly that ratio — a
        // half-span window through the same transform is half the width a bin,
        // a same-span rebuild at twice the points likewise — and it is what
        // keeps the seeded rows at the level the real transforms are about to
        // settle on instead of a band of saturated white across the waterfall.
        //
        // Right for the noise the eye reads the floor off, and wrong by the
        // same factor for a carrier, which arrives that much too quiet for the
        // one transform interval the seed lasts. That is the better of the two
        // errors by a long way: a carrier a shade dim for a tenth of a second
        // against every column of the picture pinned to the ceiling.
        let scale = ((hi - lo) * m as f64 / n as f64) as f32;
        // Both sides read and written in *display* order — frequency-ascending
        // — because natural FFT order puts the two edges of the band next to
        // each other and an interpolation across that seam is a blend of
        // opposite ends of the spectrum.
        let src_at = |k: usize| src.avg_power[if k < mh { k + mh } else { k - mh }];
        for i in 0..n {
            let f = lo + (i as f64 + 0.5) / n as f64 * (hi - lo);
            let at = (f * m as f64 - 0.5).clamp(0.0, (m - 1) as f64);
            let k = at.floor() as usize;
            let t = (at - k as f64) as f32;
            let (p0, p1) = (src_at(k), src_at((k + 1).min(m - 1)));
            self.avg_power[if i < nh { i + nh } else { i - nh }] = (p0 + (p1 - p0) * t) * scale;
        }
        self.primed = true;
    }

    /// Begin a fresh row: the peak starts again from where the spectrum is now,
    /// so a row whose interval contained no transform at all still reads as the
    /// current spectrum rather than as zero.
    pub fn reset_hold(&mut self) {
        if !self.hold.is_empty() {
            if self.reassign {
                // The reassigned grid, at the fineness the last row asked for.
                self.hold.clear();
                self.hold.resize(self.fft_size * self.ra_up, 0.0);
            } else {
                self.hold.resize(self.fft_size, 0.0);
                if self.smooth {
                    self.hold.fill(0.0);
                } else {
                    self.hold.copy_from_slice(&self.avg_power);
                }
            }
        }
        self.hold_n = 0;
    }

    /// Rows and columns as mean power (Welch) instead of peak. Switching
    /// starts the row in hand afresh, so a row is never half of each.
    // Why: HDZ 2026-11-06: "die Körnigkeit nervt mich sehr ... diesen
    //      grieseligen Kram finde ich schauerlich".
    pub fn set_smooth(&mut self, on: bool) {
        if on != self.smooth {
            self.smooth = on;
            self.reset_hold();
        }
    }

    /// Rows as the reassigned spectrogram (Auger & Flandrin 1995): each
    /// cell's power moved to the frequency it is centred on, so a carrier or a
    /// tone is drawn as a line one bin wide instead of the window's main lobe.
    /// `floor_db` is how far over the transform's median a cell has to stand
    /// to be moved; the noise under it stays in place and reads as the
    /// ordinary mean spectrum, so the floor keeps the calm of WELCH instead of
    /// turning into points. Switching starts the row in hand afresh.
    ///
    /// Only the frequency is reassigned: a waterfall row is the time grid, and
    /// moving power between rows that are already drawn is not possible.
    /// Costs a second transform per transform, on the lane holding rows only.
    // Why: HDZ 2026-11-07: "baue in den normalen Wasserfall das HTS-Spektrum
    //      als Alternative ein neben Welch".
    pub fn set_reassign(&mut self, on: bool, floor_db: f32) {
        self.reassign_floor = 10f32.powf(floor_db.clamp(0.0, 60.0) / 10.0);
        if on != self.reassign {
            self.reassign = on;
            self.reset_hold();
        }
    }

    /// How much darker the cells REASSIGN leaves in place are drawn, dB: the
    /// noise and whatever stands under the gate. Zero keeps them at their
    /// level, as WELCH draws them; raised, only what was moved onto a line
    /// stays bright, which is how a reassigned spectrogram is usually shown.
    // Why: HDZ 2026-11-07: "ist etwas schärfer, aber nicht viel, und schon
    //      gar keine Linien".
    pub fn set_reassign_dim(&mut self, dim_db: f32) {
        self.reassign_keep = 10f32.powf(-dim_db.clamp(0.0, 60.0) / 10.0);
    }

    pub fn set_avg_tc(&mut self, tc_secs: f32, sample_rate: f64) {
        let hop_time = self.hop as f32 / sample_rate as f32;
        self.alpha = if tc_secs <= 0.0 { 1.0 } else { 1.0 - (-hop_time / tc_secs).exp() };
    }

    /// Start (or stop) tracking the input peak [`Self::take_peak_dbfs`] reads.
    ///
    /// Off by default, because it costs a pass over every sample and only the
    /// terminal waterfall wants it — see [`Self::peak_track`](#structfield.peak_track).
    pub fn set_peak_track(&mut self, on: bool) {
        self.peak_track = on;
        if !on {
            self.peak_abs = 0.0;
        }
    }

    /// Consume IQ samples, running as many overlapped FFTs as fit.
    pub fn process(&mut self, iq: &[Complex32]) {
        if self.peak_track {
            let mut peak = self.peak_abs;
            for s in iq {
                let a = s.norm_sqr();
                if a > peak {
                    peak = a;
                }
            }
            self.peak_abs = peak;
        }
        self.pending.extend_from_slice(iq);

        // Walk the buffer with an offset and compact it once at the end, rather
        // than draining a hop off the front after every transform.
        //
        // `Vec::drain` from the front shifts everything after it down, so the
        // old loop memmoved the whole remaining buffer once per transform: on a
        // block of sixteen thousand samples through a 4096-point window that is
        // eight shifts of some ten thousand complex samples apiece, tens of
        // megabytes a second of pure copying, and it grows with the front end's
        // rate. One compaction per block does the same job.
        let mut at = 0;
        while self.pending.len() - at >= self.window_len {
            let frame = &self.pending[at..at + self.window_len];
            apply_window(&mut self.work[..self.window_len], frame, &self.window);
            // The transform is in place, so the tail holds the *last* one's
            // output rather than the zeros it was built with. A no-op on the
            // ordinary lane, where the window is the whole transform.
            self.work[self.window_len..].fill(Complex32::default());
            self.fft.process_with_scratch(&mut self.work, &mut self.scratch);

            let norm = 1.0 / (self.coherent_gain * self.coherent_gain);
            let alpha = if self.primed { self.alpha } else { 1.0 };
            self.primed = true;
            // Two kernels rather than one with a branch inside: this runs
            // `fft_size` times per transform and hundreds of times a second on
            // a wide front end, and the hold is off on most lanes.
            if self.hold.is_empty() {
                fold_power(&mut self.avg_power, &self.work, norm, alpha);
            } else if self.reassign {
                // The trace keeps the ordinary average; the row gets the
                // reassigned power.
                fold_power(&mut self.avg_power, &self.work, norm, alpha);
                let frame = &self.pending[at..at + self.window_len];
                self.work_d.resize(self.fft_size, Complex32::default());
                apply_window(&mut self.work_d[..self.window_len], frame, &self.dwindow);
                self.work_d[self.window_len..].fill(Complex32::default());
                self.fft.process_with_scratch(&mut self.work_d, &mut self.scratch);
                // A cell moved further than the main lobe's half width
                // (Blackman-Harris: four bins of the unpadded window) is noise
                // over noise, not a place.
                let reach = 4.0 * self.fft_size as f32 / self.window_len as f32;
                scatter_reassigned(
                    &mut self.hold,
                    &mut self.rank,
                    &self.work,
                    &self.work_d,
                    norm,
                    self.reassign_floor,
                    reach,
                    self.reassign_keep,
                );
                self.hold_n += 1;
            } else if self.smooth {
                fold_power_sum(&mut self.avg_power, &mut self.hold, &self.work, norm, alpha);
                self.hold_n += 1;
            } else {
                fold_power_hold(&mut self.avg_power, &mut self.hold, &self.work, norm, alpha);
            }
            self.transforms = self.transforms.wrapping_add(1);
            at += self.hop;
        }
        if at > 0 {
            self.pending.drain(..at);
        }
    }

    /// Transforms folded in since this analyser was built — see
    /// [`Self::transforms`](#structfield.transforms).
    pub fn transforms(&self) -> u64 {
        self.transforms
    }

    /// Peak input magnitude (dBFS) since the last call; resets on read.
    ///
    /// Reads zero unless [`Self::set_peak_track`] switched the scan on.
    pub fn take_peak_dbfs(&mut self) -> f32 {
        let p = self.peak_abs;
        self.peak_abs = 0.0;
        10.0 * (p + 1e-20).log10()
    }

    /// Averaged spectrum in dBFS, frequency-ascending (DC centered).
    pub fn spectrum_db(&self, out: &mut Vec<f32>) {
        out.clear();
        out.reserve(self.fft_size);
        let half = self.fft_size / 2;
        for &p in self.avg_power[half..].iter().chain(&self.avg_power[..half]) {
            out.push(10.0 * (p + 1e-20).log10());
        }
    }

    /// Build a display frame with `out_bins` bins (max-pooled), u8-mapped
    /// over `[db_floor, db_ceil]`. With `viewport = Some((lo_hz, hi_hz))`
    /// only that sub-span is extracted (zoomed display); the frame's
    /// center/span then describe the viewport.
    pub fn make_frame(
        &mut self,
        center_hz: f64,
        span_hz: f64,
        db_floor: f32,
        db_ceil: f32,
        out_bins: usize,
        viewport: Option<(f64, f64)>,
    ) -> SpectrumFrame {
        let n = self.fft_size;
        let half = n / 2;
        let out_bins = out_bins.clamp(1, n * 4);
        let scale = 255.0 / (db_ceil - db_floor).max(1e-3);

        let (frac_lo, frac_hi, out_center, out_span) = match viewport {
            Some((lo, hi)) if hi > lo && span_hz > 0.0 => {
                let full_lo = center_hz - span_hz / 2.0;
                let flo = ((lo - full_lo) / span_hz).clamp(0.0, 0.998);
                let fhi = ((hi - full_lo) / span_hz).clamp(flo + 0.002, 1.0);
                (flo, fhi, full_lo + (flo + fhi) / 2.0 * span_hz, (fhi - flo) * span_hz)
            }
            _ => (0.0, 1.0, center_hz, span_hz),
        };

        // Held peaks for a waterfall row, or the running average for the
        // trace — see [`Self::set_read_hold`]. Same pooling either way. A lane
        // asked for a row before its hold was switched on answers from the
        // average, which is the current spectrum and so a correct row; the
        // hold starts on the next one.
        self.hold_read = self.read_hold;
        self.drew_empty_flag = !self.primed;
        // A summed row is turned into its mean once, here; a row with no
        // transform in it reads as the current spectrum, as the peak does.
        if self.read_hold && (self.smooth || self.reassign) && !self.hold.is_empty() {
            if self.hold_n == 0 && self.reassign {
                // The reassigned grid is in display order and `up` points a bin.
                let up = (self.hold.len() / n).max(1);
                for (i, h) in self.hold.iter_mut().enumerate() {
                    let d = (i / up).min(n - 1);
                    *h = self.avg_power[if d >= half { d - half } else { d + half }];
                }
            } else if self.hold_n == 0 {
                self.hold.copy_from_slice(&self.avg_power);
            } else {
                let k = 1.0 / self.hold_n as f32;
                self.hold.iter_mut().for_each(|h| *h *= k);
            }
            self.hold_n = 1;
        }
        // A reassigned row is already in display order, on its own grid.
        let ra_read = self.read_hold && self.reassign && !self.hold.is_empty();
        if ra_read {
            let up = (self.hold.len() / n).max(1);
            self.view.clear();
            self.view.extend_from_slice(&self.hold);
            if self.dc_suppress && n > 16 {
                // The same ±2 bins around DC, read as the mean of bins 3–6
                // either side, as below.
                let bin =
                    |v: &[f32], d: usize| v[d * up..(d + 1) * up].iter().sum::<f32>() / up as f32;
                let mut acc = 0.0f32;
                for d in 3..=6 {
                    acc += bin(&self.view, half - d) + bin(&self.view, half + d);
                }
                self.view[(half - 2) * up..(half + 3) * up].fill(acc / 8.0);
            }
        } else {
            let power: &[f32] =
                if self.read_hold && !self.hold.is_empty() { &self.hold } else { &self.avg_power };

            // DC spike suppression (hardware LO leakage): read the ±2 bins
            // around DC as the average of their neighbors. Patch at read time so
            // the running average stays uncontaminated.
            let dc_repl = if self.dc_suppress && n > 16 {
                let mut acc = 0.0f32;
                for d in 3..=6 {
                    acc += power[d] + power[n - d];
                }
                Some(acc / 8.0)
            } else {
                None
            };
            // Rotate into display order once. Natural bin `nat` is drawn at
            // `(nat + half) % n`, which for an even `n` is the top half of the
            // spectrum followed by the bottom — two contiguous runs, so the whole
            // rotation is two `copy_from_slice` calls. See [`Self::view`] for why
            // this is not done per bin.
            self.view.clear();
            self.view.reserve(n);
            self.view.extend_from_slice(&power[half..]);
            self.view.extend_from_slice(&power[..half]);
            if let Some(repl) = dc_repl {
                // The five bins the old per-bin test picked out — naturally
                // `0, 1, 2, n-2, n-1` — are contiguous once rotated, because DC is
                // the middle of a DC-centred display.
                self.view[half - 2..=half + 2].fill(repl);
            }
        }
        // The grid the row is drawn from: the transform, or the finer
        // reassigned one.
        let n = self.view.len();
        let view = &self.view[..];
        let lo_bin = frac_lo * n as f64;
        let bin_range = (frac_hi - frac_lo) * n as f64;

        let mut bins = vec![0u8; out_bins];
        if bin_range < out_bins as f64 {
            bins.clear();
            // Stretching, not pooling: the window holds fewer bins than there
            // are columns to fill, so each bin has to cover several. Reading
            // between them rather than repeating them — a repeated bin is a
            // hard block, and a wall of them is what a zoom past the transform's
            // resolution used to look like. The same rule the finished-sweep
            // pooler uses; see `pool_window_to_frame` in the engine.
            //
            // Interpolated in power and then taken to dB, so the ramp between
            // two bins is the ramp between the two levels rather than between
            // their logarithms.
            for b in 0..out_bins {
                // Centre of this column in bin coordinates, less the half bin
                // that puts bin centres on column centres.
                let at = lo_bin + (b as f64 + 0.5) * bin_range / out_bins as f64 - 0.5;
                let k = at.floor().clamp(0.0, (n - 1) as f64) as usize;
                let t = (at - k as f64).clamp(0.0, 1.0) as f32;
                let (p0, p1) = (view[k], view[(k + 1).min(n - 1)]);
                let db = db10(p0 + (p1 - p0) * t + 1e-20);
                bins.push(((db - db_floor) * scale).clamp(0.0, 255.0) as u8);
            }
        } else {
            let step = bin_range / out_bins as f64;
            // Whole window, and a column width that lands on bin boundaries.
            let per = step.round() as usize;
            if lo_bin == 0.0 && (step - per as f64).abs() < 1e-9 && per >= 1 && per * out_bins <= n
            {
                pool_to_bins_uniform(
                    &mut bins,
                    &view[..per * out_bins],
                    per,
                    db_floor,
                    scale,
                    self.smooth || self.reassign,
                );
            } else {
                let mean = self.smooth || self.reassign;
                pool_to_bins(&mut bins, view, lo_bin, step, db_floor, scale, mean);
            }
        }

        // The reassigned grid for the next row: fine enough that a bin's
        // worth of it is no wider than a column.
        if self.reassign && self.read_hold {
            let bins_shown = ((frac_hi - frac_lo) * self.fft_size as f64).max(1.0);
            let per_bin = (out_bins as f64 / bins_shown).ceil().max(1.0) as usize;
            self.ra_up = per_bin.next_power_of_two().clamp(1, 16);
        }
        self.seq = self.seq.wrapping_add(1);
        SpectrumFrame {
            seq: self.seq,
            center_hz: out_center,
            span_hz: out_span,
            db_floor,
            db_ceil,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// A zero-padded transform must put the tone where the tone is and read the
    /// level the tone has — the padding interpolates between true bins, it does
    /// not move or rescale them.
    #[test]
    fn a_padded_transform_reads_the_same_tone_the_same_way() {
        let fs = 48_000.0;
        let tone_hz = 1_500.0f32;
        let sig: Vec<Complex32> = (0..200_000)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();

        let read = |an: &mut SpectrumAnalyzer| {
            an.set_dc_suppress(false);
            an.process(&sig);
            let mut db = Vec::new();
            an.spectrum_db(&mut db);
            let n = db.len();
            let (peak_bin, peak_db) = db
                .iter()
                .enumerate()
                .fold((0usize, f32::MIN), |b, (i, &v)| if v > b.1 { (i, v) } else { b });
            // Bin index back to a frequency: `spectrum_db` is DC-centred.
            let hz = (peak_bin as f64 - n as f64 / 2.0) * fs / n as f64;
            (hz, peak_db)
        };

        let (whole_hz, whole_db) = read(&mut SpectrumAnalyzer::new(4096, fs, 0.0));
        // The same 4096 samples of signal, drawn on a transform eight times
        // finer: an eighth of the time smear the plain 32768 would have had.
        let (pad_hz, pad_db) = read(&mut SpectrumAnalyzer::with_window(32_768, 4_096, fs, 0.0, 2));

        assert!((whole_hz - f64::from(tone_hz)).abs() < 20.0, "unpadded found it at {whole_hz}");
        assert!((pad_hz - f64::from(tone_hz)).abs() < 20.0, "padded found it at {pad_hz}");
        assert!(
            (whole_db - pad_db).abs() < 1.0,
            "padding moved the level: {whole_db:.2} dB against {pad_db:.2} dB"
        );
    }

    /// And the padding is padding: the tail of the working buffer holds the
    /// *previous* transform's output, so a lane that forgot to clear it would
    /// come out with a spectrum of its own history folded into every frame.
    /// A steady tone through such a lane still looks plausible; a tone that
    /// stops does not go away.
    #[test]
    fn the_padding_does_not_remember_the_last_transform() {
        let fs = 48_000.0;
        let n = 4_096;
        let mut an = SpectrumAnalyzer::with_window(16_384, n, fs, 0.0, 1);
        an.set_dc_suppress(false);
        let tone: Vec<Complex32> = (0..n * 4)
            .map(|i| {
                let ph = TAU * 1_500.0 * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        an.process(&tone);
        // Then silence, for as many transforms as the tone got.
        an.process(&vec![Complex32::new(0.0, 0.0); n * 4]);
        let mut db = Vec::new();
        an.spectrum_db(&mut db);
        let peak = db.iter().cloned().fold(f32::MIN, f32::max);
        assert!(peak < -100.0, "the tone outlived its own transform: peak {peak:.1} dBFS");
    }

    /// The window is what sets the time each transform covers, and the
    /// transform is what sets how many points it is drawn on. That separation
    /// is the whole of issue #302 — before it, asking for a finer picture asked
    /// for a longer one.
    #[test]
    fn the_window_and_the_transform_are_two_different_numbers() {
        let an = SpectrumAnalyzer::with_window(32_768, 4_096, 48_000.0, 0.0, 4);
        assert_eq!(an.fft_size(), 32_768);
        assert_eq!(an.window_len(), 4_096);
        // Unasked, they are the same thing and nothing changes.
        let plain = SpectrumAnalyzer::new(4_096, 48_000.0, 0.0);
        assert_eq!((plain.fft_size(), plain.window_len()), (4_096, 4_096));
        // A window longer than the transform is not a thing.
        let clamped = SpectrumAnalyzer::with_window(1_024, 8_192, 48_000.0, 0.0, 2);
        assert_eq!(clamped.window_len(), 1_024);
    }

    /// A lane asked for a row before its hold exists answers from the running
    /// average and says it drew, so the caller knows to switch the hold on.
    ///
    /// That is what lets a hold cost anything only on the lane somebody is
    /// actually pooling rows from: a device-wide analyser sitting behind a zoom
    /// lane pays a compare and a store per bin on every transform for a picture
    /// nobody is looking at, which on a 2.4 Msps front end through a
    /// 131072-point window is half a megabyte of write traffic per transform
    /// (issue #216).
    #[test]
    fn a_lane_asked_for_a_row_says_so_before_it_holds_anything() {
        let fs = 1_000_000.0;
        let n = 1024;
        let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
        an.set_dc_suppress(false);
        assert!(!an.row_hold(), "nothing is held until somebody asks");

        let tone: Vec<Complex32> = (0..n * 2)
            .map(|i| {
                let ph = TAU * 250_000.0 * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        an.process(&tone);

        // A row from a lane with no hold: the current spectrum, which is what
        // every build before the row clock drew, and not a frame of zeros.
        an.set_read_hold(true);
        let row = an.make_frame(0.0, fs, -120.0, 0.0, n, None);
        an.set_read_hold(false);
        assert!(an.took_row(), "the lane that drew the row has to say so");
        assert!(!an.took_row(), "…once");
        assert!(row.bins.iter().any(|&b| b > 40), "the row is empty: {:?}", row.bins.len());

        // A lane that was armed but did not draw says nothing, so its caller
        // can give the hold up.
        let mut idle = SpectrumAnalyzer::new(n, fs, 0.0);
        idle.set_read_hold(true);
        idle.set_read_hold(false);
        assert!(!idle.took_row(), "a lane nobody pooled from must not claim the row");
    }

    /// A tone that came and went between two waterfall rows still shows up in
    /// the row it happened in.
    ///
    /// The running average keeps only the newest transform (`avg_tc` 0, which
    /// is what the panadapter asks for), so by the time a row is taken the
    /// burst is already gone from it. That is how a CW dot shorter than the
    /// gap between two frames used to vanish entirely — the transform that saw
    /// it was computed, folded in, and overwritten before anyone looked. The
    /// peak hold is what makes a row mean "the loudest thing in this slice of
    /// time" instead of "whatever was last".
    #[test]
    fn a_burst_between_rows_survives_into_the_row_it_happened_in() {
        let fs = 1_000_000.0;
        let n = 1024;
        let tone_hz = 250_000.0f32;
        let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
        an.set_dc_suppress(false);
        an.set_row_hold(true);

        let burst: Vec<Complex32> = (0..n * 4)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        let quiet = vec![Complex32::new(0.0, 0.0); n * 8];

        // The burst, then silence — all inside one row interval.
        an.process(&burst);
        an.process(&quiet);

        // The strongest column anywhere in the frame — the tone's exact bin is
        // the other test's business.
        let loudest = |f: &SpectrumFrame| f.bins.iter().copied().max().unwrap_or(0);

        // The running average has forgotten it: this is what the trace draws,
        // and it is correct for the trace — the band *is* quiet now.
        let now = an.make_frame(0.0, fs, -120.0, 0.0, n, None);
        assert!(loudest(&now) < 40, "the average should have moved on: {}", loudest(&now));

        // The row has not.
        an.set_read_hold(true);
        let row = an.make_frame(0.0, fs, -120.0, 0.0, n, None);
        an.set_read_hold(false);
        assert!(loudest(&row) > 200, "the row should still hold the burst: {}", loudest(&row));

        // And the next row starts from where the band is now, not from the
        // peak it just reported — otherwise one burst would smear down the
        // whole waterfall.
        an.reset_hold();
        an.set_read_hold(true);
        let next = an.make_frame(0.0, fs, -120.0, 0.0, n, None);
        an.set_read_hold(false);
        assert!(loudest(&next) < 40, "the hold should have been released: {}", loudest(&next));
    }

    /// With the hold off, nothing about the analyser changes — the same
    /// numbers, the same frames, for every lane that does not clock rows.
    #[test]
    fn the_hold_is_inert_until_it_is_switched_on() {
        let fs = 1_000_000.0;
        let n = 256;
        let iq: Vec<Complex32> = (0..n * 4)
            .map(|i| {
                let ph = TAU * 100_000.0 * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();

        let mut plain = SpectrumAnalyzer::new(n, fs, 0.0);
        let mut held = SpectrumAnalyzer::new(n, fs, 0.0);
        held.set_row_hold(true);
        plain.process(&iq);
        held.process(&iq);

        assert!(!plain.row_hold());
        assert!(held.row_hold());
        assert_eq!(
            plain.make_frame(0.0, fs, -120.0, 0.0, n, None).bins,
            held.make_frame(0.0, fs, -120.0, 0.0, n, None).bins,
        );
    }

    /// The fast dB conversion has to agree with the real one far more closely
    /// than the picture it draws can show.
    ///
    /// A frame is `u8` over `db_ceil - db_floor`, so one level is a fraction of
    /// a dB — 0.31 for the 80 dB range a panadapter typically sits at. The
    /// assertion below is two orders of magnitude tighter than that, over the
    /// whole range of powers a spectrum can hold, so no rounding this does can
    /// move a column by one level.
    #[test]
    fn the_fast_db_conversion_is_far_finer_than_one_display_level() {
        let mut worst: f32 = 0.0;
        let mut at = 0.0f32;
        // Every octave from the `1e-20` floor every caller adds up to a
        // full-scale-squared power, sampled finely inside each.
        let mut p = 1e-20f32;
        while p < 1e6 {
            for k in 0..97 {
                let x = p * (1.0 + k as f32 / 97.0);
                let err = (db10(x) - 10.0 * x.log10()).abs();
                if err > worst {
                    worst = err;
                    at = x;
                }
            }
            p *= 2.0;
        }
        assert!(worst < 0.002, "worst error {worst} dB at p = {at:e}");
    }

    /// DC suppression must hide exactly the five bins it always hid, and no
    /// others.
    ///
    /// The rotation into display order used to be an `(i + half) % n` computed
    /// per bin, with the replacement chosen by `nat.min(n - nat) <= 2` — so
    /// "which bins get patched" was a property of that arithmetic. Now it is a
    /// `fill` over one range, and the two have to pick out the same bins: one
    /// off either way either leaves half the LO spike on screen or punches a
    /// wider hole through the middle of whatever the operator is tuned to.
    #[test]
    fn dc_suppression_replaces_the_five_centre_columns_and_nothing_else() {
        let fs = 1_000_000.0;
        let n = 1024;
        let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
        an.set_dc_suppress(false);

        // A flat floor with a large spike at DC, which is what LO leakage is.
        let mut iq: Vec<Complex32> = (0..n * 2)
            .map(|i| {
                let ph = TAU * 300_000.0 * i as f32 / fs as f32;
                Complex32::new(ph.cos() * 0.01, ph.sin() * 0.01)
            })
            .collect();
        for s in iq.iter_mut() {
            *s += Complex32::new(1.0, 0.0);
        }
        an.process(&iq);

        // One output bin per FFT bin, so a column *is* a bin.
        let off = an.make_frame(0.0, fs, -120.0, 0.0, n, None);
        an.set_dc_suppress(true);
        let on = an.make_frame(0.0, fs, -120.0, 0.0, n, None);

        let changed: Vec<usize> = (0..n).filter(|&i| off.bins[i] != on.bins[i]).collect();
        let want: Vec<usize> = (n / 2 - 2..=n / 2 + 2).collect();
        assert_eq!(changed, want, "DC patch covered the wrong columns");
        // And it really was a spike being taken out, not a no-op comparison.
        assert!(off.bins[n / 2] > on.bins[n / 2] + 40, "the LO spike should have been replaced");
    }

    #[test]
    fn tone_lands_in_the_right_bin_at_the_right_level() {
        let fs = 1_000_000.0;
        let n = 1024;
        let tone_hz = 250_000.0f32; // exactly bin n*0.25 above DC
        let mut an = SpectrumAnalyzer::new(n, fs, 0.0);

        let iq: Vec<Complex32> = (0..n * 4)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(ph.cos(), ph.sin())
            })
            .collect();
        an.process(&iq);

        let mut db = Vec::new();
        an.spectrum_db(&mut db);

        // DC-centered ordering: +fs/4 sits at 3/4 of the display.
        let expect = n * 3 / 4;
        let peak_bin = db.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert_eq!(peak_bin, expect);
        // Full-scale coherent tone should read close to 0 dBFS.
        assert!(db[peak_bin] > -1.0 && db[peak_bin] < 1.0, "{}", db[peak_bin]);
        // Blackman-Harris sidelobes: far bins well below -80 dB.
        assert!(db[n / 4] < -80.0, "{}", db[n / 4]);
    }

    /// The SMOOTH row: noise between two rows averaged (Welch) instead of
    /// held at its peak. Over twelve transforms of noise with a tone in it,
    /// the noise columns of the mean row spread far less than those of the
    /// peak row, and sit lower; the tone stands clear of them in both.
    #[test]
    fn a_mean_row_is_calmer_than_a_peak_row_and_keeps_the_tone() {
        let fs = 1_000_000.0;
        let n = 1024;
        let tone_hz = 250_000.0f32;
        // Deterministic noise: a small LCG, about ±0.05 per component.
        let mut seed: u32 = 12345;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let input: Vec<Complex32> = (0..n * 12)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(0.1 * ph.cos() + 0.1 * rnd(), 0.1 * ph.sin() + 0.1 * rnd())
            })
            .collect();
        let row = |smooth: bool| {
            let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
            an.set_dc_suppress(false);
            an.set_smooth(smooth);
            an.set_row_hold(true);
            an.reset_hold();
            an.process(&input);
            an.set_read_hold(true);
            let f = an.make_frame(0.0, fs, -140.0, 0.0, n / 4, None);
            an.set_read_hold(false);
            f.bins
        };
        let spread = |b: &[u8]| {
            // The noise: everything but the loudest tenth (the tone's columns).
            let mut v: Vec<f32> = b.iter().map(|x| *x as f32).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            let v = &v[..v.len() * 9 / 10];
            let m = v.iter().sum::<f32>() / v.len() as f32;
            ((v.iter().map(|x| (x - m).powi(2)).sum::<f32>() / v.len() as f32).sqrt(), m)
        };
        let (peak, mean) = (row(false), row(true));
        let ((sd_p, m_p), (sd_m, m_m)) = (spread(&peak), spread(&mean));
        assert!(sd_m < sd_p * 0.7, "noise spread mean {sd_m} vs peak {sd_p}");
        assert!(m_m < m_p, "noise level mean {m_m} vs peak {m_p}");
        let top = |b: &[u8]| *b.iter().max().unwrap() as f32;
        assert!(
            top(&mean) - m_m > 40.0,
            "tone over noise in the mean row: {} vs {m_m}",
            top(&mean)
        );
    }

    /// The reassigned row: a tone between two bins drawn as a line one or two
    /// bins wide, on its own frequency, where the mean row spreads it over the
    /// window's main lobe; the noise under the gate stays as calm as in the
    /// mean row and at its level.
    #[test]
    fn a_reassigned_row_draws_a_tone_as_a_line_on_its_frequency() {
        let fs = 48_000.0;
        let n = 1024;
        // 37.3 bins above DC: a third of the way between two bins.
        let tone_hz = 37.3 * fs as f32 / n as f32;
        let mut seed: u32 = 777;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let input: Vec<Complex32> = (0..n * 16)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(0.1 * ph.cos() + 0.01 * rnd(), 0.1 * ph.sin() + 0.01 * rnd())
            })
            .collect();
        let row = |reassign: bool| {
            let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
            an.set_dc_suppress(false);
            an.set_smooth(!reassign);
            an.set_reassign(reassign, 6.0);
            an.set_row_hold(true);
            an.reset_hold();
            an.process(&input);
            an.set_read_hold(true);
            let f = an.make_frame(0.0, fs, -140.0, 0.0, n, None);
            an.set_read_hold(false);
            f.bins
        };
        let (mean, sharp) = (row(false), row(true));
        // Display order: DC at n/2, so the tone sits at n/2 + 37.3.
        let at = n / 2 + 37;
        let peak = |b: &[u8]| (0..b.len()).max_by_key(|&i| b[i]).unwrap();
        assert!((peak(&sharp) as i64 - at as i64).abs() <= 1, "peak at {}", peak(&sharp));
        // Width: columns within 10 dB (about 26 steps of 140 dB / 255) of the top.
        let width = |b: &[u8]| {
            let top = *b.iter().max().unwrap() as i32;
            b.iter().filter(|&&v| top - v as i32 <= 18).count()
        };
        assert!(width(&sharp) <= 2, "reassigned line {} bins wide", width(&sharp));
        assert!(width(&mean) >= 4, "mean lobe only {} bins wide", width(&mean));
        // The noise far from the tone: same level within about 2 dB.
        let noise = |b: &[u8]| b[..n / 4].iter().map(|v| *v as f32).sum::<f32>() / (n / 4) as f32;
        assert!(
            (noise(&sharp) - noise(&mean)).abs() < 4.0,
            "noise {} vs {}",
            noise(&sharp),
            noise(&mean)
        );
    }

    /// Where one bin is drawn across many columns — a short window, or a deep
    /// look at a few bins — the reassigned row is laid on a grid that fine,
    /// so a tone is a line a few columns wide instead of a bin's worth
    /// stretched across sixteen; the mean row draws the window's main lobe.
    #[test]
    fn a_reassigned_row_draws_a_line_finer_than_a_bin() {
        let fs = 48_000.0;
        let n = 256;
        let bin_hz = fs / n as f64;
        // 21.37 bins above DC; the view is 32 bins around it, 512 columns.
        let tone_hz = 21.37 * bin_hz;
        let (lo, hi) = (tone_hz - 16.0 * bin_hz, tone_hz + 16.0 * bin_hz);
        let mut seed: u32 = 4242;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let input: Vec<Complex32> = (0..n * 32)
            .map(|i| {
                let ph = (std::f64::consts::TAU * tone_hz * i as f64 / fs) as f32;
                Complex32::new(0.1 * ph.cos() + 0.001 * rnd(), 0.1 * ph.sin() + 0.001 * rnd())
            })
            .collect();
        let row = |reassign: bool| {
            let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
            an.set_dc_suppress(false);
            an.set_smooth(!reassign);
            an.set_reassign(reassign, 6.0);
            an.set_row_hold(true);
            // Two rows: the first tells the analyser how fine the second's
            // grid has to be.
            let mut bins = Vec::new();
            for half in input.chunks(input.len() / 2) {
                an.reset_hold();
                an.process(half);
                an.set_read_hold(true);
                bins = an.make_frame(0.0, fs, -140.0, 0.0, 512, Some((lo, hi))).bins;
                an.set_read_hold(false);
            }
            bins
        };
        let (mean, sharp) = (row(false), row(true));
        let width = |b: &[u8]| {
            let top = *b.iter().max().unwrap() as i32;
            b.iter().filter(|&&v| top - v as i32 <= 18).count()
        };
        let peak = |b: &[u8]| (0..b.len()).max_by_key(|&i| b[i]).unwrap();
        // Where the mean row draws it, so switching moves no line. (Both sit
        // half a bin right of the true frequency: the frame draws a bin from
        // its frequency up, not centred on it.)
        assert!(
            (peak(&sharp) as i64 - peak(&mean) as i64).abs() <= 2,
            "peak at column {} vs {}",
            peak(&sharp),
            peak(&mean)
        );
        assert!(width(&sharp) <= 4, "reassigned line {} columns wide", width(&sharp));
        assert!(width(&mean) >= 24, "mean lobe only {} columns wide", width(&mean));
    }

    /// REASSIGN's dim: the cells left in place drawn that many dB darker, the
    /// line moved onto its frequency at its level.
    #[test]
    fn a_dimmed_reassigned_row_keeps_the_line_and_darkens_the_ground() {
        let fs = 48_000.0;
        let n = 1024;
        let tone_hz = 37.3 * fs as f32 / n as f32;
        let mut seed: u32 = 31;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let input: Vec<Complex32> = (0..n * 16)
            .map(|i| {
                let ph = TAU * tone_hz * i as f32 / fs as f32;
                Complex32::new(0.1 * ph.cos() + 0.01 * rnd(), 0.1 * ph.sin() + 0.01 * rnd())
            })
            .collect();
        let row = |dim: f32| {
            let mut an = SpectrumAnalyzer::new(n, fs, 0.0);
            an.set_dc_suppress(false);
            // A gate high enough that the noise's own peaks stay in place
            // too: at 6 dB the cells over it carry about a quarter of the
            // noise's energy, moved and undimmed, and the ground drops ~6 dB.
            an.set_reassign(true, 12.0);
            an.set_reassign_dim(dim);
            an.set_row_hold(true);
            an.reset_hold();
            an.process(&input);
            an.set_read_hold(true);
            let f = an.make_frame(0.0, fs, -140.0, 0.0, n, None);
            an.set_read_hold(false);
            f.bins
        };
        let (plain, dim) = (row(0.0), row(20.0));
        let ground = |b: &[u8]| b[..n / 4].iter().map(|v| *v as f32).sum::<f32>() / (n / 4) as f32;
        let top = |b: &[u8]| *b.iter().max().unwrap() as f32;
        // 20 dB is about 36 steps of 140 dB over 255.
        let drop = ground(&plain) - ground(&dim);
        assert!((30.0..42.0).contains(&drop), "ground dropped {drop} steps");
        assert!((top(&plain) - top(&dim)).abs() <= 2.0, "line {} vs {}", top(&plain), top(&dim));
    }
}
