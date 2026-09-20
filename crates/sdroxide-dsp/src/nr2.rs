//! NR2 — the Ephraim-Malah denoiser from [WDSP], ported to Rust.
//!
//! Ported from WDSP's `emnr.c` (Warren Pratt, NR0V, GPL-2.0-or-later), the
//! engine behind the **NR2** button in PowerSDR and Thetis. This is the
//! denoiser a great many operators mean when they say noise reduction on HF,
//! and it is here because it sounds like itself: the chain below is what an ear
//! trained on Thetis recognises, and substituting a textbook-equivalent stage
//! anywhere in it produces something else.
//!
//! Stock NR2 as `RXA.c` creates it — 4096-point FFT, 4x overlap,
//! `gain_method 2`, `npe_method 0`, artefact elimination on — is what is
//! reproduced here:
//!
//!  1. **Noise estimate — OSMS** (`LambdaD`): optimally-smoothed minimum
//!     statistics in Martin's form, with the bias correction (`invQeq`,
//!     `varHat`) that makes a minimum-follower estimate the noise *mean*.
//!  2. **A-priori SNR — decision-directed**, the same recursion the other
//!     spectral engines here use, smoothed with `alpha` derived from the frame
//!     rate rather than fixed.
//!  3. **Gain — a generalised-gamma speech prior**, evaluated by table lookup
//!     (see below), multiplied by a speech-presence factor that pulls the gain
//!     down where the prior claims signal but the observation shows none.
//!  4. **Artefact elimination** (`aepf`): a frequency-domain moving average
//!     over the mask whose width opens up as the frame gets noisier, which is
//!     what keeps hard suppression from ringing into birdies.
//!
//! ## The gain tables
//!
//! WDSP does not evaluate its gain rule at run time. It looks the answer up in
//! two 241x241 tables — `GG`, the gain, and `GGS`, the speech-presence factor —
//! generated offline and shipped as ~3 MB of C source in `calculus.c`. The
//! estimator's closed form is nowhere in WDSP; only the sampled result is.
//!
//! The tables are therefore copied rather than computed, transcoded to
//! little-endian `f32` by `tools/gen_nr2_tables.py` into `nr2_tables.bin`
//! (454 KiB, loaded once on first use). Computing them was tried first and
//! abandoned: the generalised-gamma family is demonstrably the right one — it
//! collapses to WDSP's own `gain_method 0` exactly at shape `nu = 1`, constant
//! `Gamma(1.5) = sqrt(pi)/2` and all — but no shape parameter reproduces the
//! shipped values. The closest, `nu ~ 0.35`, is off by 3.6 % median and 21 % at
//! the ninetieth percentile, against a table whose own grid costs ~0.4 %. A
//! gain rule that misses by that much is a different denoiser, so the data
//! stands as upstream computed it.
//!
//! Both axes run -30 dB to +30 dB in 0.25 dB steps — WDSP's `getKey` maps a
//! ratio through `10*log10(v / 0.001)` at four steps per dB — and lookups
//! outside that range clamp to the edge.
//!
//! **A known upstream artefact.** In the low-`xi` corner (`xi` below about
//! -24.75 dB) the tabulated gain underflowed to exactly zero for `gamma`
//! between roughly +22.5 and +27.75 dB, and then — worse — climbs back to 0.46
//! at +30 dB where the true gain is still falling through 0.002. The values are
//! left as upstream has them, because that corner cannot be reached: the
//! decision-directed recursion floors `xi` at `(1 - alpha) * (gamma - 1)`, and
//! with `alpha ~ 0.973` at stock framing a `gamma` of +22.5 dB forces `xi`
//! above -6 dB in the same frame. Should the smoothing ever be loosened enough
//! to reach it, this is where the +46 dB of misplaced gain came from.
//!
//! ## Deviations from upstream
//!
//!  * **The frame follows the audio rate.** WDSP runs at a fixed DSP rate and
//!    sizes the FFT in samples; the demodulator here hands over whatever rate
//!    the mode wants. The FFT is the power of two nearest a fixed *duration*,
//!    so the window spans the same milliseconds — and every time constant
//!    derived from it means the same thing — at 48 kHz and at 12 kHz.
//!  * **Strength is a layer on top**, not a change to the gain rule. NR2 has no
//!    intensity control: the Low/Med/High the rest of this program offers is a
//!    noise over-estimation factor and a gain floor applied to the finished
//!    mask, so the ported maths stays at its stock settings at every setting.
//!
//! [WDSP]: https://github.com/TAPR/OpenHPSDR-Thetis

use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

// ------------------------------------------------------------------ tables

/// Points per axis in WDSP's gain tables.
const GRID: usize = 241;
/// Lowest ratio the tables resolve; below this a lookup clamps to the edge.
/// WDSP's `dmin`.
const TBL_MIN: f32 = 0.001;
/// Highest ratio the tables resolve. WDSP's `dmax`.
const TBL_MAX: f32 = 1000.0;
/// Grid steps per dB — the 0.25 dB spacing, as a multiplier.
const STEPS_PER_DB: f32 = 4.0;

/// The gain tables, transcoded from WDSP's `calculus.c` by
/// `tools/gen_nr2_tables.py`.
const BLOB: &[u8] = include_bytes!("nr2_tables.bin");

