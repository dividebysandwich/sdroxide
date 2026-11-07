use super::receiver::{Ctx, Decoded, Detection, Hint};
use super::{FS3, snr};
use crate::constellation::constellation;
use crate::decode::FrameDecoder;
use crate::demap::soft_bits;
use crate::dsp::{CHI_MIN, Hold, Track, argmax, thin, unit};
use crate::eq::dpsk_nvar;
use crate::hf::level::{Bandwidth, Level, Record, Waveform};
use crate::hf::psk::{pulse_taps, stagger, symbol_map};
use num_complex::Complex64;
use std::f64::consts::PI;

const SPS: i64 = crate::hf::psk::SPS as i64;
const STEP: i64 = 2;
const MAX_DAT: usize = 1500;

struct Carrier {
    fc: f64,
    stagger: i64,
    phasors: Vec<Complex64>,
    pilots: Vec<usize>,
    data: Vec<usize>,
    data_all: Vec<usize>,
    pilot_ref: Vec<Complex64>,
    data_ref: Vec<Complex64>,
    marker_syms: Vec<usize>,
    filtered: Track<Complex64>,
    diff: Track<Complex64>,
    diff_pow: Track<Complex64>,
}

pub(crate) struct PskEngine {
    pub record: Record,
    l: &'static Level,
    nsym: usize,
    order: i32,
    taps: Vec<f64>,
    delay: i64,
    pub span: i64,
    pub tail: i64,
    max_stagger: i64,
    pilot_count: usize,
    data_count: usize,
    carriers: Vec<Carrier>,
    pub(crate) filter_next: i64,
    search_next: Option<i64>,
    hold: Hold<()>,
}

impl PskEngine {
    pub fn new(bw: Bandwidth, record: Record, other: &[u16], phasors: &[Vec<Complex64>]) -> Self {
        let l = bw.level(record);
        let Waveform::Psk { carriers_hz, pulse, markers, .. } = l.waveform else { panic!("not a PSK level") };
        let nc = carriers_hz.len();
        let nsym = l.data_symbols() as usize;
        let order = 1i32 << l.bps;
        let map = symbol_map(l, other);
        let hp = pulse_taps(pulse);
        let hs: f64 = hp.iter().sum();
        let taps: Vec<f64> = hp.iter().map(|v| v / hs).collect();
        let delay = (taps.len() as i64 - 1) / 2;
        let pilot = |c: usize, s: usize| s >= 2 && map[c][s - 2];
        let ndat_all: usize = (0..nc).map(|c| (1..nsym).filter(|&s| !pilot(c, s)).count()).sum();
        let keep = (MAX_DAT as f64 / ndat_all.max(1) as f64).min(1.0);
        let max_stagger = (0..nc).map(|c| stagger(c, nc) as i64).max().unwrap_or(0);
        let carriers: Vec<Carrier> = carriers_hz
            .iter()
            .enumerate()
            .map(|(c, &fc)| {
                let p = phasors[c][..nsym].to_vec();
                let pilots: Vec<usize> = (1..nsym).filter(|&s| pilot(c, s)).collect();
                let data_all: Vec<usize> = (1..nsym).filter(|&s| !pilot(c, s)).collect();
                let n = ((data_all.len() as f64 * keep) as usize).max(1);
                let data = thin(data_all.clone(), n);
                let q = |s: usize| p[s] * p[s - 1].conj();
                Carrier {
                    fc: f64::from(fc),
                    stagger: stagger(c, nc) as i64,
                    pilot_ref: pilots.iter().map(|&s| q(s)).collect(),
                    data_ref: data.iter().map(|&s| q(s).powi(order)).collect(),
                    marker_syms: other[c * markers as usize..(c + 1) * markers as usize]
                        .iter()
                        .map(|&v| v as usize)
                        .collect(),
                    phasors: p,
                    pilots,
                    data,
                    data_all,
                    filtered: Track::new(nsym * SPS as usize + 4096),
                    diff: Track::new(nsym * SPS as usize + 4096),
                    diff_pow: Track::new(nsym * SPS as usize + 4096),
                }
            })
            .collect();
        let span = nsym as i64 * SPS;
        Self {
            record,
            l,
            nsym,
            order,
            delay,
            span,
            tail: max_stagger + delay,
            max_stagger,
            pilot_count: carriers.iter().map(|c| c.pilots.len()).sum(),
            data_count: carriers.iter().map(|c| c.data.len()).sum(),
            taps,
            carriers,
            filter_next: 0,
            search_next: None,
            hold: Hold::new(8, span - 4 * SPS),
        }
    }

