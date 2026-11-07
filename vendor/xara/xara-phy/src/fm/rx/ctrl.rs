use super::NF;
use crate::dsp::{Track, Window};
use crate::fm::ctrl::{BLOCK, CODE_BLOCK0, CODE_ROWS, Codebook, Codeword, LEAD_IN, LEAD_ROWS, NO_LEAD_ROW};
use num_complex::Complex64;

pub const HOP: i64 = 16;
pub const ROW: i64 = 8;
pub const MIN_HITS: usize = 18;
pub const NO_LEAD_HITS: usize = 22;
pub const HOLD: u32 = 4;
const TOP: usize = 64;
const SPAN: i64 = ROW * (CODE_ROWS as i64 - 1);
const KEEP: usize = 8 * (LEAD_ROWS + CODE_ROWS + 4);

#[derive(Clone, Debug)]
pub struct CtrlDetection {
    pub code: u32,
    pub hits: usize,
    pub lead_row: u8,
    pub t_end: i64,
}

#[derive(Clone)]
struct Spec {
    power: [f32; TOP],
    peak: [bool; TOP],
}

impl Default for Spec {
    fn default() -> Self {
        Self { power: [0.0; TOP], peak: [false; TOP] }
    }
}

#[derive(Clone, Copy)]
struct Best {
    energy: f64,
    j: i64,
    code: u32,
    hits: usize,
}

pub struct CtrlDetector {
    cb: Codebook,
    enabled: Vec<u32>,
    win: Window,
    spec: Track<Spec>,
    stft_next: i64,
    erased: i64,
    best: Option<Best>,
    hold: u32,
}

impl CtrlDetector {
    pub fn new(cb: Codebook, enabled: &[u32]) -> Self {
        Self {
            cb,
            enabled: enabled.to_vec(),
            win: Window::new(NF),
            spec: Track::new(KEEP),
            stft_next: 0,
            erased: i64::MIN,
            best: None,
            hold: 0,
        }
    }

    pub fn codebook(&self) -> &Codebook {
        &self.cb
    }

    pub fn set_codebook(&mut self, cb: Codebook) {
        self.cb = cb;
    }

    pub fn set_enabled(&mut self, enabled: &[u32]) {
        self.enabled = enabled.to_vec();
        self.best = None;
        self.hold = 0;
    }

    pub fn start_at(&mut self, m: i64) {
        self.stft_next = self.stft_next.max(m.div_euclid(HOP));
    }

    fn spectrum(&mut self, buf: &Track<Complex64>, j: i64) -> Spec {
        let seg = self.win.at(buf, j * HOP, NF);
        let power: [f32; TOP] = std::array::from_fn(|k| seg[k].norm_sqr() as f32);
        let mut peak = [false; TOP];
        for b in (CODE_BLOCK0..CODE_BLOCK0 + 7 * BLOCK).step_by(BLOCK as usize) {
            let blk = &power[b as usize..(b + BLOCK) as usize];
            let m = blk.iter().copied().fold(0.0f32, f32::max);
            for (i, &v) in blk.iter().enumerate() {
                peak[b as usize + i] = m > 0.0 && v == m;
            }
        }
        Spec { power, peak }
    }

    fn row(&self, j: i64) -> Option<&Spec> {
        (j > self.erased && j >= self.spec.base && j < self.spec.end()).then(|| self.spec.at(j))
    }

    fn correlate(&self, j: i64, cw: &Codeword) -> Option<(usize, f64)> {
        let (mut hits, mut energy) = (0, 0.0);
        for (r, tones) in cw.iter().enumerate() {
            let Some(s) = self.row(j - ROW * (CODE_ROWS - 1 - r) as i64) else { continue };
            for &k in tones {
                energy += f64::from(s.power[k as usize]);
                hits += usize::from(s.peak[k as usize]);
            }
        }
        (hits >= MIN_HITS).then_some((hits, energy))
    }

    fn lead_row(&self, j: i64) -> u8 {
        let (mut start, mut run) = (0, 0);
        for r in 1..=LEAD_ROWS {
            let jj = j - SPAN - ROW * (i64::from(NO_LEAD_ROW) - r as i64);
            let ok = self.row(jj).is_some_and(|s| LEAD_IN[r - 1].iter().all(|&k| s.peak[k as usize]));
            if ok {
                if start == 0 {
                    start = r;
                }
                run += 1;
                if run == 4 {
                    break;
                }
            } else {
                start = 0;
                run = 0;
            }
        }
        if start == 0 { NO_LEAD_ROW } else { start as u8 }
    }

    pub fn step(&mut self, buf: &Track<Complex64>) -> Vec<CtrlDetection> {
        let j_hi = (buf.end() - NF as i64).div_euclid(HOP);
        self.stft_next = self.stft_next.max((buf.base + HOP - 1).div_euclid(HOP));
        let mut out = Vec::new();
        while self.stft_next <= j_hi {
            let j = self.stft_next;
            let s = self.spectrum(buf, j);
            self.spec.append(j, [s]);
            self.stft_next += 1;
            for &c in &self.enabled {
                if let Some((hits, energy)) = self.correlate(j, self.cb.code(c)) {
                    if self.best.is_none_or(|b| energy >= b.energy) {
                        self.best = Some(Best { energy, j, code: c, hits });
                        self.hold = HOLD;
                    }
                }
            }
            if self.hold > 0 {
                self.hold -= 1;
            } else if let Some(Best { j: jb, code, hits, .. }) = self.best.take() {
                let lead_row = self.lead_row(jb);
                if lead_row > LEAD_ROWS as u8 && hits < NO_LEAD_HITS {
                    continue;
                }
                out.push(CtrlDetection { code, hits, lead_row, t_end: jb * HOP + NF as i64 });
                self.erased = j;
            }
        }
        out
    }
}