/// WDSP's `GG` and `GGS`, each `GRID * GRID` and indexed `[GRID * n_xi + n_gamma]`.
struct Tables {
    /// The gain under the speech prior.
    gg: Vec<f32>,
    /// The speech-presence factor the gain is multiplied by.
    ggs: Vec<f32>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

/// The gain tables, decoded on first use.
///
/// Both denoiser instances — a receiver's and the audio-input path's — share
/// one copy: the tables are constant, and 454 KiB is worth decoding once.
fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let n = GRID * GRID;
        assert!(BLOB.starts_with(b"NR2T"), "nr2_tables.bin is not an NR2 table blob");
        let grid = u32::from_le_bytes(BLOB[4..8].try_into().unwrap()) as usize;
        assert_eq!(grid, GRID, "nr2_tables.bin has a {grid}-point axis, expected {GRID}");
        assert_eq!(BLOB.len(), 8 + 2 * 4 * n, "nr2_tables.bin is the wrong length");
        let read = |off: usize| {
            BLOB[off..off + 4 * n]
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect::<Vec<f32>>()
        };
        Tables { gg: read(8), ggs: read(8 + 4 * n) }
    })
}

/// Where a ratio falls on a table axis: `(low cell, high cell, fraction)`.
///
/// WDSP's `getKey` in one dimension. Outside `TBL_MIN ..= TBL_MAX` both cells
/// are the edge and the fraction is zero, which is what makes a lookup clamp
/// rather than wrap or extrapolate.
fn axis(v: f32) -> (usize, usize, f32) {
    if v <= TBL_MIN {
        (0, 0, 0.0)
    } else if v >= TBL_MAX {
        (GRID - 1, GRID - 1, 0.0)
    } else {
        let t = STEPS_PER_DB * 10.0 * (v / TBL_MIN).log10();
        // `min` because the rounding can land `t` on exactly `GRID - 1` for a
        // `v` fractionally below `TBL_MAX`, and the cell above would then be
        // off the end of the table. Upstream has the same edge in `double` and
        // has never been bitten by it; a panic in the audio thread is not the
        // way to find out whether `f32` is luckier.
        let i = (t as usize).min(GRID - 2);
        (i, i + 1, t - i as f32)
    }
}

/// Bilinear interpolation over one of the tables — WDSP's `getKey`.
///
/// `gamma` is the a-posteriori SNR and `xi` the a-priori SNR, both as power
/// ratios. The argument order matches upstream's, as does the indexing: `xi`
/// selects the row, `gamma` the column.
fn key(table: &[f32], gamma: f32, xi: f32) -> f32 {
    let (g1, g2, dg) = axis(gamma);
    let (x1, x2, dx) = axis(xi);
    (1.0 - dg) * (1.0 - dx) * table[GRID * x1 + g1]
        + (1.0 - dg) * dx * table[GRID * x2 + g1]
        + dg * (1.0 - dx) * table[GRID * x1 + g2]
        + dg * dx * table[GRID * x2 + g2]
}

/// The tabulated gain at `(gamma, xi)`, both power ratios.
pub(crate) fn gain(gamma: f32, xi: f32) -> f32 {
    key(&tables().gg, gamma, xi)
}

/// The tabulated speech-presence factor at `(gamma, xi)`, both power ratios.
pub(crate) fn speech_presence(gamma: f32, xi: f32) -> f32 {
    key(&tables().ggs, gamma, xi)
}

// ------------------------------------------------------------------ engine

/// Overlap factor — WDSP's `ovrlp`.
const OVRLP: usize = 4;

/// Analysis-window duration, in seconds.
///
/// WDSP fixes the FFT at 4096 samples and runs at a fixed DSP rate, which is
/// 85 ms at 48 kHz and three quarters of that in latency. Half the window is
/// kept here: it kept NR2 in the same latency class as the other engines on
/// this receiver, and the frequency resolution it costs is resolution the
/// noise estimator was averaging away over its 1.5-second window regardless.
const FRAME_S: f64 = 2048.0 / 48_000.0;

/// Ceiling on the a-posteriori SNR — WDSP's `gamma_max`.
const GAMMA_MAX: f32 = 1000.0;
/// Ceiling on the mask — WDSP's `gmax`.
const GMAX: f32 = 10_000.0;
/// A-priori probability of speech absence — WDSP's `q`.
const Q: f32 = 0.2;
/// Floor under the a-priori SNR estimate. WDSP's `eps_floor` is 1e-300, which
/// is a `double`'s idea of zero; the nearest thing `f32` has is used instead,
/// and the difference is unobservable — both exist only to keep the estimate
/// off zero.
const EPS_FLOOR: f32 = f32::MIN_POSITIVE;
/// Stands in for WDSP's `1.0e300` sentinel in the minimum trackers.
const BIG: f32 = 1e30;

/// Artefact-elimination threshold — WDSP's `ae.zetaThresh`.
const AE_ZETA_THRESH: f32 = 0.75;
/// Artefact-elimination width scale — WDSP's `ae.psi`.
const AE_PSI: f32 = 10.0;