    pub fn start_at(&mut self, start: i64) {
        self.filter_next = start + self.delay;
        for c in &mut self.carriers {
            c.filtered.restart(self.filter_next);
            c.diff.restart(self.filter_next);
            c.diff_pow.restart(self.filter_next);
        }
    }

    fn filter(&mut self, buf: &Track<Complex64>) {
        let m_hi = buf.end() - 1 - self.delay;
        if m_hi < self.filter_next {
            return;
        }
        if self.filter_next < buf.base + self.delay {
            self.start_at(buf.base);
        }
        let a = self.filter_next - self.delay;
        let seg = buf.get(a, m_hi + self.delay + 1);
        let order = self.order;
        for c in &mut self.carriers {
            let v: Vec<Complex64> = super::rot(a, seg.len(), c.fc).zip(&seg).map(|(r, &s)| r * s).collect();
            let y: Vec<Complex64> = (0..=(m_hi - self.filter_next) as usize)
                .map(|i| v[i..i + self.taps.len()].iter().zip(&self.taps).map(|(&x, &t)| x * t).sum())
                .collect();
            c.filtered.append(self.filter_next, y);
            let diff: Vec<Complex64> = (self.filter_next..=m_hi)
                .map(|n| {
                    if n - SPS >= c.filtered.base {
                        unit(*c.filtered.at(n) * c.filtered.at(n - SPS).conj())
                    } else {
                        Complex64::default()
                    }
                })
                .collect();
            c.diff_pow.append(self.filter_next, diff.iter().map(|p| p.powi(order)));
            c.diff.append(self.filter_next, diff);
        }
        self.filter_next = m_hi + 1;
    }

    fn chi(&self, t: i64) -> (f64, Complex64) {
        let mut a = Complex64::default();
        let mut b = Complex64::default();
        for c in &self.carriers {
            let base = t + c.stagger;
            for (&s, &q) in c.pilots.iter().zip(&c.pilot_ref) {
                a += *c.diff.at(base + SPS * s as i64) * q;
            }
            for (&s, &q) in c.data.iter().zip(&c.data_ref) {
                b += *c.diff_pow.at(base + SPS * s as i64) * q;
            }
        }
        let mut chi = a.norm_sqr() / self.pilot_count as f64;
        if self.data_count > 0 {
            chi += b.norm_sqr() / self.data_count as f64;
        }
        (chi, a)
    }

    pub fn step(&mut self, ctx: &Ctx) -> Vec<Detection> {
        self.filter(ctx.buf);
        let y0 = &self.carriers[0].filtered;
        let lo_ok = y0.base + SPS;
        let hi = y0.end() - 1 - self.max_stagger - SPS * (self.nsym as i64 - 1);
        let mut lo = self.search_next.unwrap_or(lo_ok).max(lo_ok);
        lo += (-lo).rem_euclid(STEP);
        let mut out = Vec::new();
        if hi < lo {
            return out;
        }
        let mut t = lo;
        while t <= hi {
            let (chi, _) = self.chi(t);
            if let Some((t, _, ())) = self.hold.offer(t, (chi >= CHI_MIN).then_some((chi, ()))) {
                out.push(Detection { record: self.record, t, hint: Hint::Psk });
            }
            t += STEP;
        }
        self.search_next = Some(t);
        out
    }

