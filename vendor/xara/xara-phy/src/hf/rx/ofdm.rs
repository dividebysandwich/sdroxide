#![allow(clippy::needless_range_loop)]

use super::receiver::{Ctx, Decoded, Detection, Hint};
use super::{DEC, FS3, rot, snr};
use crate::constellation::constellation;
use crate::decode::FrameDecoder;
use crate::demap::soft_bits;
use crate::dsp::{CHI_MIN, Hold, Track, argmax, peak_mid, thin, unit};
use crate::eq;
use crate::hf::level::{Bandwidth, Level, Record, Waveform};
use crate::hf::ofdm::{PILOT_PERIOD, qam_pilot};
use crate::hf::psk::symbol_map;
use crate::ofdm::{NCP, NFFT};
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::sync::Arc;

const HOP: i64 = 4;
const NF: usize = NFFT / DEC as usize;
const SYMLEN: i64 = (NFFT + NCP) as i64 / DEC;
const SYM: i64 = SYMLEN / HOP;
const PP: usize = PILOT_PERIOD as usize;
const PILOT_ROWS: i64 = PILOT_PERIOD as i64;
const RESUME_WINDOW: i64 = (PILOT_ROWS + 1) * SYMLEN;
const GUARD: i64 = 8;
const MAX_DAT: usize = 2000;
pub const HYPOTHESES: [f64; 3] = [0.0, -23.4375, 23.4375];

pub(crate) struct Stft {
    fh: f64,
    fft: Arc<dyn Fft<f64>>,
    rows: VecDeque<Vec<Complex64>>,
    base: i64,
}

impl Stft {
    pub(crate) fn hypotheses() -> Vec<Stft> {
        HYPOTHESES
            .iter()
            .map(|&fh| Stft { fh, fft: FftPlanner::new().plan_fft_forward(NF), rows: VecDeque::new(), base: 0 })
            .collect()
    }

    fn end(&self) -> i64 {
        self.base + self.rows.len() as i64
    }

    fn row(&self, j: i64) -> &[Complex64] {
        &self.rows[(j - self.base) as usize]
    }

    fn compute(&self, buf: &Track<Complex64>, j: i64) -> Vec<Complex64> {
        let a = j * HOP;
        let mut seg = buf.get(a, a + NF as i64);
        if self.fh != 0.0 {
            for (v, r) in seg.iter_mut().zip(rot(a, NF, self.fh)) {
                *v *= r;
            }
        }
        self.fft.process(&mut seg);
        seg
    }

    pub(crate) fn update(&mut self, buf: &Track<Complex64>, (lo, hi): (i64, i64)) {
        if self.rows.is_empty() || lo > self.end() {
            self.rows.clear();
            self.base = lo;
        }
        while self.base > lo {
            let r = self.compute(buf, self.base - 1);
            self.rows.push_front(r);
            self.base -= 1;
        }
        for j in self.end()..=hi {
            let r = self.compute(buf, j);
            self.rows.push_back(r);
        }
    }

    pub(crate) fn keep_from(&mut self, j: Option<i64>) {
        let n = j.map_or(self.rows.len(), |j| (j - self.base).clamp(0, self.rows.len() as i64) as usize);
        self.rows.drain(..n);
        self.base += n as i64;
    }
}

struct Scan {
    f_base: i64,
    stft_next: i64,
    acc: VecDeque<(Complex64, Complex64)>,
    acc_base: i64,
    chi: Track<f64>,
}

impl Scan {
    fn new() -> Self {
        Self { f_base: 0, stft_next: 0, acc: VecDeque::new(), acc_base: 0, chi: Track::new(1024) }
    }

    fn restart(&mut self, j: i64) {
        self.f_base = j;
        self.stft_next = j;
        self.acc.clear();
        self.acc_base = j;
        self.chi.restart(j);
    }

    fn rows(&self, buf: &Track<Complex64>) -> (i64, i64) {
        let j0 = self.stft_next.max((buf.base + HOP - 1).div_euclid(HOP));
        (j0, (buf.end() - NF as i64).div_euclid(HOP))
    }
}

struct Group {
    off: i64,
    cells: Vec<(usize, Complex64)>,
}