/// Martin's bias-correction table: the `D` axis, shared by `M(D)` below.
const DVALS: [f32; 18] = [
    1.0, 2.0, 5.0, 8.0, 10.0, 15.0, 20.0, 30.0, 40.0, 60.0, 80.0, 120.0, 140.0, 160.0, 180.0,
    220.0, 260.0, 300.0,
];
/// Martin's `M(D)`, the mean of the minimum of `D` samples.
const MVALS: [f32; 18] = [
    0.000, 0.260, 0.480, 0.580, 0.610, 0.668, 0.705, 0.762, 0.800, 0.841, 0.865, 0.890, 0.900,
    0.910, 0.920, 0.930, 0.935, 0.940,
];

/// WDSP's `interpM`: interpolate `yvals` against `xvals` on a log-x axis,
/// clamping at both ends. Upstream's `Hvals` companion table is not here —
/// `LambdaD` never reads it.
fn interp_m(x: f32, xvals: &[f32], yvals: &[f32]) -> f32 {
    if x <= xvals[0] {
        return yvals[0];
    }
    if x >= xvals[xvals.len() - 1] {
        return yvals[yvals.len() - 1];
    }
    let mut idx = 0;
    while x >= xvals[idx] {
        idx += 1;
    }
    let (lo, hi) = (xvals[idx - 1].log10(), xvals[idx].log10());
    let frac = (x.log10() - lo) / (hi - lo);
    yvals[idx - 1] + frac * (yvals[idx] - yvals[idx - 1])
}

/// A time constant expressed the way WDSP writes them: the per-frame smoothing
/// factor that decays like `factor` did at 8 kHz with a 128-sample hop.
fn tau_alpha(factor: f64, hop: usize, rate: f64) -> f32 {
    let tau = -128.0 / 8000.0 / factor.ln();
    (-(hop as f64) / rate / tau).exp() as f32
}

/// Minimum-statistics noise-power estimation with optimal smoothing — WDSP's
/// `LambdaD`, which is Martin's estimator (2001) with the bias correction that
/// makes a minimum follower track the noise *mean* rather than its minimum.
struct Npe {
    bins: usize,
    alpha_c_smooth: f32,
    alpha_max: f32,
    alpha_cmin: f32,
    alpha_min_max_value: f32,
    snrq: f32,
    betamax: f32,
    inv_qeq_max: f32,
    av: f32,
    /// Sub-windows, and frames per sub-window.
    u: usize,
    v: usize,
    /// `M(D)` and `M(V)`.
    mof_d: f32,
    mof_v: f32,
    d: f32,
    invqbar_points: [f32; 3],
    nsmax: [f32; 4],

    alpha_c: f32,
    subwc: usize,
    amb_idx: usize,

    p: Vec<f32>,
    alpha_opt_hat: Vec<f32>,
    alpha_hat: Vec<f32>,
    sigma2n: Vec<f32>,
    pbar: Vec<f32>,
    p2bar: Vec<f32>,
    qeq: Vec<f32>,
    bmin: Vec<f32>,
    bmin_sub: Vec<f32>,
    k_mod: Vec<bool>,
    actmin: Vec<f32>,
    actmin_sub: Vec<f32>,
    lmin_flag: Vec<bool>,
    pmin_u: Vec<f32>,
    actminbuff: Vec<Vec<f32>>,
}

impl Npe {
    fn new(bins: usize, hop: usize, rate: f64) -> Self {
        // The window Martin averages over, 1.536 s, split into `u` sub-windows
        // of `v` frames. Upstream solves for both from the frame rate so the
        // window stays the same *duration* whatever the hop is.
        let dtime = 8.0 * 12.0 * 128.0 / 8000.0;
        let mut u = 8usize;
        let mut v = (0.5 + dtime * rate / (u as f64 * hop as f64)) as usize;
        if v < 4 {
            v = 4;
        }
        u = ((0.5 + dtime * rate / (v as f64 * hop as f64)) as usize).max(1);
        let d = (u * v) as f32;

        let vh = v as f64 * hop as f64 / rate;
        let nsmax = [8.0f64, 4.0, 2.0, 1.2].map(|slope| {
            let db = 10.0 * slope.log10() / (12.0 * 128.0 / 8000.0);
            10f64.powf(db / 10.0 * vh) as f32
        });

        let mut npe = Npe {
            bins,
            alpha_c_smooth: tau_alpha(0.7, hop, rate),
            alpha_max: tau_alpha(0.96, hop, rate),
            alpha_cmin: tau_alpha(0.7, hop, rate),
            alpha_min_max_value: tau_alpha(0.3, hop, rate),
            snrq: -(hop as f32) / (0.064 * rate as f32),
            betamax: tau_alpha(0.8, hop, rate),
            inv_qeq_max: 0.5,
            av: 2.12,
            u,
            v,
            mof_d: interp_m(d, &DVALS, &MVALS),
            mof_v: interp_m(v as f32, &DVALS, &MVALS),
            d,
            invqbar_points: [0.03, 0.05, 0.06],
            nsmax,
            alpha_c: 1.0,
            subwc: v,
            amb_idx: 0,
            p: Vec::new(),
            alpha_opt_hat: vec![0.0; bins],
            alpha_hat: vec![0.0; bins],
            sigma2n: Vec::new(),
            pbar: Vec::new(),
            p2bar: Vec::new(),
            qeq: vec![0.0; bins],
            bmin: vec![0.0; bins],
            bmin_sub: vec![0.0; bins],
            k_mod: vec![false; bins],
            actmin: Vec::new(),
            actmin_sub: Vec::new(),
            lmin_flag: vec![false; bins],
            pmin_u: Vec::new(),
            actminbuff: Vec::new(),
        };
        npe.reset();
        npe
    }

