use super::FS3;
use super::snr::SNR_REF_DB;
use crate::dsp::{Hold, Track, Window};
use crate::hf::ctrl::{COMB, Codeword, Fmt, family, family_comb};
use crate::hf::level::Bandwidth;
use num_complex::Complex64;

pub const HOP: i64 = 16;
pub const NS: usize = 128;
pub const ROWS: i64 = NS as i64 / HOP;
const COMB_TONES: usize = COMB as usize;
pub const NFFT: usize = 256;
pub const NBLK: usize = NFFT - 26;
pub const HB_HZ: f64 = FS3 / NFFT as f64;
pub const MIN_SCORE: f64 = 0.15;
const REF_2500_DB: f64 = 13.8;

pub(super) fn clamp_bin(i: i64, n: usize) -> usize {
    i.clamp(0, n as i64 - 1) as usize
}

#[derive(Clone, Debug)]
pub struct CtrlEntry {
    pub code: u32,
    pub fmt: Fmt,
    pub tag: usize,
    pub codeword: Codeword,
}

struct Entry {
    code: u32,
    tag: usize,
    tones: Vec<Vec<usize>>,
    blocks: Vec<Vec<usize>>,
    cells: Vec<(usize, i64, i64)>,
}

#[derive(Clone, Copy)]
struct Held {
    offset: i64,
    entry: usize,
    hits: usize,
}

struct Table {
    symbols: usize,
    tones: usize,
    min_hits: usize,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct CtrlDetection {
    pub code: u32,
    pub fmt: Fmt,
    pub hits: usize,
    pub tag: usize,
    pub t_end: i64,
    pub df_hz: f64,
    pub snr: f64,
    pub snr_norm: f64,
}

#[derive(Clone)]
pub(crate) struct Spectra {
    pub power: [f32; NFFT],
    pub comb_sum: [f32; NBLK],
    pub comb_max: [f32; NBLK],
}

impl Default for Spectra {
    fn default() -> Self {
        Self { power: [0.0; NFFT], comb_sum: [0.0; NBLK], comb_max: [0.0; NBLK] }
    }
}

pub(crate) struct Stft {
    win: Window,
    pub spec: Track<Spectra>,
    stft_next: i64,
}

impl Stft {
    pub fn new(keep: usize) -> Self {
        Self { win: Window::new(NFFT), spec: Track::new(keep), stft_next: 0 }
    }

    pub fn start_at(&mut self, start: i64) {
        self.stft_next = start.div_euclid(HOP);
    }

    pub fn update(&mut self, buf: &Track<Complex64>) {
        let j_hi = (buf.end() - NS as i64).div_euclid(HOP);
        self.stft_next = self.stft_next.max((buf.base + HOP - 1).div_euclid(HOP));
        if j_hi < self.stft_next {
            return;
        }
        let rows: Vec<Spectra> = (self.stft_next..=j_hi)
            .map(|j| {
                let seg = self.win.at(buf, j * HOP, NS);
                let mut r = Spectra { power: std::array::from_fn(|k| seg[k].norm_sqr() as f32), ..Spectra::default() };
                for h in 0..NBLK {
                    (r.comb_sum[h], r.comb_max[h]) = (0..COMB_TONES)
                        .map(|i| r.power[h + 2 * i])
                        .fold((0.0f32, f32::NEG_INFINITY), |(su, mx), v| (su + v, mx.max(v)));
                }
                r
            })
            .collect();
        self.spec.append(self.stft_next, rows);
        self.stft_next = j_hi + 1;
    }
}

pub struct CtrlDetector {
    bw: Bandwidth,
    tabs: Vec<Table>,
    stft: Stft,
    search_next: [Option<i64>; 3],
    holds: [Hold<Held>; 3],
}

impl CtrlDetector {
    pub fn new(bw: Bandwidth, entries: &[CtrlEntry]) -> Self {
        let mut d = Self {
            bw,
            tabs: Vec::new(),
            stft: Stft::new(ROWS as usize * 40),
            search_next: [None; 3],
            holds: Fmt::ALL.map(|f| Hold::new(2, 4 * (f.params().nsym as i64 - 1))),
        };
        d.set_entries(entries);
        d
    }

