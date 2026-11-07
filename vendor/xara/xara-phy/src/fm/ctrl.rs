pub mod code;

use crate::call::call_hash;
use crate::rnd::VbRnd;
use crate::seq;
use std::f64::consts::PI;
use std::sync::LazyLock;

pub const SYMBOL: usize = 1024;
pub const CODE_ROWS: usize = 10;
pub const LEAD_ROWS: usize = 28;
pub const NO_LEAD_ROW: u8 = LEAD_ROWS as u8 + 1;
pub const TONE_AMP: f64 = 1.0 / 2.45;
const PAPR_DB: f64 = 7.45;
pub const BLOCK: u32 = 7;
pub const CODE_BLOCK0: u32 = 13;
pub const LEAD_BLOCK0: u32 = 27;

pub type Tones = [u32; 3];
pub type Codeword = [Tones; CODE_ROWS];

pub const TAMPER_KEY: u32 = 0x599;

fn rows<const N: usize>(r: &mut VbRnd, nblock: u32, block0: u32) -> [Tones; N] {
    std::array::from_fn(|_| {
        let mut bases = [u32::MAX; 3];
        let mut tones = [0; 3];
        for i in 0..3 {
            let b = loop {
                let b = r.rand_int(nblock - 1) * BLOCK + block0;
                if !bases[..i].contains(&b) {
                    break b;
                }
            };
            bases[i] = b;
            tones[i] = b + r.rand_int(6);
        }
        tones
    })
}

pub fn codes(key: u32) -> Vec<Codeword> {
    let mut r = VbRnd::new();
    r.seed_rnd_wide(u64::from(key) + 10);
    (0..22).map(|_| rows(&mut r, 7, CODE_BLOCK0)).collect()
}

pub fn connect_codes(key: u32) -> [Codeword; 2] {
    let mut r = VbRnd::new();
    r.seed_rnd_wide(u64::from(key) + 199);
    std::array::from_fn(|_| rows(&mut r, 7, CODE_BLOCK0))
}

pub static LEAD_IN: LazyLock<[Tones; LEAD_ROWS]> = LazyLock::new(|| {
    let mut r = VbRnd::stream(seq::FM_SEED);
    for _ in 0..seq::WHITENING_LEN {
        r.next_f32();
    }
    rows(&mut r, 3, LEAD_BLOCK0)
});

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Source,
    Target,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::Source, Side::Target];

    pub fn other(self) -> Side {
        match self {
            Side::Source => Side::Target,
            Side::Target => Side::Source,
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.idx())
    }
}

impl std::fmt::Debug for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codebook {
    pub key_d: u32,
    pub key_u: u32,
    codes: Vec<Codeword>,
}

impl Codebook {
    pub fn new(key_d: u32, key_u: u32) -> Self {
        let mut c = codes(key_d);
        c.extend(connect_codes(key_u));
        Self { key_d, key_u, codes: c }
    }

    pub fn for_link(upstream: &[u8], downstream: &[u8]) -> Self {
        Self::new(call_hash(downstream), call_hash(upstream))
    }

    pub fn code(&self, code: u32) -> &Codeword {
        &self.codes[code as usize - 1]
    }

    pub fn frame_rows(&self, code: u32, lead: usize) -> Vec<Tones> {
        assert!(lead <= LEAD_ROWS);
        LEAD_IN[LEAD_ROWS - lead..].iter().chain(self.code(code)).copied().collect()
    }
}

pub fn ctrl_gain(drive_db: f64) -> f64 {
    crate::fm::level::gain(PAPR_DB, drive_db)
}

pub fn synth(rows: &[Tones], drive_db: f64) -> Vec<f64> {
    static SIN: LazyLock<Vec<f64>> =
        LazyLock::new(|| (0..SYMBOL).map(|n| (2.0 * PI * n as f64 / SYMBOL as f64).sin()).collect());
    let g = 2.0 * TONE_AMP * ctrl_gain(drive_db);
    rows.iter()
        .flat_map(|t| {
            (0..SYMBOL)
                .map(move |n| (t.iter().map(|&k| SIN[(k as usize * n) % SYMBOL]).sum::<f64>() * g).clamp(-1.0, 1.0))
        })
        .collect()
}

pub fn frame_audio(cb: &Codebook, code: u32, lead: usize, drive_db: f64) -> Vec<f64> {
    synth(&cb.frame_rows(code, lead), drive_db)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeadIn {
    pub len: u8,
    max_missed: i32,
    reports: u32,
}

impl Default for LeadIn {
    fn default() -> Self {
        Self { len: LEAD_ROWS as u8, max_missed: -1, reports: 0 }
    }
}

impl LeadIn {
    pub fn new_session(&mut self) {
        self.reports = 0;
    }

    pub fn report(&mut self, row: u8) -> u8 {
        let missed = (i32::from(self.len) + i32::from(row) - i32::from(NO_LEAD_ROW)).clamp(0, LEAD_ROWS as i32);
        self.max_missed = self.max_missed.max(missed);
        self.reports += 1;
        if self.reports > 4 {
            return self.len;
        }
        let m = self.max_missed;
        let inc = if self.reports == 1 { ((f64::from(m) * 0.5).round_ties_even() as i32).max(1) } else { 1 };
        self.len = (m + inc).min(LEAD_ROWS as i32) as u8;
        self.len
    }

    pub fn data_symbols(&self) -> usize {
        data_lead_in(self.len)
    }
}

pub const fn data_lead_in(c: u8) -> usize {
    (8 * c as usize).div_ceil(9)
}