    /// Upstream seeds every power estimate at 0.5 rather than at zero, so the
    /// first frames divide by something sane instead of producing infinities.
    fn reset(&mut self) {
        let seed = 0.5f32;
        self.p = vec![seed; self.bins];
        self.sigma2n = vec![seed; self.bins];
        self.pbar = vec![seed; self.bins];
        self.pmin_u = vec![seed; self.bins];
        self.p2bar = vec![seed * seed; self.bins];
        self.actmin = vec![BIG; self.bins];
        self.actmin_sub = vec![BIG; self.bins];
        self.actminbuff = vec![vec![BIG; self.bins]; self.u];
        self.lmin_flag.fill(false);
        self.k_mod.fill(false);
        self.alpha_c = 1.0;
        self.subwc = self.v;
        self.amb_idx = 0;
    }

    /// One frame: fold `lambda_y` in and write the noise estimate to `lambda_d`.
    fn run(&mut self, lambda_y: &[f32], lambda_d: &mut [f32]) {
        let n = self.bins;
        let sum_prev_p: f32 = self.p.iter().sum();
        let sum_lambda_y: f32 = lambda_y.iter().sum::<f32>().max(f32::MIN_POSITIVE);
        let sum_prev_sigma2n: f32 = self.sigma2n.iter().sum::<f32>().max(f32::MIN_POSITIVE);

        for k in 0..n {
            let f0 = self.p[k] / self.sigma2n[k] - 1.0;
            self.alpha_opt_hat[k] = 1.0 / (1.0 + f0 * f0);
        }
        let snr = sum_prev_p / sum_prev_sigma2n;
        let alpha_min = self.alpha_min_max_value.min(snr.powf(self.snrq));
        for a in self.alpha_opt_hat.iter_mut() {
            *a = a.max(alpha_min);
        }
        let f1 = sum_prev_p / sum_lambda_y - 1.0;
        let alpha_ctilda = 1.0 / (1.0 + f1 * f1);
        self.alpha_c = self.alpha_c_smooth * self.alpha_c
            + (1.0 - self.alpha_c_smooth) * alpha_ctilda.max(self.alpha_cmin);
        let f2 = self.alpha_max * self.alpha_c;
        for k in 0..n {
            self.alpha_hat[k] = f2 * self.alpha_opt_hat[k];
            self.p[k] = self.alpha_hat[k] * self.p[k] + (1.0 - self.alpha_hat[k]) * lambda_y[k];
        }

        let mut inv_qbar = 0.0;
        for k in 0..n {
            let beta = self.betamax.min(self.alpha_hat[k] * self.alpha_hat[k]);
            self.pbar[k] = beta * self.pbar[k] + (1.0 - beta) * self.p[k];
            self.p2bar[k] = beta * self.p2bar[k] + (1.0 - beta) * self.p[k] * self.p[k];
            let var_hat = self.p2bar[k] - self.pbar[k] * self.pbar[k];
            // `var_hat` is a difference of two nearly equal averages and can
            // come out slightly negative in `f32` where upstream's `double`
            // stayed positive. Flooring it keeps `Qeq` finite; a negative
            // variance would otherwise propagate a NaN through every bin.
            let inv_qeq = (var_hat / (2.0 * self.sigma2n[k] * self.sigma2n[k]))
                .clamp(1e-12, self.inv_qeq_max);
            self.qeq[k] = 1.0 / inv_qeq;
            inv_qbar += inv_qeq;
        }
        inv_qbar /= n as f32;
        let bc = 1.0 + self.av * inv_qbar.sqrt();

        for k in 0..n {
            let qeq_tilda = (self.qeq[k] - 2.0 * self.mof_d) / (1.0 - self.mof_d);
            let qeq_tilda_sub = (self.qeq[k] - 2.0 * self.mof_v) / (1.0 - self.mof_v);
            self.bmin[k] = 1.0 + 2.0 * (self.d - 1.0) / qeq_tilda;
            self.bmin_sub[k] = 1.0 + 2.0 * (self.v as f32 - 1.0) / qeq_tilda_sub;
        }

        self.k_mod.fill(false);
        for k in 0..n {
            let f3 = self.p[k] * self.bmin[k] * bc;
            if f3 < self.actmin[k] {
                self.actmin[k] = f3;
                self.actmin_sub[k] = self.p[k] * self.bmin_sub[k] * bc;
                self.k_mod[k] = true;
            }
        }

        if self.subwc == self.v {
            // End of a sub-window: rotate the minimum buffers and allow the
            // estimate to climb, but no faster than the noise plausibly can.
            let noise_slope_max = if inv_qbar < self.invqbar_points[0] {
                self.nsmax[0]
            } else if inv_qbar < self.invqbar_points[1] {
                self.nsmax[1]
            } else if inv_qbar < self.invqbar_points[2] {
                self.nsmax[2]
            } else {
                self.nsmax[3]
            };

            for k in 0..n {
                if self.k_mod[k] {
                    self.lmin_flag[k] = false;
                }
                self.actminbuff[self.amb_idx][k] = self.actmin[k];
                let mut min = BIG;
                for ku in 0..self.u {
                    min = min.min(self.actminbuff[ku][k]);
                }
                self.pmin_u[k] = min;
                if self.lmin_flag[k]
                    && self.actmin_sub[k] < noise_slope_max * self.pmin_u[k]
                    && self.actmin_sub[k] > self.pmin_u[k]
                {
                    self.pmin_u[k] = self.actmin_sub[k];
                    for ku in 0..self.u {
                        self.actminbuff[ku][k] = self.actmin_sub[k];
                    }
                }
                self.lmin_flag[k] = false;
                self.actmin[k] = BIG;
                self.actmin_sub[k] = BIG;
            }
            self.amb_idx = (self.amb_idx + 1) % self.u;
            self.subwc = 1;
        } else {
            if self.subwc > 1 {
                for k in 0..n {
                    if self.k_mod[k] {
                        self.lmin_flag[k] = true;
                        self.sigma2n[k] = self.actmin_sub[k].min(self.pmin_u[k]);
                        self.pmin_u[k] = self.sigma2n[k];
                    }
                }
            }
            self.subwc += 1;
        }
        lambda_d.copy_from_slice(&self.sigma2n);
    }
}

