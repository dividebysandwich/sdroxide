//! Where FIT's floor leaves the noise under PEAK, WELCH and REASSIGN: a
//! measurement through the whole engine, for the FIT program each look gets.
//!
//! FIT measures the frame's spectrum line (`bins`), but the waterfall draws
//! the rows — and the three looks draw their rows differently: PEAK the
//! strongest bin per column, WELCH the mean power averaged over neighbours and
//! time, REASSIGN the cells moved to their frequency and the rest dimmed by
//! `reassign_dim_db`. This test feeds the engine Gaussian noise with a weak and
//! a strong carrier, applies the UI's own row averaging (a copy of
//! `waterfall_gpu::average_row`) and FIT's program per look (a copy of
//! `app::spectrum::pick_levels` and `wanted_levels`), prints where the drawn
//! noise and the weak carrier land, and holds the looks to one ground: the
//! drawn noise within `TOL` dB over the floor of where it is under PEAK.
//! Measured 2026-11-12: PEAK 13.4 / 11.8 dB, WELCH 10.6 / 10.2, REASSIGN
//! 10.6 / 10.1 (192 kHz FFT 4096 / 96 kHz FFT 16384). Without REASSIGN's floor
//! following its dim the noise would sit 13 dB nearer the floor — clipped
//! black — which is what the bound is there to catch (checked: 0.6 dB).
//!
//! Run with `--nocapture` to read the table; `FPL_CASE=<span Hz>,<fft>` tries
//! another view, `FPL_EXTRA` a floor further down for WELCH and REASSIGN,
//! `FPL_TOL` another bound.
// Why: HDZ 2026-11-12: "baue gleich welch und reassign ein, müssen die
//      unterschiedliche fit targets bekommen, quasi als eigene Programme?"

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig};

const RATE: f64 = 192_000.0;
const CENTER: f64 = 7_100_000.0;
/// A strong carrier and a weak one, off any bin centre and clear of DC.
const STRONG_OFF: f64 = 20_123.4;
const WEAK_OFF: f64 = -31_456.7;
/// Their amplitudes against the noise's per-component sigma of 0.002.
const STRONG_A: f32 = 0.05;
const WEAK_A: f32 = 0.0015;
/// The rows' clock the test asks for.
const ROWS_PER_SEC: u16 = 30;
/// REASSIGN's dim, the UI's default.
const DIM: f32 = 10.0;

struct Noise {
    center_hz: f64,
    n: u64,
    seed: u64,
}

