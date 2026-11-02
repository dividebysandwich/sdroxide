#![allow(clippy::needless_range_loop)]

use super::{NF, SYMLEN, to48};
use crate::demap::soft_bits;
use crate::dsp::{CHI_MIN, Hold, Track, Window, argmax, par_map, peak_mid, thin, unit};
use crate::eq;
use crate::fm::code::{Codec, Phasors};
use crate::fm::constellation::constellation;
use crate::fm::level::{Level, PILOT_PERIOD, Record, qam_pilot};
use crate::fm::tx::symbol_map;
use num_complex::{Complex32, Complex64};
use rustfft::{Fft, FftPlanner};
use std::sync::{Arc, LazyLock};

pub const HOP: i64 = 4;
pub const SYM: i64 = SYMLEN / HOP;
pub const MK_FRAC: f64 = 0.25;
pub const MIN_COHERENCE: f64 = 0.45;
pub const CP_MID: i64 = 10;
const PP: usize = PILOT_PERIOD as usize;
const MAX_DAT: usize = 600;
const REFINE: i64 = 3;
const SN_BW_HZ: f64 = 2500.0;
const CATCH_UP: i64 = 256;

static FFT: LazyLock<Arc<dyn Fft<f64>>> = LazyLock::new(|| FftPlanner::new().plan_fft_forward(NF));

pub struct Stft {
    win: Window,
    spectra: Track<Vec<Complex32>>,
    pub(crate) diff: Track<Vec<Complex32>>,
    stft_next: i64,
}

impl Stft {
    pub fn new(max_spf: usize) -> Self {
        Self {
            win: Window::new(NF),
            spectra: Track::new(2 * SYM as usize + 8),
            diff: Track::new(SYM as usize * max_spf + 64),
            stft_next: 0,
        }
    }

    pub fn update(&mut self, buf: &Track<Complex64>) {
        let j_hi = (buf.end() - NF as i64).div_euclid(HOP);
        let j0 = self.stft_next.max((buf.base + HOP - 1).div_euclid(HOP));
        for j in j0..=j_hi {
            let seg = self.win.at(buf, j * HOP, NF);
            let row: Vec<Complex32> = seg.iter().map(|z| Complex32::new(z.re as f32, z.im as f32)).collect();
            let g: Vec<Complex32> = if j - SYM >= self.spectra.base && j - SYM < self.spectra.end() {
                row.iter()
                    .zip(self.spectra.at(j - SYM))
                    .map(|(&a, &b)| {
                        let p = a * b.conj();
                        p / (p.norm() + 1e-30)
                    })
                    .collect()
            } else {
                vec![Complex32::default(); NF]
            };
            self.spectra.append(j, [row]);
            self.diff.append(j, [g]);
        }
        self.stft_next = self.stft_next.max(j_hi + 1);
    }
}

#[derive(Clone, Copy)]
struct Cell {
    row: usize,
    carrier: usize,
    bin: usize,
    reference: Complex32,
}

#[derive(Clone, Copy, Debug)]
pub struct Detection {
    pub record: Record,
    pub t: i64,
    pub metric: f64,
}

#[derive(Clone, Debug, Default)]
pub struct FrameRx {
    pub record: Record,
    pub index: usize,
    pub t: i64,
    pub t_end: i64,
    pub ok: bool,
    pub missing: bool,
    pub bytes: Vec<u8>,
    pub esn0_db: f64,
    pub snr_db: f64,
    pub iterations: usize,
    pub coherence: f64,
}

impl FrameRx {
    pub fn t_end48(&self) -> i64 {
        to48(self.t_end)
    }
}

pub struct OfdmEngine {
    pub record: Record,
    pub l: Level,
    map: Vec<Vec<bool>>,
    phasors: Vec<Vec<Complex64>>,
    markers: Vec<Cell>,
    data: Vec<Cell>,
    pub span: i64,
    fft: Arc<dyn Fft<f64>>,
    search_next: Option<i64>,
    hold: Hold<()>,
}