/// NR2 — WDSP's Ephraim-Malah denoiser.
///
/// In place, same length in and out, with a fixed latency of
/// [`Nr2::latency_samples`] — one frame less one hop.
pub struct Nr2 {
    rate: f64,
    frame: usize,
    hop: usize,
    bins: usize,

    fwd: Arc<dyn RealToComplex<f32>>,
    inv: Arc<dyn ComplexToReal<f32>>,
    /// The square-root Hamming of WDSP's `wintype 0`, used for analysis and
    /// synthesis both.
    window: Vec<f32>,
    /// Undoes the unnormalised inverse FFT and the windowed overlap-add
    /// together, so an all-ones mask is an identity.
    inv_scale: f32,

    /// Noise over-estimation factor, and the floor under the finished mask —
    /// the Low/Med/High layer, which is ours rather than WDSP's.
    over: f32,
    floor: f32,
    /// Decision-directed smoothing — WDSP's `g.alpha`.
    alpha_dd: f32,

    npe: Npe,

    time: Vec<f32>,
    spec: Vec<Complex32>,
    lambda_y: Vec<f32>,
    lambda_d: Vec<f32>,
    mask: Vec<f32>,
    nmask: Vec<f32>,
    prev_mask: Vec<f32>,
    prev_gamma: Vec<f32>,

    inbuf: Vec<f32>,
    acc: Vec<f32>,
    out: VecDeque<f32>,
}

impl Nr2 {
    pub fn new() -> Self {
        let mut nr = Nr2 {
            rate: 0.0,
            frame: 0,
            hop: 0,
            bins: 0,
            fwd: RealFftPlanner::<f32>::new().plan_fft_forward(2),
            inv: RealFftPlanner::<f32>::new().plan_fft_inverse(2),
            window: Vec::new(),
            inv_scale: 1.0,
            over: 1.0,
            floor: 1.0,
            alpha_dd: 0.0,
            npe: Npe::new(1, 1, 48_000.0),
            time: Vec::new(),
            spec: Vec::new(),
            lambda_y: Vec::new(),
            lambda_d: Vec::new(),
            mask: Vec::new(),
            nmask: Vec::new(),
            prev_mask: Vec::new(),
            prev_gamma: Vec::new(),
            inbuf: Vec::new(),
            acc: Vec::new(),
            out: VecDeque::new(),
        };
        nr.rebuild(48_000.0);
        nr
    }

    /// Follow the demodulator's audio rate. Cheap to call per block: it only
    /// rebuilds when the rate has actually moved.
    pub fn set_rate(&mut self, rate: f64) {
        if (rate - self.rate).abs() < 0.01 || rate < 1000.0 {
            return;
        }
        self.rebuild(rate);
    }

    /// The Low/Med/High layer: `over` scales the noise estimate the gain rule
    /// is given, and `floor` is the least the mask may fall to. WDSP's own
    /// settings are `(1.0, 0.0)`.
    pub fn set_params(&mut self, over: f32, floor: f32) {
        self.over = over.max(0.0);
        self.floor = floor.clamp(0.0, 1.0);
    }

    pub fn reset(&mut self) {
        self.npe.reset();
        self.mask.fill(1.0);
        self.prev_mask.fill(1.0);
        self.prev_gamma.fill(1.0);
        self.lambda_d.fill(0.5);
        self.acc.fill(0.0);
        self.inbuf.clear();
        self.out.clear();
    }

    /// The delay the denoiser adds, in samples at the current rate.
    pub fn latency_samples(&self) -> usize {
        self.frame - self.hop
    }