fn groups(cells: &[Cell], off: impl Fn(usize) -> i64) -> Vec<Group> {
    let mut g: Vec<Group> = Vec::new();
    for c in cells {
        let o = off(c.s);
        match g.iter_mut().find(|g| g.off == o) {
            Some(g) => g.cells.push((c.b, c.q)),
            None => g.push(Group { off: o, cells: vec![(c.b, c.q)] }),
        }
    }
    g
}

#[derive(Clone, Copy)]
struct Cell {
    s: usize,
    b: usize,
    q: Complex64,
}

struct Fit {
    coh: f64,
    df: f64,
    c: Vec<Vec<Complex64>>,
    s: Complex64,
}

pub(crate) struct OfdmEngine {
    pub record: Record,
    l: &'static Level,
    nb: usize,
    fb: usize,
    spf: usize,
    nsy: usize,
    qam: bool,
    order: i32,
    map: Vec<Vec<bool>>,
    phasors: Vec<Vec<Complex64>>,
    markers: Vec<Cell>,
    data: Vec<Cell>,
    qam_pilots: Vec<Cell>,
    ga: Vec<Group>,
    gb: Vec<Group>,
    pub span: i64,
    fft: Arc<dyn Fft<f64>>,
    scans: Vec<Scan>,
    search_next: Option<i64>,
    hold: Hold<f64>,
}

impl OfdmEngine {
    pub fn new(bw: Bandwidth, record: Record, other: &[u16], phasors: &[Vec<Complex64>]) -> Self {
        let l = bw.level(record);
        let Waveform::Ofdm { first, nc, spf, .. } = l.waveform else { panic!("not an OFDM level") };
        let (fb, nb, spf) = (first as usize, nc as usize, spf as usize);
        let nsy = spf - 1;
        let qam = l.is_qam();
        let order = 1i32 << l.bps;
        let map = symbol_map(l, other);
        let p: Vec<Vec<Complex64>> = (0..nsy).map(|r| (0..nb).map(|bi| phasors[fb + bi][r]).collect()).collect();
        let marker = |bi: usize, s: usize| map[bi][s - 2];
        let mut markers = Vec::new();
        let mut data = Vec::new();
        let mut qam_pilots = Vec::new();
        for bi in 0..nb {
            for s in 3..=spf {
                let q = p[s - 2][bi] * p[s - 3][bi].conj();
                if marker(bi, s) {
                    markers.push(Cell { s, b: bi, q });
                } else if !qam {
                    data.push(Cell { s, b: bi, q: q.powi(order) });
                }
            }
            if qam {
                for s in 2..=spf {
                    if qam_pilot((fb + bi) as u32, s as u32) && s + PP <= spf {
                        qam_pilots.push(Cell { s, b: bi, q: p[s - 2 + PP][bi] * p[s - 2][bi].conj() });
                    }
                }
            }
        }
        let data = thin(data, MAX_DAT);
        let ga = groups(&markers, |s| s as i64 - 2);
        let gb = if qam { groups(&qam_pilots, |s| s as i64 + 6) } else { groups(&data, |s| s as i64 - 2) };
        Self {
            record,
            l,
            nb,
            fb,
            spf,
            nsy,
            qam,
            order,
            map,
            phasors: p,
            markers,
            data,
            qam_pilots,
            ga,
            gb,
            span: nsy as i64 * SYMLEN,
            fft: FftPlanner::new().plan_fft_forward(NF),
            scans: HYPOTHESES.iter().map(|_| Scan::new()).collect(),
            search_next: None,
            hold: Hold::new(2, SYM * (nsy as i64 - 2)),
        }
    }

    pub fn start_at(&mut self, start: i64) {
        for sc in &mut self.scans {
            if sc.stft_next * HOP < start - RESUME_WINDOW {
                sc.restart(start.div_euclid(HOP));
            }
        }
    }

    fn spectrum(&self, seg: &mut [Complex64]) -> Vec<Complex64> {
        self.fft.process(seg);
        seg[self.fb..self.fb + self.nb].to_vec()
    }

    pub(crate) fn rows_needed(&self, i: usize, buf: &Track<Complex64>) -> Option<(i64, i64)> {
        let sc = &self.scans[i];
        let (j0, j_hi) = sc.rows(buf);
        if j_hi < j0 {
            return None;
        }
        Some((if j0 != sc.stft_next { j0 } else { sc.f_base.max(j0 - PILOT_ROWS * SYM) }, j_hi))
    }