    fn symbols(&self, buf: &Track<Complex64>, t: i64, df: f64) -> Vec<Vec<Complex64>> {
        let idx = |c: &Carrier, k: usize| t + c.stagger + SPS * k as i64;
        if df == 0.0 {
            return self
                .carriers
                .iter()
                .map(|c| (0..self.nsym).map(|k| *c.filtered.at(idx(c, k)) * c.phasors[k]).collect())
                .collect();
        }
        let a = t - self.delay - 1;
        let b = t + self.max_stagger + SPS * (self.nsym as i64 - 1) + self.delay + 1;
        let seg = buf.get(a, b);
        self.carriers
            .iter()
            .map(|c| {
                let v: Vec<Complex64> = super::rot(a, seg.len(), c.fc + df).zip(&seg).map(|(r, &s)| r * s).collect();
                (0..self.nsym)
                    .map(|k| {
                        let i = (idx(c, k) - self.delay - a) as usize;
                        let y: Complex64 = v[i..i + self.taps.len()].iter().zip(&self.taps).map(|(&x, &t)| x * t).sum();
                        y * c.phasors[k]
                    })
                    .collect()
            })
            .collect()
    }

    fn coherence(&self, z: &[Vec<Complex64>]) -> Complex64 {
        let mut acc = Complex64::default();
        for (c, zc) in self.carriers.iter().zip(z) {
            for &s in &c.pilots {
                acc += unit(zc[s] * zc[s - 1].conj());
            }
        }
        acc
    }

    pub fn raw_snr(&self, z: &[Vec<Complex64>]) -> f64 {
        let prods: Vec<Complex64> = self
            .carriers
            .iter()
            .zip(z)
            .flat_map(|(c, zc)| c.marker_syms.iter().map(move |&p| zc[p - 1].conj() * zc[p]))
            .collect();
        snr::phase_error_snr(&prods, self.l, snr::PSK_K_DB)
    }

    pub fn decode(&self, buf: &Track<Complex64>, dec: &FrameDecoder, t0: i64, index: usize) -> Option<Decoded> {
        let refine = 3;
        let y0 = &self.carriers[0].filtered;
        if t0 + self.max_stagger + SPS * (self.nsym as i64 - 1) + refine > y0.end() - 1 || t0 - refine - SPS < y0.base {
            return None;
        }
        let cands: Vec<(f64, Complex64)> = (t0 - refine..=t0 + refine).map(|t| self.chi(t)).collect();
        let i = argmax(&cands.iter().map(|c| c.0).collect::<Vec<_>>());
        let t = t0 - refine + i as i64;
        let df = cands[i].1.arg() * FS3 / (2.0 * PI * SPS as f64);
        let df = if df.abs() > 1.0 { df } else { 0.0 };
        let z = self.symbols(buf, t, df);
        let acc = self.coherence(&z);
        let r = unit(acc.conj());
        let df_tot = df - r.arg() * FS3 / (2.0 * PI * SPS as f64);
        let mut ds: Vec<Vec<Complex64>> =
            z.iter().map(|zc| zc.windows(2).map(|w| w[1] * w[0].conj() * r).collect()).collect();
        let marks = self.carriers.iter().enumerate().flat_map(|(i, c)| c.pilots.iter().map(move |&s| (i, s - 1)));
        let nvar = dpsk_nvar(&mut ds, 0.0, marks);
        let cst = constellation(self.l.bps);
        let mut llr = Vec::with_capacity(self.l.n() as usize);
        for (c, d) in self.carriers.iter().zip(&ds) {
            let obs: Vec<Complex64> = c.data_all.iter().map(|&s| d[s - 1]).collect();
            soft_bits(&obs, cst, nvar, &mut llr);
        }
        let out = dec.decode(self.record, &llr);
        let raw = self.raw_snr(&z);
        Some(Decoded {
            record: self.record,
            index,
            t,
            t_end: t + self.max_stagger + self.span,
            ok: out.ok,
            bytes: out.bytes,
            raw_snr_db: raw,
            snr_db: snr::normalise(raw, self.l),
            snr_avg_db: None,
            afc_res: df_tot,
            iterations: out.iterations,
            metric: cands[i].0,
        })
    }
}