    fn rebuild(&mut self, rate: f64) {
        self.rate = rate;
        // A whole number of hops, so the overlap-add is exact, and a power of
        // two so the transform is the fast one.
        let frame = ((rate * FRAME_S).round() as usize).next_power_of_two().clamp(256, 8192);
        let bins = frame / 2 + 1;
        self.frame = frame;
        self.hop = frame / OVRLP;
        self.bins = bins;

        let mut planner = RealFftPlanner::<f32>::new();
        self.fwd = planner.plan_fft_forward(frame);
        self.inv = planner.plan_fft_inverse(frame);

        // WDSP's `wintype 0`: the square root of a Hamming window. Upstream
        // then scales by its inverse coherent gain and divides the synthesis by
        // `fsize * ovrlp`; both are folded into `inv_scale` here, which is
        // derived from the window actually built rather than assumed, so it
        // stays unity at any frame size.
        self.window = (0..frame)
            .map(|i| {
                let arg = std::f32::consts::TAU * i as f32 / frame as f32;
                (0.54 - 0.46 * arg.cos()).sqrt()
            })
            .collect();
        let sum_sq: f32 = self.window.iter().map(|w| w * w).sum();
        self.inv_scale = 1.0 / (sum_sq * frame as f32 / self.hop as f32);

        self.npe = Npe::new(bins, self.hop, rate);
        self.alpha_dd = tau_alpha(0.98, self.hop, rate);

        self.time = vec![0.0; frame];
        self.spec = vec![Complex32::default(); bins];
        self.lambda_y = vec![0.0; bins];
        self.lambda_d = vec![0.5; bins];
        self.mask = vec![1.0; bins];
        self.nmask = vec![1.0; bins];
        self.prev_mask = vec![1.0; bins];
        self.prev_gamma = vec![1.0; bins];

        self.inbuf = Vec::with_capacity(frame + self.hop);
        self.acc = vec![0.0; frame];
        self.out = VecDeque::with_capacity(frame + self.hop);
    }

    /// Window and transform one frame, leaving the bin powers in `lambda_y`.
    fn analyze(&mut self) {
        for i in 0..self.frame {
            self.time[i] = self.inbuf[i] * self.window[i];
        }
        self.fwd.process(&mut self.time, &mut self.spec).expect("forward fft");
        for (p, c) in self.lambda_y.iter_mut().zip(&self.spec) {
            *p = c.re * c.re + c.im * c.im;
        }
    }

    /// WDSP's `calc_gain` at `gain_method 2` / `npe_method 0`, plus the
    /// strength layer.
    fn compute_mask(&mut self) {
        self.npe.run(&self.lambda_y, &mut self.lambda_d);

        for k in 0..self.bins {
            // The over-estimation factor is the only thing between the noise
            // estimate and the gain rule, so a harder setting simply tells the
            // rule there is more noise than there is.
            let lambda_d = (self.lambda_d[k] * self.over).max(f32::MIN_POSITIVE);
            let gamma = (self.lambda_y[k] / lambda_d).min(GAMMA_MAX);
            let eps_hat =
                self.alpha_dd * self.prev_mask[k] * self.prev_mask[k] * self.prev_gamma[k]
                    + (1.0 - self.alpha_dd) * (gamma - 1.0).max(EPS_FLOOR);
            let eps_p = eps_hat / (1.0 - Q);
            let mut mask = gain(gamma, eps_hat) * speech_presence(gamma, eps_p);
            if mask > GMAX {
                mask = GMAX;
            }
            if mask.is_nan() {
                // Upstream's guard, kept: a bin that has gone non-finite is
                // silenced rather than allowed to reach the overlap-add.
                mask = 0.01;
            }
            self.prev_gamma[k] = gamma;
            self.prev_mask[k] = mask;
            self.mask[k] = mask;
        }

        self.artefact_filter();

        // The floor goes on last, after the artefact filter has had the real
        // mask to work on: flooring first would flatten the very contrast the
        // filter measures to decide how hard to smooth.
        for m in self.mask.iter_mut() {
            *m = m.max(self.floor);
        }
    }

    /// WDSP's `aepf`: smooth the mask across frequency, by a width that opens
    /// up as more of the frame's energy is being removed. A mask carved into
    /// isolated spikes is what birdies sound like; averaging the spikes into
    /// their neighbours is what stops them.
    fn artefact_filter(&mut self) {
        let sum_pre: f32 = self.lambda_y.iter().sum();
        let sum_post: f32 = self.mask.iter().zip(&self.lambda_y).map(|(m, l)| m * m * l).sum();
        if sum_pre <= 0.0 {
            return;
        }
        let zeta = sum_post / sum_pre;
        let zeta_t = if zeta >= AE_ZETA_THRESH { 1.0 } else { zeta };
        let width = if zeta_t == 1.0 {
            1
        } else {
            1 + 2 * (0.5 + AE_PSI * (1.0 - zeta_t / AE_ZETA_THRESH)) as usize
        };
        if width <= 1 {
            return;
        }
        let n = width / 2;
        if 2 * n >= self.bins {
            return;
        }
        for k in n..self.bins - n {
            let sum: f32 = self.mask[k - n..=k + n].iter().sum();
            self.nmask[k] = sum / width as f32;
        }
        self.mask[n..self.bins - n].copy_from_slice(&self.nmask[n..self.bins - n]);
    }

