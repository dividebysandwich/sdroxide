use super::receiver::{Ctx, Decoded, Detection, Hint};
use super::{FS3, rot, snr};
use crate::decode::FrameDecoder;
use crate::demap::energy_llrs;
use crate::dsp::{Hold, Track, Window, argmax, median};
use crate::hf::fsk::{GRAY, Grid};
use crate::hf::level::{Bandwidth, FSK_PILOTS, Level, Record};
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

const HOP: i64 = 4;
const MIN_HITS: usize = 14;
const FRACTIONS: [f64; 7] = [-0.375, -0.25, -0.125, 0.0, 0.125, 0.25, 0.375];

pub(crate) struct FskEngine {
    pub record: Record,
    l: &'static Level,
    grid: Grid,
    sps: usize,
    nrow: usize,
    tones: usize,
    pub span: i64,
    hop_rows: i64,
    binhz: f64,
    base: Vec<u16>,
    is_pilot: Vec<bool>,
    pilot_rows: Vec<usize>,
    pilot_bins: Vec<usize>,
    pilot_tones: Vec<Vec<usize>>,
    fft: Arc<dyn Fft<f64>>,
    win: Window,
    energy: Track<Vec<f64>>,
    pub(crate) stft_next: i64,
    pub(crate) search_next: i64,
    hold: Hold<i64>,
}

impl FskEngine {
    pub fn new(bw: Bandwidth, record: Record, other: &[u16]) -> Self {
        let l = bw.level(record);
        let grid = Grid::of(l);
        let (sps, nrow, tones) = (grid.sps as usize, grid.symbols - 1, 1usize << l.bps);
        let bin = |b: u16, t: u32| grid.bin(b, t) as usize;
        let base = other[..nrow].to_vec();
        let pairs = &other[nrow + FSK_PILOTS as usize..];
        let is_pilot: Vec<bool> = (0..nrow).map(|i| pairs[2 * i] != 0).collect();
        let pval: Vec<u16> = (0..nrow).map(|i| pairs[2 * i + 1]).collect();
        let pilot_rows: Vec<usize> = (0..nrow).filter(|&i| is_pilot[i]).collect();
        let pilot_bins = pilot_rows.iter().map(|&i| bin(base[i], u32::from(pval[i]))).collect();
        let pilot_tones = pilot_rows.iter().map(|&i| (0..tones as u32).map(|t| bin(base[i], t)).collect()).collect();
        let hop_rows = sps as i64 / HOP;
        Self {
            record,
            l,
            grid,
            sps,
            nrow,
            tones,
            span: (nrow * sps) as i64,
            hop_rows,
            binhz: FS3 / sps as f64,
            base,
            is_pilot,
            pilot_rows,
            pilot_bins,
            pilot_tones,
            fft: FftPlanner::new().plan_fft_forward(sps),
            win: Window::new(sps),
            energy: Track::new(nrow * hop_rows as usize + 64 * hop_rows as usize),
            stft_next: 0,
            search_next: 0,
            hold: Hold::new(4, (nrow as i64 - 2) * hop_rows),
        }
    }

    pub fn start_at(&mut self, start: i64) {
        self.stft_next = start.div_euclid(HOP);
        self.search_next = self.stft_next;
    }

    fn power(&self, seg: &mut [Complex64]) -> Vec<f64> {
        self.fft.process(seg);
        seg.iter().map(|v| v.norm_sqr()).collect()
    }

    fn stft(&mut self, buf: &Track<Complex64>) {
        let j_hi = (buf.end() - self.sps as i64).div_euclid(HOP);
        if j_hi < self.stft_next {
            return;
        }
        self.stft_next = self.stft_next.max((buf.base + HOP - 1).div_euclid(HOP));
        let rows: Vec<Vec<f64>> = (self.stft_next..=j_hi)
            .map(|j| self.win.at(buf, j * HOP, self.sps).iter().map(|v| v.norm_sqr()).collect())
            .collect();
        self.energy.append(self.stft_next, rows);
        self.stft_next = j_hi + 1;
    }

    fn metric(&self, j: i64, d: i64) -> (f64, usize) {
        let clip = |k: usize| (k as i64 + d).clamp(0, self.sps as i64 - 1) as usize;
        let (mut pe, mut ce, mut hits) = (0.0, 0.0, 0);
        for (n, &i) in self.pilot_rows.iter().enumerate() {
            let row = self.energy.at(j + i as i64 * self.hop_rows);
            let p = row[clip(self.pilot_bins[n])];
            let mut cmax = f64::NEG_INFINITY;
            for &k in &self.pilot_tones[n] {
                let v = row[clip(k)];
                ce += v;
                cmax = cmax.max(v);
            }
            pe += p;
            if p >= cmax && p > 1e-9 {
                hits += 1;
            }
        }
        (pe / (ce + 1e-30), hits)
    }