    pub(crate) fn rows_kept(&self, i: usize) -> i64 {
        let sc = &self.scans[i];
        sc.f_base.max(sc.stft_next - PILOT_ROWS * SYM)
    }

    fn update(&mut self, i: usize, buf: &Track<Complex64>, st: &Stft) {
        let (j0, j_hi) = self.scans[i].rows(buf);
        if j_hi < j0 {
            return;
        }
        if j0 != self.scans[i].stft_next {
            self.scans[i].restart(j0);
        }
        let (na, nb) =
            (self.markers.len() as f64, if self.qam { self.qam_pilots.len() } else { self.data.len() } as f64);
        let done = SYM * (self.spf as i64 - 2);
        let zero = Complex64::default();
        let bins = self.fb..self.fb + self.nb;
        let sc = &mut self.scans[i];
        for j in j0..=j_hi {
            let row = &st.row(j)[bins.clone()];
            let lagged = |lag: i64| -> Vec<Complex64> {
                if j - lag >= sc.f_base {
                    row.iter().zip(&st.row(j - lag)[bins.clone()]).map(|(&a, &b)| unit(a * b.conj())).collect()
                } else {
                    vec![zero; row.len()]
                }
            };
            let g1 = lagged(SYM);
            let gb = if self.qam { lagged(PILOT_ROWS * SYM) } else { g1.iter().map(|v| v.powi(self.order)).collect() };
            sc.acc.push_back((zero, zero));
            for (gs, g, second) in [(&self.ga, &g1, false), (&self.gb, &gb, true)] {
                for grp in gs {
                    let k = j - SYM * grp.off - sc.acc_base;
                    if k < 0 {
                        continue;
                    }
                    let v: Complex64 = grp.cells.iter().map(|&(b, q)| g[b] * q).sum();
                    let e = &mut sc.acc[k as usize];
                    if second { e.1 += v } else { e.0 += v }
                }
            }
            while sc.acc_base <= j - done {
                let (x, y) = sc.acc.pop_front().unwrap();
                sc.chi.append(sc.acc_base, [x.norm_sqr() / na + y.norm_sqr() / nb]);
                sc.acc_base += 1;
            }
        }
        sc.stft_next = j_hi + 1;
    }

    pub fn step(&mut self, ctx: &Ctx) -> Vec<Detection> {
        let nh = if ctx.linked { 1 } else { HYPOTHESES.len() };
        for i in 0..nh {
            self.update(i, ctx.buf, &ctx.ofdm[i]);
        }
        let scans = &self.scans[..nh];
        let hi = scans.iter().map(|s| s.chi.end()).min().unwrap_or(0) - 1;
        let lo_ok = scans.iter().map(|s| s.chi.base).max().unwrap_or(0) + if self.qam { PILOT_ROWS * SYM } else { SYM };
        let lo = self.search_next.unwrap_or(lo_ok).max(lo_ok);
        let mut out = Vec::new();
        if hi < lo {
            return out;
        }
        for j in lo..=hi {
            let (mut chi, mut fh) = (f64::NEG_INFINITY, 0.0);
            for (sc, &h) in scans.iter().zip(&HYPOTHESES) {
                let v = *sc.chi.at(j);
                if v > chi {
                    (chi, fh) = (v, h);
                }
            }
            if let Some((j, _, fh)) = self.hold.offer(j, (chi >= CHI_MIN).then_some((chi, fh))) {
                out.push(Detection { record: self.record, t: j * HOP, hint: Hint::Ofdm(fh) });
            }
        }
        self.search_next = Some(hi + 1);
        out
    }

    fn cells(&self, buf: &Track<Complex64>, t: i64, df: f64) -> Vec<Vec<Complex64>> {
        let mut seg = buf.get(t, t + self.span);
        if df != 0.0 {
            let n = seg.len();
            for (v, r) in seg.iter_mut().zip(rot(t, n, df)) {
                *v *= r;
            }
        }
        seg.chunks_mut(SYMLEN as usize)
            .enumerate()
            .map(|(r, c)| self.spectrum(&mut c[..NF]).iter().zip(&self.phasors[r]).map(|(&y, &p)| y * p).collect())
            .collect()
    }