    /// Apply the mask, transform back, and window into the overlap-add.
    fn synthesize(&mut self) {
        for (c, m) in self.spec.iter_mut().zip(&self.mask) {
            c.re *= *m;
            c.im *= *m;
        }
        // A real signal's spectrum has no imaginary part at DC or Nyquist, and
        // a real mask cannot introduce one; `realfft` rejects the transform if
        // it finds one, so this only guards against drift.
        self.spec[0].im = 0.0;
        if let Some(last) = self.spec.last_mut() {
            last.im = 0.0;
        }
        self.inv.process(&mut self.spec, &mut self.time).expect("inverse fft");
        for i in 0..self.frame {
            self.acc[i] += self.time[i] * self.window[i] * self.inv_scale;
        }
    }

    fn run_frame(&mut self) {
        self.analyze();
        self.compute_mask();
        self.synthesize();
    }

    pub fn process(&mut self, audio: &mut [f32]) {
        self.inbuf.extend_from_slice(audio);

        while self.inbuf.len() >= self.frame {
            self.run_frame();
            self.out.extend(self.acc[..self.hop].iter().copied());
            self.acc.copy_within(self.hop.., 0);
            self.acc[self.frame - self.hop..].fill(0.0);
            self.inbuf.drain(..self.hop);
        }

        // One output per input sample. The queue starts a frame less a hop
        // short, so it zero-fills through the priming and runs one delay behind
        // afterwards.
        for s in audio.iter_mut() {
            *s = self.out.pop_front().unwrap_or(0.0);
        }
    }
}