impl OfdmEngine {
    pub fn new(l: Level, other: &[u16], phasors: &Phasors) -> Self {
        let map = symbol_map(&l, other);
        let bins: Vec<usize> = l.bins().collect();
        let p: Vec<Vec<Complex64>> = (0..l.nsym()).map(|r| bins.iter().map(|&b| phasors[b][r]).collect()).collect();
        let q32 = |z: Complex64| Complex32::new(z.re as f32, z.im as f32);
        let (mut markers, mut data) = (Vec::new(), Vec::new());
        for (c, &bin) in bins.iter().enumerate() {
            for r in 1..l.nsym() {
                let q = p[r][c] * p[r - 1][c].conj();
                if map[c][r] {
                    markers.push(Cell { row: r, carrier: c, bin, reference: q32(q) });
                } else if !l.is_qam() {
                    data.push(Cell { row: r, carrier: c, bin, reference: q32(q.powi(4)) });
                }
            }
        }
        Self {
            record: l.record,
            l,
            map,
            phasors: p,
            markers,
            data: thin(data, MAX_DAT),
            span: l.nsym() as i64 * SYMLEN,
            fft: FFT.clone(),
            search_next: None,
            hold: Hold::new(SYM / 2, SYM * (l.nsym() as i64 - 2)),
        }
    }

    pub fn start_at(&mut self, j: i64) {
        self.search_next = Some(self.search_next.unwrap_or(j).max(j));
    }

    fn terms(&self, st: &Stft, j: i64) -> (f64, f64) {
        let mut a = Complex32::default();
        for m in &self.markers {
            a += st.diff.at(j + SYM * m.row as i64)[m.bin] * m.reference;
        }
        let mut b = Complex32::default();
        for d in &self.data {
            b += st.diff.at(j + SYM * d.row as i64)[d.bin].powi(4) * d.reference;
        }
        let marker_chi = f64::from(a.norm_sqr()) / self.markers.len() as f64;
        let data_chi = if self.data.is_empty() { 0.0 } else { f64::from(b.norm_sqr()) / self.data.len() as f64 };
        (marker_chi, data_chi)
    }

    pub fn step(&mut self, st: &Stft) -> Vec<Detection> {
        let need = SYM * (self.l.nsym() as i64 - 1);
        let hi = st.diff.end() - 1 - need - 2 * REFINE;
        let lo = self.search_next.unwrap_or(st.diff.base).max(st.diff.base);
        let mut out = Vec::new();
        if hi < lo {
            return out;
        }
        let marker_min = (MK_FRAC * self.markers.len() as f64).max(6.0);
        let terms = if hi - lo >= CATCH_UP {
            par_map(&(lo..=hi).collect::<Vec<_>>(), |&j| self.terms(st, j))
        } else {
            (lo..=hi).map(|j| self.terms(st, j)).collect()
        };
        for (j, (marker_chi, data_chi)) in (lo..=hi).zip(terms) {
            let chi = marker_chi + data_chi;
            let candidate = (marker_chi >= marker_min && chi >= CHI_MIN).then_some((chi, ()));
            if let Some((j, metric, ())) = self.hold.offer(j, candidate) {
                out.push(Detection { record: self.record, t: j * HOP, metric });
            }
        }
        self.search_next = Some(hi + 1);
        out
    }

    fn cells(&self, buf: &Track<Complex64>, t: i64, rows: usize) -> Vec<Vec<Complex64>> {
        let seg = buf.get(t, t + rows as i64 * SYMLEN);
        let fb: Vec<usize> = self.l.bins().collect();
        let mut w = vec![Complex64::default(); NF];
        let mut scratch = vec![Complex64::default(); self.fft.get_inplace_scratch_len()];
        seg.chunks(SYMLEN as usize)
            .enumerate()
            .map(|(r, ch)| {
                w.copy_from_slice(&ch[..NF]);
                self.fft.process_with_scratch(&mut w, &mut scratch);
                fb.iter().zip(&self.phasors[r]).map(|(&b, &p)| w[b] * p).collect()
            })
            .collect()
    }

    fn coherence(&self, c: &[Vec<Complex64>]) -> (f64, Complex64) {
        let s: Complex64 =
            self.markers.iter().map(|m| unit(c[m.row - 1][m.carrier].conj() * c[m.row][m.carrier])).sum();
        (s.norm() / self.markers.len() as f64, s)
    }