    pub fn set_entries(&mut self, entries: &[CtrlEntry]) {
        let old = std::mem::take(&mut self.tabs);
        self.tabs = Fmt::ALL
            .iter()
            .map(|&f| {
                let fm = f.params();
                let mut seen: Vec<(u32, Vec<Vec<usize>>)> = Vec::new();
                let mut list = Vec::new();
                let mut sorted: Vec<&CtrlEntry> = entries.iter().filter(|e| e.fmt == f).collect();
                sorted.sort_by_key(|e| e.code);
                for e in sorted {
                    let tones: Vec<Vec<usize>> = e
                        .codeword
                        .iter()
                        .map(|s| {
                            let mut v: Vec<usize> = s.iter().map(|&k| k as usize).collect();
                            v.sort_unstable();
                            v
                        })
                        .collect();
                    if seen.iter().any(|(c, seen_tones)| *c == e.code && *seen_tones == tones) {
                        continue;
                    }
                    seen.push((e.code, tones.clone()));
                    let comb0 = family_comb(family(e.code, self.bw)).1 as usize;
                    let blocks: Vec<Vec<usize>> = tones
                        .iter()
                        .map(|s| s.iter().map(|&k| comb0 + COMB_TONES * ((k - comb0) / COMB_TONES)).collect())
                        .collect();
                    let cells = (0..tones.len())
                        .flat_map(|k| (0..tones[k].len()).map(move |i| (k, i)))
                        .map(|(k, i)| (k, 2 * tones[k][i] as i64, 2 * blocks[k][i] as i64))
                        .collect();
                    list.push(Entry { code: e.code, tag: e.tag, tones, blocks, cells });
                }
                Table { symbols: fm.nsym as usize - 1, tones: fm.ntones, min_hits: fm.min_hits as usize, entries: list }
            })
            .collect();
        for (f, tb) in old.iter().enumerate() {
            let new = &self.tabs[f].entries;
            self.holds[f].retain(|x| {
                let e = &tb.entries[x.entry];
                match new.iter().position(|n| n.code == e.code && n.tones == e.tones) {
                    Some(i) => {
                        x.entry = i;
                        true
                    }
                    None => false,
                }
            });
        }
    }

    pub fn start_at(&mut self, start: i64) {
        self.stft.start_at(start);
        self.search_next = [None; 3];
    }

    fn score(&self, f: Fmt, j: i64, offset: i64, e: &Entry) -> (f64, usize, f64) {
        score(&self.stft.spec, &self.tabs[f.index()], j, offset, e)
    }

    pub fn step(&mut self, buf: &Track<Complex64>, linked: bool) -> Vec<CtrlDetection> {
        self.stft.update(buf);
        let max_offset = if linked { 3 } else { 10 };
        let mut found = Vec::new();
        for fmt in Fmt::ALL {
            let f = fmt.index();
            let tb = &self.tabs[f];
            if tb.entries.is_empty() {
                continue;
            }
            let lo_ok = self.stft.spec.base + ROWS * (tb.symbols as i64 - 1);
            let hi = self.stft.spec.end() - 1;
            let lo = self.search_next[f].unwrap_or(lo_ok).max(lo_ok);
            if hi < lo {
                continue;
            }
            for j in lo..=hi {
                let rows: Vec<&Spectra> =
                    (0..tb.symbols).map(|k| self.stft.spec.at(j - ROWS * (tb.symbols - 1 - k) as i64)).collect();
                let mut best: Option<(f64, Held)> = None;
                for offset in -max_offset..=max_offset {
                    for (index, e) in tb.entries.iter().enumerate() {
                        if !enough_hits(&rows, tb, offset, e) {
                            continue;
                        }
                        let (sc, hits, _) = score(&self.stft.spec, tb, j, offset, e);
                        if hits >= tb.min_hits && sc >= MIN_SCORE && best.is_none_or(|b| sc > b.0) {
                            best = Some((sc, Held { offset, entry: index, hits }));
                        }
                    }
                }
                if let Some(e) = self.holds[f].offer(j, best.filter(|b| b.0 > 0.0)) {
                    found.push((fmt, e));
                }
            }
            self.search_next[f] = Some(hi + 1);
        }
        found.into_iter().map(|(f, e)| self.emit(f, e)).collect()
    }

