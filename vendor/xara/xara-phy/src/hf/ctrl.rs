pub mod code;

use crate::dsp::sine;
use crate::hf::level::Bandwidth;
use crate::hf::tx;
use crate::rnd::VbRnd;
use std::f64::consts::SQRT_2;

pub const SYMBOL_SAMPLES: usize = 2048;
pub const NUM_CODES: usize = 58;
pub const ATU_SYMBOLS: usize = 70;
pub const ATU_SILENT_SYMBOLS: usize = 24;
pub const TONE_BIN: u32 = 64;
pub const COMB: u32 = 14;
pub const SEED_OFF: u32 = 10;
pub const SEED_OFF_REPLY_2750: u32 = 300;
pub const SEED_OFF_LEVEL: u32 = 1500;
pub const KISS_L1_SEED: u64 = 0x02D2_77B0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Format {
    pub nsym: u32,
    pub ntones: usize,
    pub(crate) lead_long_ms: f64,
    pub(crate) lead_short_ms: f64,
    pub min_hits: u32,
    pub snr_db: f32,
    pub k_db: f32,
}

const FORMATS: [Format; 3] = [
    Format { nsym: 32, ntones: 1, lead_long_ms: 40.0, lead_short_ms: 40.0, min_hits: 19, snr_db: -15.5, k_db: 3.0 },
    Format { nsym: 16, ntones: 1, lead_long_ms: 80.0, lead_short_ms: 40.0, min_hits: 13, snr_db: -7.5, k_db: 3.0 },
    Format { nsym: 8, ntones: 2, lead_long_ms: 160.0, lead_short_ms: 40.0, min_hits: 12, snr_db: -3.5, k_db: 6.0 },
];

impl Format {
    pub fn norm(&self) -> f64 {
        if self.ntones == 1 { SQRT_2 } else { 2.0 }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fmt {
    #[default]
    F0,
    F1,
    F2,
}

impl Fmt {
    pub const ALL: [Fmt; 3] = [Fmt::F0, Fmt::F1, Fmt::F2];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Fmt::F0),
            1 => Some(Fmt::F1),
            2 => Some(Fmt::F2),
            _ => None,
        }
    }

    pub const fn from_bits(r: u8) -> Self {
        match r {
            0 => Fmt::F0,
            1 => Fmt::F1,
            _ => Fmt::F2,
        }
    }

    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub const fn params(self) -> Format {
        FORMATS[self as usize]
    }

    pub const fn more_robust(self) -> Self {
        match self {
            Fmt::F0 | Fmt::F1 => Fmt::F0,
            Fmt::F2 => Fmt::F1,
        }
    }
}

impl TryFrom<u64> for Fmt {
    type Error = u64;
    fn try_from(v: u64) -> Result<Self, u64> {
        usize::try_from(v).ok().and_then(Fmt::from_index).ok_or(v)
    }
}

impl std::fmt::Debug for Fmt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.index())
    }
}

impl std::fmt::Display for Fmt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.index())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Answerer = 0,
    Caller = 1,
}

impl Role {
    pub const fn peer(self) -> Self {
        match self {
            Role::Answerer => Role::Caller,
            Role::Caller => Role::Answerer,
        }
    }
}

pub const fn book_off(sender: Role) -> u32 {
    match sender {
        Role::Answerer => 278,
        Role::Caller => 50,
    }
}

pub const fn connect_off(sender: Role) -> u32 {
    match sender {
        Role::Answerer => 1078,
        Role::Caller => 850,
    }
}