    pub fn early_coherence(&self, buf: &Track<Complex64>, t0: i64, rows: usize) -> Option<f64> {
        let rows = rows.min(self.l.nsym());
        if t0 + rows as i64 * SYMLEN + REFINE > buf.end() || t0 - REFINE < buf.base {
            return None;
        }
        let qam = self.l.is_qam();
        let cells: Vec<(usize, usize)> = (1..rows)
            .flat_map(|r| (0..self.l.nc()).map(move |c| (r, c)))
            .filter(|&(r, c)| !qam || self.map[c][r])
            .collect();
        (t0 - REFINE..=t0 + REFINE)
            .map(|t| {
                let c = self.cells(buf, t, rows);
                let s: Complex64 = cells
                    .iter()
                    .map(|&(r, k)| {
                        let d = c[r - 1][k].conj() * c[r][k];
                        unit(if qam { d } else { d.powi(4) })
                    })
                    .sum();
                s.norm() / cells.len().max(1) as f64
            })
            .max_by(f64::total_cmp)
    }

    pub fn decode(&self, buf: &Track<Complex64>, codec: &Codec, t0: i64, index: usize) -> Option<FrameRx> {
        if t0 + self.span + REFINE > buf.end() || t0 - REFINE < buf.base {
            return None;
        }
        let cand: Vec<(Vec<Vec<Complex64>>, f64, Complex64)> = (t0 - REFINE..=t0 + REFINE)
            .map(|t| {
                let c = self.cells(buf, t, self.l.nsym());
                let (m, s) = self.coherence(&c);
                (c, m, s)
            })
            .collect();
        let vals: Vec<f64> = cand.iter().map(|x| x.1).collect();
        let i = peak_mid(&vals, argmax(&vals), 0.02);
        let t = t0 - REFINE + i as i64;
        let (c, coh, s) = &cand[i];
        let (llr, esn0) = if self.l.is_qam() { self.qam_llr(c) } else { self.dpsk_llr(c, *s) };
        let out = codec.decode(self.record, &llr);
        let esn0_db = 10.0 * esn0.max(1e-6).log10();
        let nc = self.l.nc() as f64;
        Some(FrameRx {
            record: self.record,
            index,
            t,
            t_end: t - CP_MID + self.span,
            ok: out.ok,
            missing: false,
            bytes: out.bytes,
            esn0_db,
            snr_db: esn0_db + 10.0 * (nc * super::FS6 / NF as f64 / SN_BW_HZ).log10(),
            iterations: out.iterations,
            coherence: *coh,
        })
    }

    fn dpsk_llr(&self, c: &[Vec<Complex64>], s: Complex64) -> (Vec<f64>, f64) {
        let nc = self.l.nc();
        let rot = unit(s.conj());
        let mut d = eq::diff(c, c.len(), nc, rot);
        let nvar = eq::dpsk_nvar(&mut d, 1e-30, self.markers.iter().map(|m| (m.row - 1, m.carrier)));
        let cst = constellation(2);
        let mut llr = Vec::with_capacity(self.l.n());
        for b in 0..nc {
            let obs: Vec<Complex64> = (1..c.len()).filter(|&r| !self.map[b][r]).map(|r| d[r - 1][b]).collect();
            soft_bits(&obs, cst, nvar, &mut llr);
        }
        (llr, 2.0 / nvar)
    }

    fn qam_llr(&self, c: &[Vec<Complex64>]) -> (Vec<f64>, f64) {
        let (nc, nsy) = (self.l.nc(), self.l.nsym());
        let bins: Vec<usize> = self.l.bins().collect();
        let known = |b: usize, r: usize| qam_pilot(bins[b], r + 2) || self.map[b][r];
        let mut v = Complex64::default();
        for b in 0..nc {
            for r in 0..nsy.saturating_sub(PP) {
                if qam_pilot(bins[b], r + 2) {
                    v += c[r][b].conj() * c[r + PP][b];
                }
            }
        }
        let thd = v.arg() / PP as f64;
        let mut c = c.to_vec();
        eq::derotate(&mut c, thd);
        let cst = constellation(self.l.layout.bps);
        let es = cst.iter().map(|z| z.norm_sqr()).sum::<f64>() / cst.len() as f64;
        let mut e = eq::Eq::default();
        for b in 0..nc {
            let ks: Vec<usize> = (0..nsy).filter(|&r| known(b, r)).collect();
            let hs: Vec<Complex64> = ks.iter().map(|&r| c[r][b]).collect();
            for r in (0..nsy).filter(|&r| !known(b, r)) {
                e.push(c[r][b], r as f64, &ks, &hs);
            }
        }
        let mut llr = Vec::with_capacity(self.l.n());
        let nvar = e.llrs(cst, 1e-4, &mut llr);
        (llr, es / nvar)
    }
}