    fn emit(&self, f: Fmt, (j, _, x): (i64, f64, Held)) -> CtrlDetection {
        let tb = &self.tabs[f.index()];
        let (offset, e) = (x.offset, &tb.entries[x.entry]);
        let y: Vec<f64> = (offset - 1..=offset + 1).map(|trial| self.score(f, j, trial, e).2).collect();
        let den = y[0] - 2.0 * y[1] + y[2];
        let dlt = if den < 0.0 { 0.5 * (y[0] - y[2]) / den } else { 0.0 };
        let df = (offset as f64 + dlt.clamp(-0.5, 0.5)) * HB_HZ;
        let mut ratio = 0.0;
        for k in 0..tb.symbols {
            let row = self.stft.spec.at(j - ROWS * (tb.symbols - 1 - k) as i64);
            for i in 0..tb.tones {
                let et = f64::from(row.power[clamp_bin(2 * e.tones[k][i] as i64 + offset, NFFT)]);
                let bs = f64::from(row.comb_sum[clamp_bin(2 * e.blocks[k][i] as i64 + offset, NBLK)]);
                ratio += et / ((bs - et) / f64::from(COMB - 1) + 1e-30);
            }
        }
        ratio /= (tb.symbols * tb.tones) as f64;
        let snr =
            10.0 * ratio.max(1e-12).log10() + 10.0 * (tb.tones as f64).log10() + 10.0 * 0.5f64.log10() - REF_2500_DB;
        CtrlDetection {
            code: e.code,
            fmt: f,
            hits: x.hits,
            tag: e.tag,
            t_end: j * HOP + NS as i64,
            df_hz: df,
            snr,
            snr_norm: snr - (SNR_REF_DB - f64::from(f.params().k_db)),
        }
    }
}

fn enough_hits(rows: &[&Spectra], tb: &Table, offset: i64, e: &Entry) -> bool {
    let mut misses = (tb.symbols * tb.tones).saturating_sub(tb.min_hits);
    for &(k, t2, b2) in &e.cells {
        let row = rows[k];
        let v = row.power[clamp_bin(t2 + offset, NFFT)];
        if !(v >= row.comb_max[clamp_bin(b2 + offset, NBLK)] && v > 1e-9) {
            if misses == 0 {
                return false;
            }
            misses -= 1;
        }
    }
    true
}

fn score(spec: &Track<Spectra>, tb: &Table, j: i64, offset: i64, e: &Entry) -> (f64, usize, f64) {
    let (mut q, mut hits, mut en) = (0.0, 0, 0.0);
    for k in 0..tb.symbols {
        let row = spec.at(j - ROWS * (tb.symbols - 1 - k) as i64);
        for i in 0..tb.tones {
            let ht = clamp_bin(2 * e.tones[k][i] as i64 + offset, NFFT);
            let hb = clamp_bin(2 * e.blocks[k][i] as i64 + offset, NBLK);
            let v = row.power[ht];
            q += f64::from(v / (row.comb_sum[hb] + 1e-30));
            if v >= row.comb_max[hb] && v > 1e-9 {
                hits += 1;
            }
            en += f64::from(v);
        }
    }
    (q / (tb.symbols * tb.tones) as f64, hits, en)
}

pub fn rx_entries(cb: &crate::hf::ctrl::Codebook, tag: usize) -> Vec<CtrlEntry> {
    Fmt::ALL
        .into_iter()
        .flat_map(|fmt| {
            (1..=crate::hf::ctrl::NUM_CODES as u32).map(move |code| CtrlEntry {
                code,
                fmt,
                tag,
                codeword: cb.rx(fmt, code).clone(),
            })
        })
        .collect()
}