pub const fn req_seed_off(bw: Bandwidth) -> u32 {
    match bw {
        Bandwidth::Hz500 => 7,
        Bandwidth::Hz2300 => 0,
        Bandwidth::Hz2750 => 8,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Symbol {
    t: [u32; 2],
    n: u8,
}

impl Symbol {
    fn push(&mut self, k: u32) {
        self.t[usize::from(self.n)] = k;
        self.n += 1;
    }
}

impl std::ops::Deref for Symbol {
    type Target = [u32];
    fn deref(&self) -> &[u32] {
        &self.t[..usize::from(self.n)]
    }
}

impl<'a> IntoIterator for &'a Symbol {
    type Item = &'a u32;
    type IntoIter = std::slice::Iter<'a, u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<u32> for Symbol {
    fn from_iter<I: IntoIterator<Item = u32>>(it: I) -> Self {
        let mut s = Self::default();
        it.into_iter().for_each(|k| s.push(k));
        s
    }
}

pub type Codeword = Vec<Symbol>;

pub fn family(code: u32, bw: Bandwidth) -> u32 {
    match (code::connect_reply_q(code), code::connect_req_alias(code)) {
        (Some((b, _)), _) => bw_family(b),
        (_, Some((Bandwidth::Hz2750, _))) => bw_family(Bandwidth::Hz2300),
        (_, Some((b, _))) => bw_family(b),
        _ if code::KISS.contains(&code) || code == code::CQ => 1,
        _ => bw_family(bw),
    }
}

pub const fn bw_family(bw: Bandwidth) -> u32 {
    match bw {
        Bandwidth::Hz500 => 1,
        Bandwidth::Hz2300 => 2,
        Bandwidth::Hz2750 => 3,
    }
}

pub const fn family_comb(v: u32) -> (u32, u32) {
    match v {
        1 => (1, 50),
        2 => (4, 29),
        _ => (5, 22),
    }
}

pub fn gen_code(r: &mut VbRnd, fmt: Fmt, v: u32) -> Codeword {
    let (n, off) = family_comb(v);
    let f = fmt.params();
    (2..=f.nsym)
        .map(|s| {
            let mut bases = Symbol::default();
            while bases.len() < f.ntones {
                let b = r.rand_int(n) * COMB + off;
                if !bases.contains(&b) {
                    bases.push(b);
                }
            }
            bases.iter().map(|&b| b + u32::from(s % 2 == 0 && v != 1) + 2 * r.rand_int(6)).collect()
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codebook {
    pub bw: Bandwidth,
    codes: [[Vec<Codeword>; 3]; 2],
}

impl Codebook {
    pub fn new(seed_value: u32, mycall_hashes: [u32; 5], role: Role, bw: Bandwidth) -> Self {
        let side_off = [book_off(role.peer()), book_off(role)].map(u64::from);
        let seed = u64::from(seed_value);
        let mut codes: [[Vec<Codeword>; 3]; 2] = Default::default();
        let mut r = VbRnd::new();
        for (u, &so) in side_off.iter().enumerate() {
            for f in Fmt::ALL {
                let fo = f.index() as u64 + so;
                let book = &mut codes[u][f.index()];
                book.push(Vec::new());
                r.seed_rnd_wide(seed + u64::from(SEED_OFF) + fo);
                for c in 1..=NUM_CODES as u32 {
                    if let Some((b, k)) = code::connect_req_alias(c) {
                        let peer = if u == 1 && k == 0 { seed_value } else { mycall_hashes[k as usize] };
                        r.seed_rnd_wide(u64::from(peer) + fo + u64::from(req_seed_off(b)));
                    }
                    match c {
                        code::CONNECT_REPLY_2750 => r.seed_rnd_wide(seed + u64::from(SEED_OFF_REPLY_2750) + fo),
                        code::KISS_L2 => r.seed_rnd_wide(0xCD80A4 + fo),
                        code::CQ => r.seed_rnd_wide(0xA3CE37 + fo),
                        code::KISS_L4 => r.seed_rnd_wide(0x49E730 + fo),
                        _ if c == code::start_level_code(1) => r.seed_rnd_wide(seed + u64::from(SEED_OFF_LEVEL) + fo),
                        _ => {}
                    }
                    let v = if c == code::KISS_L1 { bw_family(bw) } else { family(c, bw) };
                    book.push(gen_code(&mut r, f, v));
                }
                r.seed_rnd_wide(KISS_L1_SEED + fo);
                book[code::KISS_L1 as usize] = gen_code(&mut r, f, family(code::KISS_L1, bw));
            }
        }
        Self { bw, codes }
    }

    pub fn set_connect_codes(&mut self, seed_value: u32, role: Role) {
        self.set_connect_codes_tx_rnd(seed_value, role);
    }

    pub fn set_connect_codes_tx_rnd(&mut self, seed_value: u32, role: Role) -> VbRnd {
        let side_off = [connect_off(role.peer()), connect_off(role)].map(u64::from);
        let mut r = VbRnd::new();
        for (u, &so) in side_off.iter().enumerate() {
            for f in Fmt::ALL {
                r.seed_rnd_wide(f.index() as u64 + u64::from(seed_value) + so);
                for c in [code::BREAK, code::IDLE] {
                    self.codes[u][f.index()][c as usize] = gen_code(&mut r, f, family(c, self.bw));
                }
            }
        }
        r
    }

    pub fn rx(&self, fmt: Fmt, code: u32) -> &Codeword {
        &self.codes[0][fmt.index()][code as usize]
    }

    pub fn tx(&self, fmt: Fmt, code: u32) -> &Codeword {
        &self.codes[1][fmt.index()][code as usize]
    }
}

pub fn leadin_grids() -> [Vec<Symbol>; 3] {
    let mut r = VbRnd::new();
    FORMATS.map(|f| {
        (0..100)
            .map(|_| {
                let mut ts = Symbol::default();
                while ts.len() < f.ntones {
                    let x = r.rand_int(34) + 50;
                    if !ts.contains(&x) {
                        ts.push(x);
                    }
                }
                ts
            })
            .collect()
    })
}

pub fn leadin_symbols(fmt: Fmt, code: u32, extra: u32) -> u32 {
    let f = fmt.params();
    let ms = if code::LONG_LEADIN.contains(&code) { f.lead_long_ms } else { f.lead_short_ms };
    (ms / (SYMBOL_SAMPLES as f64 / 48.0)).ceil() as u32 + extra
}

pub fn synth(lead: &[Symbol], codeword: &[Symbol]) -> Vec<f64> {
    lead.iter()
        .chain(codeword)
        .flat_map(|tones| {
            let rows: Vec<&[f64]> = tones.iter().map(|&k| sine(k, SYMBOL_SAMPLES)).collect();
            (0..SYMBOL_SAMPLES).map(move |n| rows.iter().map(|r| r[n]).sum::<f64>() / tones.len() as f64)
        })
        .collect()
}

pub fn gain(fmt: Fmt, drive_db: f64) -> f64 {
    tx::gain(f64::from(fmt.params().k_db), drive_db)
}

pub fn frame_audio(grids: &[Vec<Symbol>; 3], cb: &Codebook, fmt: Fmt, code: u32, extra: u32) -> Vec<f64> {
    let f = fmt.params();
    let scale = 2.0 * f.ntones as f64 / f.norm();
    synth(&grids[fmt.index()][..leadin_symbols(fmt, code, extra) as usize], cb.tx(fmt, code))
        .iter()
        .map(|v| v * scale)
        .collect()
}

pub fn atu_tone(fmt: Fmt) -> Vec<f64> {
    let tone = synth(&[[TONE_BIN].into_iter().collect(); ATU_SYMBOLS], &[]);
    let scale = 1.0 / fmt.params().norm();
    tone.iter().map(|v| v * scale).chain(std::iter::repeat_n(0.0, ATU_SILENT_SYMBOLS * SYMBOL_SAMPLES)).collect()
}