impl Noise {
    fn uniform(&mut self) -> f64 {
        // xorshift64*
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        let v = self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    /// Two independent standard normals (Box–Muller).
    fn gauss2(&mut self) -> (f32, f32) {
        let (u, v) = (self.uniform(), self.uniform());
        let r = (-2.0 * u.ln()).sqrt();
        let t = std::f64::consts::TAU * v;
        ((r * t.cos()) as f32, (r * t.sin()) as f32)
    }
}

impl IqSource for Noise {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min((RATE * 0.005) as usize);
        let ws = std::f64::consts::TAU * (CENTER + STRONG_OFF - self.center_hz) / RATE;
        let ww = std::f64::consts::TAU * (CENTER + WEAK_OFF - self.center_hz) / RATE;
        for s in buf[..n].iter_mut() {
            let (gi, gq) = self.gauss2();
            let t = self.n as f64;
            *s = Complex32::new(
                STRONG_A * (ws * t).cos() as f32 + WEAK_A * (ww * t).cos() as f32 + 0.002 * gi,
                STRONG_A * (ws * t).sin() as f32 + WEAK_A * (ww * t).sin() as f32 + 0.002 * gq,
            );
            self.n += 1;
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "noise and two carriers".into()
    }
}

fn engine() -> EngineHandles {
    start_engine(
        Box::new(Noise { center_hz: CENTER, n: 0, seed: 0x9E37_79B9_7F4A_7C15 }),
        DeviceCaps {
            driver: "test".into(),
            label: "test".into(),
            rx_channels: 1,
            sample_rates: vec![RATE],
            freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
            ..DeviceCaps::default()
        },
        EngineConfig { remember_session: false, ..Default::default() },
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Look {
    Peak,
    Welch,
    Reassign,
}

/// The UI's row average (copy of `waterfall_gpu::average_row`), on dB values.
fn average_row(row: &mut [f32], avg: &mut Vec<f32>, new: f32, kernel: &[f32]) {
    let lin = |db: f32| 10f32.powf(db / 10.0);
    let n = row.len();
    let r = (kernel.len() / 2) as isize;
    let p: Vec<f32> = (0..n)
        .map(|i| {
            kernel
                .iter()
                .enumerate()
                .map(|(j, w)| {
                    let at = (i as isize + j as isize - r).clamp(0, n as isize - 1) as usize;
                    w * lin(row[at])
                })
                .sum()
        })
        .collect();
    if avg.len() != n {
        *avg = p;
    } else {
        for (a, v) in avg.iter_mut().zip(&p) {
            *a += new.clamp(0.0, 1.0) * (v - *a);
        }
    }
    for (b, a) in row.iter_mut().zip(avg.iter()) {
        *b = 10.0 * a.log10();
    }
}

fn pct(v: &[f32], p: f32) -> f32 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[((p * (s.len() - 1) as f32).round() as usize).min(s.len() - 1)]
}

/// FIT's floor under the line's 25th percentile, and its least span (widened
/// about the middle) — the 5 dB and 24 dB of `app::spectrum::pick_levels`.
const BASE_MARGIN: f32 = 5.0;
const MIN_SPAN: f32 = 24.0;
/// How far the drawn noise may sit from PEAK's, dB over the floor.
const TOL: f32 = 3.5;

/// A floor further down for WELCH and REASSIGN, to try one: `FPL_EXTRA`.
fn extra() -> f32 {
    std::env::var("FPL_EXTRA").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0)
}
/// Another bound, to try one: `FPL_TOL`.
fn tol() -> f32 {
    std::env::var("FPL_TOL").ok().and_then(|v| v.parse().ok()).unwrap_or(TOL)
}

/// FIT's program per look (copy of `app::spectrum::pick_levels` and
/// `wanted_levels`): the floor `BASE_MARGIN` under the line's 25th percentile,
/// the ceiling 6 dB over the 99th, at least `MIN_SPAN` apart; REASSIGN's floor
/// then `dim` lower still, its ceiling where the lines are.
fn program_fit(bins_db: &[f32], look: Look, dim: f32) -> (f32, f32) {
    let margin = if look == Look::Peak { BASE_MARGIN } else { BASE_MARGIN + extra() };
    let mut floor = pct(bins_db, 0.25) - margin;
    let mut ceil = pct(bins_db, 0.99) + 6.0;
    if ceil - floor < MIN_SPAN {
        let mid = 0.5 * (ceil + floor);
        (floor, ceil) = (mid - 0.5 * MIN_SPAN, mid + 0.5 * MIN_SPAN);
    }
    if look == Look::Reassign { (floor - dim, ceil) } else { (floor, ceil) }
}

struct Measured {
    /// The fit FIT would pick from the spectrum line, median over frames.
    fit: (f32, f32),
    /// The drawn rows: noise median and 90th percentile, the weak carrier.
    noise50: f32,
    noise90: f32,
    weak: f32,
    /// The spectrum line's own 25th percentile, what the fit leans on.
    bins25: f32,
}

fn measure(look: Look, span: f64, fft: u32) -> Measured {
    let mut h = engine();
    let lo = CENTER - span / 2.0;
    let (db_floor, db_ceil) = (-160.0f32, 0.0f32);
    h.cmd_tx
        .send(Command::SetSpectrumCfg(SpectrumConfig {
            fft_size: fft,
            display_bins: 1024,
            rows_per_sec: ROWS_PER_SEC,
            db_floor,
            db_ceil,
            viewport: Some((lo, lo + span)),
            smooth: look == Look::Welch,
            reassign: look == Look::Reassign,
            reassign_floor_db: sdroxide_types::reassign_floor_db_default(),
            reassign_dim_db: if look == Look::Reassign { DIM } else { 0.0 },
            fps: 30,
            avg_tc: 0.0,
        }))
        .unwrap();
    // The UI's defaults: WELCH and REASSIGN 0.1 s; PEAK unaveraged.
    let (tc, kernel): (f32, &[f32]) = match look {
        Look::Peak => (0.0, &[1.0]),
        Look::Welch => (0.1, &[1.0 / 9.0, 2.0 / 9.0, 3.0 / 9.0, 2.0 / 9.0, 1.0 / 9.0]),
        Look::Reassign => (0.1, &[1.0]),
    };
    let new = 1.0 - (-1.0 / (ROWS_PER_SEC as f32 * tc.max(0.01))).exp();
    let to_db = |b: u8| db_floor + b as f32 / 255.0 * (db_ceil - db_floor);
    let (mut fits_f, mut fits_c, mut b25) = (Vec::new(), Vec::new(), Vec::new());
    let (mut noise, mut weak) = (Vec::new(), Vec::new());
    let mut avg = Vec::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if h.spectrum_out.update() {
            let f = h.spectrum_out.output_buffer();
            let cols = f.bins.len();
            if cols == 0 || start.elapsed() < Duration::from_millis(1500) {
                continue;
            }
            let base = f.center_hz - f.span_hz / 2.0;
            let col = |hz: f64| (((hz - base) / f.span_hz) * cols as f64) as isize;
            let (cs, cw) = (col(CENTER + STRONG_OFF), col(CENTER + WEAK_OFF));
            // Columns away from both carriers and the DC patch count as noise.
            let guard = (cols / 64).max(4) as isize;
            let dc = col(CENTER);
            let is_noise = |i: usize| {
                let i = i as isize;
                (i - cs).abs() > guard && (i - cw).abs() > guard && (i - dc).abs() > guard
            };
            let bins: Vec<f32> = f.bins.iter().map(|&b| to_db(b)).collect();
            let (fl, ce) = program_fit(&bins, look, DIM);
            fits_f.push(fl);
            fits_c.push(ce);
            b25.push(pct(&bins, 0.25));
            for chunk in f.rows.chunks_exact(cols) {
                let mut row: Vec<f32> = chunk.iter().map(|&b| to_db(b)).collect();
                if look != Look::Peak {
                    average_row(&mut row, &mut avg, new, kernel);
                }
                noise.extend(row.iter().enumerate().filter(|(i, _)| is_noise(*i)).map(|(_, v)| *v));
                if (0..cols as isize).contains(&cw) {
                    let a = (cw - 1).max(0) as usize;
                    let b = ((cw + 2) as usize).min(cols);
                    weak.push(row[a..b].iter().cloned().fold(f32::MIN, f32::max));
                }
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }
    drop(h.cmd_tx);
    Measured {
        fit: (pct(&fits_f, 0.5), pct(&fits_c, 0.5)),
        noise50: pct(&noise, 0.5),
        noise90: pct(&noise, 0.9),
        weak: if weak.is_empty() { f32::NAN } else { pct(&weak, 0.5) },
        bins25: pct(&b25, 0.5),
    }
}

#[test]
fn each_looks_fit_program_draws_the_noise_on_the_same_ground() {
    let cases: Vec<(f64, u32)> = match std::env::var("FPL_CASE") {
        Ok(c) => {
            let (a, b) = c.split_once(',').unwrap();
            vec![(a.parse().unwrap(), b.parse().unwrap())]
        }
        Err(_) => vec![(192_000.0, 4096), (96_000.0, 16384)],
    };
    for (span, fft) in cases {
        let mut peak_above = None;
        for look in [Look::Peak, Look::Welch, Look::Reassign] {
            let m = measure(look, span, fft);
            let (fl, ce) = m.fit;
            let at = |db: f32| (db - fl) / (ce - fl);
            eprintln!(
                "span {span:>6.0} FFT {fft:>5} {look:<8?}: line p25 {:7.1} | fit {fl:7.1}..{ce:6.1} | rows noise p50 {:7.1} ({:.2}) p90 {:7.1} ({:.2}) | weak {:7.1} ({:.2}) | rows p50 - line p25 {:+.1} dB",
                m.bins25,
                m.noise50,
                at(m.noise50),
                m.noise90,
                at(m.noise90),
                m.weak,
                at(m.weak),
                m.noise50 - m.bins25,
            );
            assert!(m.noise50.is_finite() && m.fit.0.is_finite(), "{look:?}: nothing measured");
            let above = m.noise50 - fl;
            match peak_above {
                None => peak_above = Some(above),
                Some(p) => assert!(
                    (above - p).abs() <= tol(),
                    "{look:?}: drawn noise {above:.1} dB over the floor, PEAK {p:.1}"
                ),
            }
            assert!(at(m.weak) > at(m.noise90) + 0.2, "{look:?}: the weak carrier is lost");
        }
    }
}