    fn coherence(&self, c: &[Vec<Complex64>]) -> (f64, Complex64) {
        let s: Complex64 = self.markers.iter().map(|m| unit(c[m.s - 3][m.b].conj() * c[m.s - 2][m.b])).sum();
        let m = s.norm() / self.markers.len() as f64;
        if self.qam {
            let w: Complex64 = self.qam_pilots.iter().map(|p| unit(c[p.s - 2][p.b].conj() * c[p.s + 6][p.b])).sum();
            return (m + w.norm() / self.qam_pilots.len() as f64, s);
        }
        (m, s)
    }

    fn marker_products(&self, c: &[Vec<Complex64>]) -> Vec<Complex64> {
        self.markers.iter().map(|m| c[m.s - 3][m.b].conj() * c[m.s - 2][m.b]).collect()
    }

    pub fn decode(
        &self,
        buf: &Track<Complex64>,
        dec: &FrameDecoder,
        t0: i64,
        fh: f64,
        index: usize,
    ) -> Option<Decoded> {
        let refine = 3;
        if t0 + self.span + refine > buf.end() || t0 - refine < buf.base {
            return None;
        }
        let res: Vec<(f64, Complex64)> =
            (t0 - refine..=t0 + refine).map(|t| self.coherence(&self.cells(buf, t, fh))).collect();
        let vals: Vec<f64> = res.iter().map(|r| r.0).collect();
        let i = peak_mid(&vals, argmax(&vals), 0.02);
        let t = t0 - refine + i as i64;
        let w = 2.0 * PI * SYMLEN as f64 / FS3;
        let f0 = fh + res[i].1.arg() / w;
        let fit = |k: f64| {
            let df = f0 + k * FS3 / SYMLEN as f64;
            let c = self.cells(buf, t, df);
            let (coh, s) = self.coherence(&c);
            Fit { coh, df, c, s }
        };
        let mut best = fit(0.0);
        for k in [-1.0, 1.0] {
            let f = fit(k);
            if f.coh > best.coh {
                best = f;
            }
        }
        let Fit { coh, df, mut c, s } = best;
        let cst = constellation(self.l.bps);
        let mut llr = Vec::with_capacity(self.l.n() as usize);
        let df_tot;
        if !self.qam {
            let r = unit(s.conj());
            let mut d = eq::diff(&c, self.nsy, self.nb, r);
            let nvar = eq::dpsk_nvar(&mut d, 0.0, self.markers.iter().map(|m| (m.s - 3, m.b)));
            for bi in 0..self.nb {
                let obs: Vec<Complex64> =
                    (3..=self.spf).filter(|&s| !self.map[bi][s - 2]).map(|s| d[s - 3][bi]).collect();
                soft_bits(&obs, cst, nvar, &mut llr);
            }
            df_tot = df - r.arg() / w;
        } else {
            let v: Complex64 = self.qam_pilots.iter().map(|p| c[p.s - 2][p.b].conj() * c[p.s - 2 + PP][p.b]).sum();
            let thd = v.arg() / PP as f64;
            eq::derotate(&mut c, thd);
            df_tot = df + thd / w;
            let mut e = eq::Eq::default();
            for bi in 0..self.nb {
                let b = (self.fb + bi) as u32;
                let ps: Vec<usize> = (2..=self.spf).filter(|&s| qam_pilot(b, s as u32)).collect();
                let hp: Vec<Complex64> = ps.iter().map(|&s| c[s - 2][bi]).collect();
                let ds: Vec<usize> =
                    (2..=self.spf).filter(|&s| !qam_pilot(b, s as u32) && !self.map[bi][s - 2]).collect();
                for &s in &ds {
                    e.push(c[s - 2][bi], s as f64, &ps, &hp);
                }
            }
            e.llrs(cst, 1e-3, &mut llr);
        }
        let out = dec.decode(self.record, &llr);
        let raw = snr::phase_error_snr(&self.marker_products(&c), self.l, snr::OFDM_K_DB);
        Some(Decoded {
            record: self.record,
            index,
            t,
            t_end: t + self.span - GUARD,
            ok: out.ok,
            bytes: out.bytes,
            raw_snr_db: raw,
            snr_db: snr::normalise(raw, self.l),
            snr_avg_db: None,
            afc_res: df_tot,
            iterations: out.iterations,
            metric: coh,
        })
    }
}