impl Default for Nr2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ratio in dB, as a table axis reads it.
    fn db(x: f32) -> f32 {
        10f32.powf(x / 10.0)
    }

    #[test]
    fn blob_decodes_to_two_full_tables() {
        let t = tables();
        assert_eq!(t.gg.len(), GRID * GRID);
        assert_eq!(t.ggs.len(), GRID * GRID);
        // The bounds the run-time code assumes. A gain cannot be negative, and
        // a probability cannot leave (0, 1] — either would mean a mangled
        // decode, which is otherwise only audible as a strange noise floor.
        assert!(t.gg.iter().all(|g| *g >= 0.0), "GG has a negative gain");
        assert!(t.ggs.iter().all(|p| *p > 0.0 && *p <= 1.0), "GGS is not a probability");
    }

    /// The one mistake that would leave everything running and merely wrong:
    /// transposing the axes. These four values come from WDSP's `calculus.c`
    /// and are deliberately asymmetric — at `xi +10 / gamma -10` the gain is
    /// 1.74, and at the transpose of that point it is 0.23.
    #[test]
    fn grid_points_match_wdsp() {
        for (xi, gamma, want) in [
            (0.0, 0.0, 0.529_693_2),
            (10.0, -10.0, 1.742_108_1),
            (-10.0, 10.0, 0.228_745_4),
            (30.0, -30.0, 19.119_792),
        ] {
            let got = gain(db(gamma), db(xi));
            assert!(
                (got - want).abs() <= 1e-5 * want.abs(),
                "GG at xi {xi:+} dB, gamma {gamma:+} dB: got {got}, want {want}"
            );
        }
        assert!((speech_presence(db(0.0), db(20.0)) - 0.399_457_2).abs() < 1e-6);
        assert!((speech_presence(db(-30.0), db(-30.0)) - 0.800_014_9).abs() < 1e-6);
    }

    #[test]
    fn interpolates_between_grid_points() {
        // Half a cell (0.125 dB) along gamma from a known pair.
        let lo = gain(db(0.0), db(0.0));
        let hi = gain(db(0.25), db(0.0));
        let mid = gain(db(0.125), db(0.0));
        assert!((mid - 0.5 * (lo + hi)).abs() < 1e-6, "{mid} is not midway between {lo} and {hi}");
    }

    #[test]
    fn clamps_outside_the_grid() {
        // Below dmin and above dmax the edge cell stands in, rather than the
        // lookup running off the end of the table or extrapolating a gain.
        assert_eq!(gain(db(-45.0), db(0.0)), gain(db(-30.0), db(0.0)));
        assert_eq!(gain(db(0.0), db(-45.0)), gain(db(0.0), db(-30.0)));
        assert_eq!(gain(db(45.0), db(0.0)), gain(db(30.0), db(0.0)));
        assert_eq!(gain(db(0.0), db(45.0)), gain(db(0.0), db(30.0)));
        // Including the degenerate arguments a silent bin produces.
        assert!(gain(0.0, 0.0).is_finite());
    }

    /// The two regimes the decision-directed recursion settles into, which
    /// together are the character of the denoiser.
    ///
    /// While `gamma` is below unity the a-priori estimate sits on its floor, and
    /// a *louder* bin is then evidence of more noise rather than of speech — so
    /// the gain falls as the bin rises. Once `gamma` clears unity the estimate
    /// tracks it and the gain opens back up to unity. Getting either direction
    /// backwards would still denoise something; it would just be the wrong
    /// something.
    #[test]
    fn gain_follows_the_operating_line_in_both_regimes() {
        let line = |gamma_db: f32| {
            let gamma = db(gamma_db);
            let xi = (gamma - 1.0).max(TBL_MIN);
            gain(gamma, xi) * speech_presence(gamma, xi)
        };
        // Noise only: suppression deepens as the bin gets louder.
        let mut prev = f32::INFINITY;
        for gamma_db in [-20.0f32, -15.0, -10.0, -5.0, 0.0] {
            let g = line(gamma_db);
            assert!(g < prev, "suppression should deepen: {prev} then {g} at {gamma_db} dB");
            prev = g;
        }
        // Signal present: the gain opens up, monotonically, and reaches unity.
        let mut prev = 0.0;
        for gamma_db in [1.0f32, 3.0, 5.0, 10.0, 20.0, 30.0] {
            let g = line(gamma_db);
            assert!(g > prev, "gain should open up: {prev} then {g} at {gamma_db} dB");
            prev = g;
        }
        assert!((prev - 1.0).abs() < 0.01, "gain should reach unity at high SNR, got {prev}");
    }

    /// White-ish noise from a small LCG — deterministic, so a failure is
    /// reproducible.
    fn noise(n: usize, amp: f32, seed: u32) -> Vec<f32> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                amp * ((s >> 8) as f32 / (1 << 23) as f32 - 1.0)
            })
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Noise through the denoiser at one setting, measured once the noise
    /// estimator has had its 1.5-second window to converge.
    fn suppressed_rms(over: f32, floor: f32) -> f32 {
        let mut nr = Nr2::new();
        nr.set_rate(48_000.0);
        nr.set_params(over, floor);
        let mut buf = noise(48_000 * 3, 0.2, 0x51EE);
        nr.process(&mut buf);
        rms(&buf[48_000 * 2..])
    }

    #[test]
    fn preserves_sample_count() {
        let mut nr = Nr2::new();
        nr.set_rate(48_000.0);
        for len in [512usize, 300, 480, 1024, 137, 4096] {
            let mut buf = vec![0.25f32; len];
            nr.process(&mut buf);
            assert_eq!(buf.len(), len);
            assert!(buf.iter().all(|s| s.is_finite()), "non-finite sample at len {len}");
        }
    }

    /// With the mask forced to unity the whole analysis/synthesis chain has to
    /// be an identity. That pins the window scaling, the frame size and the
    /// claimed latency at once — none of which an RMS test can see.
    ///
    /// The mask is set directly rather than through `set_params` because NR2
    /// has no transparent setting: its gain rule can exceed unity, so no
    /// combination of over-estimation and floor reduces to a bypass.
    #[test]
    fn unity_mask_is_an_identity() {
        let mut nr = Nr2::new();
        nr.set_rate(48_000.0);
        let input = noise(48_000, 0.2, 0xFEED);
        let (frame, hop) = (nr.frame, nr.hop);

        let mut out = Vec::with_capacity(input.len());
        nr.inbuf.extend_from_slice(&input);
        while nr.inbuf.len() >= frame {
            nr.analyze();
            nr.mask.fill(1.0);
            nr.synthesize();
            out.extend(nr.acc[..hop].iter().copied());
            nr.acc.copy_within(hop.., 0);
            nr.acc[frame - hop..].fill(0.0);
            nr.inbuf.drain(..hop);
        }

        // This loop emits hops directly rather than through the output queue,
        // so the output is aligned with the input: the queue is what turns the
        // alignment into `latency_samples` of delay, and it is not in the way
        // here. Skip one frame of overlap-add warm-up, where the earliest
        // output has only some of its four window contributions.
        for i in frame..out.len() {
            assert!(
                (out[i] - input[i]).abs() < 1e-3,
                "not transparent at {i}: {} vs {}",
                out[i],
                input[i]
            );
        }
    }

    #[test]
    fn reduces_broadband_noise() {
        let input = rms(&noise(48_000, 0.2, 0x51EE));
        let out = suppressed_rms(2.0, 0.07);
        let db = 20.0 * (out / input).log10();
        assert!(db < -6.0, "high should take more than 6 dB off the noise, took {db:.1} dB");
    }

    #[test]
    fn low_reduces_less_than_high() {
        let low = suppressed_rms(1.0, 0.30);
        let high = suppressed_rms(2.0, 0.07);
        assert!(high < low, "high ({high}) should suppress more than low ({low})");
    }

    #[test]
    fn runs_at_other_rates() {
        for rate in [8_000.0f64, 16_000.0, 44_100.0] {
            let mut nr = Nr2::new();
            nr.set_rate(rate);
            nr.set_params(1.4, 0.14);
            let mut buf = noise(rate as usize, 0.2, 0xC0FFEE);
            nr.process(&mut buf);
            assert!(buf.iter().all(|s| s.is_finite()), "non-finite output at {rate} Hz");
            assert!(nr.latency_samples() < rate as usize / 10, "implausible latency at {rate} Hz");
        }
    }

    /// The frame follows the audio rate rather than staying at WDSP's fixed
    /// 4096, so the window spans the same milliseconds whatever the mode hands
    /// over. Half of upstream's 85 ms is the target.
    #[test]
    fn frame_tracks_the_audio_rate() {
        let mut nr = Nr2::new();
        for (rate, want) in [(48_000.0, 2048usize), (24_000.0, 1024), (12_000.0, 512)] {
            nr.set_rate(rate);
            assert_eq!(nr.frame, want, "frame at {rate} Hz");
            assert_eq!(nr.hop, want / OVRLP);
            assert_eq!(nr.latency_samples(), want - want / OVRLP);
            let ms = 1000.0 * want as f64 / rate;
            assert!((ms - 42.7).abs() < 1.0, "window is {ms:.1} ms at {rate} Hz");
        }
    }
}