    pub fn step(&mut self, ctx: &Ctx) -> Vec<Detection> {
        self.stft(ctx.buf);
        let need = (self.nrow as i64 - 1) * self.hop_rows;
        let c_hi = self.energy.end() - 1 - need;
        let lo = self.search_next.max(self.energy.base);
        let dmax = ((ctx.search_hz / self.binhz).ceil() as i64).max(1);
        let mut out = Vec::new();
        if c_hi < lo {
            return out;
        }
        for j in lo..=c_hi {
            let mut best: Option<(f64, i64)> = None;
            for d in -dmax..=dmax {
                let (frac, hits) = self.metric(j, d);
                if hits >= MIN_HITS && best.is_none_or(|b| frac > b.0) {
                    best = Some((frac, d));
                }
            }
            if let Some((j, _, d)) = self.hold.offer(j, best.filter(|b| b.0 > 0.0)) {
                out.push(Detection { record: self.record, t: j * HOP, hint: Hint::Fsk(d) });
            }
        }
        self.search_next = c_hi + 1;
        out
    }

    fn rows_at(&self, buf: &Track<Complex64>, t: i64, shift_hz: f64) -> Vec<Vec<f64>> {
        let mut seg = buf.get(t, t + self.span);
        if shift_hz != 0.0 {
            for (v, r) in seg.iter_mut().zip(rot(t, self.span as usize, shift_hz)) {
                *v *= r;
            }
        }
        seg.chunks_mut(self.sps).map(|c| self.power(c)).collect()
    }

    fn pilot_frac(&self, rows: &[Vec<f64>]) -> f64 {
        let mut pe = 0.0;
        let mut ce = 0.0;
        for (n, &i) in self.pilot_rows.iter().enumerate() {
            pe += rows[i][self.pilot_bins[n]];
            ce += self.pilot_tones[n].iter().map(|&k| rows[i][k]).sum::<f64>();
        }
        pe / (ce + 1e-30)
    }

    fn tone(&self, row: usize, t: u32) -> usize {
        self.grid.bin(self.base[row], t) as usize
    }

    fn demap(&self, rows: &[Vec<f64>]) -> Vec<f64> {
        let below: Vec<f64> = rows.iter().flat_map(|r| r[..self.grid.first as usize].iter().copied()).collect();
        let noise = median(&below) + 1e-12;
        let mut llr = Vec::with_capacity(self.l.n() as usize);
        let mut m = vec![0.0; self.tones];
        for (r, row) in rows.iter().enumerate() {
            if self.is_pilot[r] {
                continue;
            }
            for (v, mv) in m.iter_mut().enumerate() {
                *mv = row[self.tone(r, GRAY[v])] / (noise * 10.0);
            }
            energy_llrs(&m, self.l.bps, &mut llr);
        }
        llr
    }

    pub fn raw_snr(&self, rows: &[Vec<f64>]) -> f64 {
        let ratios: Vec<f64> = self
            .pilot_rows
            .iter()
            .enumerate()
            .map(|(n, &i)| {
                let p = rows[i][self.pilot_bins[n]];
                let other =
                    (self.pilot_tones[n].iter().map(|&k| rows[i][k]).sum::<f64>() - p) / (self.tones - 1) as f64;
                if other > 0.0 { p / other } else { 0.0 }
            })
            .collect();
        10.0 * snr::lg(median(&ratios)) + 10.0 * snr::lg(64.0 / self.sps as f64) - snr::FSK_K_DB
    }

    pub fn decode(&self, buf: &Track<Complex64>, dec: &FrameDecoder, t0: i64, d: i64, index: usize) -> Option<Decoded> {
        let refine = HOP;
        if t0 + self.span + refine > buf.end() || t0 - refine < buf.base {
            return None;
        }
        let sh0 = d as f64 * self.binhz;
        let ts: Vec<i64> = (t0 - refine + 1..t0 + refine).collect();
        let vals: Vec<f64> = ts.iter().map(|&t| self.pilot_frac(&self.rows_at(buf, t, sh0))).collect();
        let t = ts[argmax(&vals)];
        let (mut metric, mut sh) = (f64::NEG_INFINITY, sh0);
        for f in FRACTIONS {
            let s = sh0 + f * self.binhz;
            let v = self.pilot_frac(&self.rows_at(buf, t, s));
            if v > metric || (v == metric && s > sh) {
                (metric, sh) = (v, s);
            }
        }
        let rows = self.rows_at(buf, t, sh);
        let llr = self.demap(&rows);
        let out = dec.decode(self.record, &llr);
        let raw = self.raw_snr(&rows);
        let norm = |r: f64| snr::normalise(r, self.l);
        let sv: f64 =
            [-2, -1, 1].iter().map(|&dd| norm(self.raw_snr(&self.rows_at(buf, t + dd, sh)))).sum::<f64>() + norm(raw);
        Some(Decoded {
            record: self.record,
            index,
            t,
            t_end: t + self.span,
            ok: out.ok,
            bytes: out.bytes,
            raw_snr_db: raw,
            snr_db: norm(raw),
            snr_avg_db: Some(sv / 4.0),
            afc_res: sh,
            iterations: out.iterations,
            metric,
        })
    }
}
