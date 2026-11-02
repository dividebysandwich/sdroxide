use crate::fec::puncture_period;
use crate::ofdm::{BIN_HZ, SYM_RATIO};

use std::fmt;

pub const RECORDS: usize = 17;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DataLevel(u8);

impl DataLevel {
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(13);

    pub const fn new(n: usize) -> Option<Self> {
        if n >= 1 && n <= Self::MAX.0 as usize { Some(Self(n as u8)) } else { None }
    }

    pub const fn get(self) -> usize {
        self.0 as usize
    }

    pub fn up(self, d: usize) -> Option<Self> {
        Self::new(self.get() + d)
    }

    pub const fn down(self) -> Self {
        if self.0 > 1 { Self(self.0 - 1) } else { self }
    }

    pub fn clamped(n: usize, top: Self) -> Self {
        Self(n.clamp(1, top.get()) as u8)
    }

    pub fn range(lo: Self, hi: Self) -> impl DoubleEndedIterator<Item = Self> {
        (lo.0..=hi.0).map(Self)
    }
}

impl fmt::Debug for DataLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}

impl fmt::Display for DataLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.get(), f)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Record {
    #[default]
    Connect,
    Data(DataLevel),
    Short,
    Start,
    Broadcast,
}

impl Record {
    pub const fn index(self) -> usize {
        match self {
            Record::Connect => 0,
            Record::Data(l) => l.get(),
            Record::Short => 14,
            Record::Start => 15,
            Record::Broadcast => 16,
        }
    }

    pub const fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Record::Connect),
            14 => Some(Record::Short),
            15 => Some(Record::Start),
            16 => Some(Record::Broadcast),
            _ => match DataLevel::new(i) {
                Some(l) => Some(Record::Data(l)),
                None => None,
            },
        }
    }

    pub const fn data_level(self) -> Option<DataLevel> {
        match self {
            Record::Data(l) => Some(l),
            _ => None,
        }
    }

    pub const fn level(self) -> Option<DataLevel> {
        match self {
            Record::Data(l) => Some(l),
            Record::Short | Record::Start => Some(DataLevel::ONE),
            Record::Connect | Record::Broadcast => None,
        }
    }

    pub const fn keyed(self) -> bool {
        !matches!(self, Record::Connect | Record::Broadcast)
    }
}

impl From<DataLevel> for Record {
    fn from(l: DataLevel) -> Self {
        Record::Data(l)
    }
}

impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.index(), f)
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.index(), f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FmBandwidth {
    Narrow,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub nc: u32,
    pub first: u32,
    pub step: u32,
    pub bps: u32,
    pub spf: u32,
    pub markers: u32,
    pub k: u32,
}

impl Layout {
    pub const fn n(&self) -> u32 {
        if self.bps == 2 {
            self.nc * 2 * (self.spf - 2 - self.markers)
        } else {
            self.nc * self.bps * (self.spf - 1 - self.markers - ((self.spf - 3) / PILOT_PERIOD + 1))
        }
    }

    pub const fn other_len(&self) -> u32 {
        self.nc * (self.markers + self.spf - 1)
    }
}

impl FmBandwidth {
    pub fn layouts(self) -> &'static [Layout; RECORDS] {
        match self {
            FmBandwidth::Narrow => &FM_NARROW,
            FmBandwidth::Wide => &FM_WIDE,
        }
    }

    pub fn layout(self, r: Record) -> &'static Layout {
        &self.layouts()[r.index()]
    }

    pub const fn num_levels(self) -> usize {
        match self {
            FmBandwidth::Narrow => 11,
            FmBandwidth::Wide => 13,
        }
    }

    pub const fn top_level(self) -> DataLevel {
        DataLevel(self.num_levels() as u8)
    }

    pub fn levels(self) -> impl DoubleEndedIterator<Item = DataLevel> {
        DataLevel::range(DataLevel::ONE, self.top_level())
    }

    pub fn level(self, record: impl Into<Record>) -> Level {
        let record = record.into();
        let x = match self {
            FmBandwidth::Narrow => &NARROW[record.index()],
            FmBandwidth::Wide => &WIDE[record.index()],
        };
        let layout = *self.layout(record);
        let wide116 = self == FmBandwidth::Wide && layout.nc == NC116.nc;
        Level {
            bw: self,
            record,
            layout,
            rate: x.rate,
            papr_db: x.papr_db,
            amp_norm: x.amp_norm,
            snr_thr_db: x.thr,
            decode_allow: x.decode_allow,
            wide_penalty_db: if wide116 { 1.7 } else { 0.0 },
        }
    }
}

pub const MAX_SPF: u32 = {
    let (mut i, mut m) = (0, 0);
    while i < RECORDS {
        m = max(m, max(FM_NARROW[i].spf, FM_WIDE[i].spf));
        i += 1;
    }
    m
};

const fn max(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

const NC14: Layout = Layout { nc: 14, first: 10, step: 4, bps: 0, spf: 0, markers: 0, k: 0 };
const NC29: Layout = Layout { nc: 29, first: 10, step: 2, bps: 0, spf: 0, markers: 0, k: 0 };
const NC58: Layout = Layout { nc: 58, first: 10, step: 1, bps: 0, spf: 0, markers: 0, k: 0 };
const NC98: Layout = Layout { nc: 98, first: 6, step: 1, bps: 0, spf: 0, markers: 0, k: 0 };
const NC116: Layout = Layout { nc: 116, first: 6, step: 1, bps: 0, spf: 0, markers: 0, k: 0 };

static FM_NARROW: [Layout; RECORDS] = [
    Layout { bps: 2, spf: 29, markers: 4, k: 316, ..NC14 },
    Layout { bps: 2, spf: 200, markers: 4, k: 2710, ..NC14 },
    Layout { bps: 2, spf: 200, markers: 2, k: 5678, ..NC29 },
    Layout { bps: 2, spf: 200, markers: 1, k: 11420, ..NC58 },
    Layout { bps: 2, spf: 200, markers: 1, k: 15226, ..NC58 },
    Layout { bps: 3, spf: 201, markers: 1, k: 20408, ..NC58 },
    Layout { bps: 4, spf: 201, markers: 1, k: 27212, ..NC58 },
    Layout { bps: 5, spf: 201, markers: 1, k: 34018, ..NC58 },
    Layout { bps: 6, spf: 201, markers: 1, k: 40824, ..NC58 },
    Layout { bps: 6, spf: 201, markers: 1, k: 45927, ..NC58 },
    Layout { bps: 7, spf: 201, markers: 1, k: 53583, ..NC58 },
    Layout { bps: 8, spf: 192, markers: 1, k: 58455, ..NC58 },
    Layout { bps: 2, spf: 50, markers: 1, k: 2720, ..NC58 },
    Layout { bps: 2, spf: 50, markers: 1, k: 2720, ..NC58 },
    Layout { bps: 2, spf: 83, markers: 4, k: 1072, ..NC14 },
    Layout { bps: 2, spf: 18, markers: 6, k: 134, ..NC14 },
    Layout { bps: 2, spf: 69, markers: 4, k: 876, ..NC14 },
];

static FM_WIDE: [Layout; RECORDS] = [
    Layout { bps: 2, spf: 29, markers: 4, k: 316, ..NC14 },
    Layout { bps: 2, spf: 200, markers: 4, k: 2710, ..NC14 },
    Layout { bps: 2, spf: 200, markers: 2, k: 5678, ..NC29 },
    Layout { bps: 2, spf: 200, markers: 1, k: 11420, ..NC58 },
    Layout { bps: 2, spf: 200, markers: 1, k: 19300, ..NC98 },
    Layout { bps: 2, spf: 200, markers: 1, k: 25732, ..NC98 },
    Layout { bps: 3, spf: 201, markers: 1, k: 34488, ..NC98 },
    Layout { bps: 4, spf: 201, markers: 1, k: 45986, ..NC98 },
    Layout { bps: 4, spf: 201, markers: 1, k: 54434, ..NC116 },
    Layout { bps: 5, spf: 165, markers: 1, k: 55672, ..NC116 },
    Layout { bps: 6, spf: 138, markers: 1, k: 55672, ..NC116 },
    Layout { bps: 6, spf: 120, markers: 1, k: 54279, ..NC116 },
    Layout { bps: 7, spf: 102, markers: 1, k: 53583, ..NC116 },
    Layout { bps: 8, spf: 93, markers: 1, k: 55671, ..NC116 },
    Layout { bps: 2, spf: 83, markers: 4, k: 1072, ..NC14 },
    Layout { bps: 2, spf: 18, markers: 6, k: 134, ..NC14 },
    Layout { bps: 2, spf: 69, markers: 4, k: 876, ..NC14 },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    pub bw: FmBandwidth,
    pub record: Record,
    pub layout: Layout,
    pub rate: u32,
    pub papr_db: f32,
    pub amp_norm: f32,
    pub snr_thr_db: f32,
    pub decode_allow: u32,
    pub wide_penalty_db: f32,
}

struct Extra {
    rate: u32,
    papr_db: f32,
    amp_norm: f32,
    thr: f32,
    decode_allow: u32,
}

const C14: Extra = Extra { rate: 3, papr_db: 8.6, amp_norm: 5.29, thr: -0.4, decode_allow: 176 };

static NARROW: [Extra; RECORDS] = [
    C14,
    C14,
    Extra { rate: 3, papr_db: 9.3, amp_norm: 7.62, thr: 2.8, decode_allow: 198 },
    Extra { rate: 3, papr_db: 10.0, amp_norm: 10.77, thr: 5.6, decode_allow: 224 },
    Extra { rate: 4, papr_db: 10.0, amp_norm: 10.77, thr: 7.6, decode_allow: 245 },
    Extra { rate: 4, papr_db: 9.9, amp_norm: 9.4844, thr: 9.8, decode_allow: 300 },
    Extra { rate: 4, papr_db: 9.9, amp_norm: 9.05, thr: 11.8, decode_allow: 335 },
    Extra { rate: 4, papr_db: 9.9, amp_norm: 10.28, thr: 15.2, decode_allow: 393 },
    Extra { rate: 4, papr_db: 9.9, amp_norm: 11.2, thr: 17.4, decode_allow: 430 },
    Extra { rate: 5, papr_db: 9.9, amp_norm: 11.2, thr: 19.0, decode_allow: 465 },
    Extra { rate: 5, papr_db: 9.9, amp_norm: 11.24, thr: 21.8, decode_allow: 540 },
    Extra { rate: 5, papr_db: 9.9, amp_norm: 11.18, thr: 24.0, decode_allow: 550 },
    Extra { rate: 3, papr_db: 10.0, amp_norm: 10.77, thr: 5.6, decode_allow: 0 },
    Extra { rate: 3, papr_db: 10.0, amp_norm: 10.77, thr: 5.6, decode_allow: 0 },
    C14,
    C14,
    Extra { rate: 3, papr_db: 8.6, amp_norm: 5.29, thr: -0.4, decode_allow: 0 },
];

static WIDE: [Extra; RECORDS] = [
    C14,
    C14,
    Extra { rate: 3, papr_db: 9.3, amp_norm: 7.62, thr: 2.8, decode_allow: 198 },
    Extra { rate: 3, papr_db: 10.0, amp_norm: 10.77, thr: 5.6, decode_allow: 236 },
    Extra { rate: 3, papr_db: 10.3, amp_norm: 14.0, thr: 7.4, decode_allow: 289 },
    Extra { rate: 4, papr_db: 10.3, amp_norm: 14.0, thr: 9.4, decode_allow: 335 },
    Extra { rate: 4, papr_db: 10.3, amp_norm: 12.32448, thr: 12.0, decode_allow: 415 },
    Extra { rate: 4, papr_db: 10.2, amp_norm: 11.76, thr: 14.2, decode_allow: 463 },
    Extra { rate: 4, papr_db: 10.4, amp_norm: 12.79, thr: 15.0, decode_allow: 526 },
    Extra { rate: 4, papr_db: 10.3, amp_norm: 14.53, thr: 18.2, decode_allow: 551 },
    Extra { rate: 4, papr_db: 10.3, amp_norm: 15.83, thr: 20.4, decode_allow: 526 },
    Extra { rate: 5, papr_db: 10.3, amp_norm: 15.83, thr: 22.0, decode_allow: 500 },
    Extra { rate: 5, papr_db: 10.3, amp_norm: 15.9, thr: 24.8, decode_allow: 528 },
    Extra { rate: 5, papr_db: 10.3, amp_norm: 15.9, thr: 27.2, decode_allow: 520 },
    C14,
    C14,
    Extra { rate: 3, papr_db: 8.6, amp_norm: 5.29, thr: -0.4, decode_allow: 0 },
];

impl Level {
    pub const fn frame_bytes(&self) -> usize {
        self.layout.k as usize / 8
    }

    pub const fn n(&self) -> usize {
        self.layout.n() as usize
    }

    pub const fn nc(&self) -> usize {
        self.layout.nc as usize
    }

    pub const fn spf(&self) -> usize {
        self.layout.spf as usize
    }

    pub const fn nsym(&self) -> usize {
        self.layout.spf as usize - 1
    }

    pub const fn is_qam(&self) -> bool {
        self.layout.bps > 2
    }

    pub const fn puncture_period(&self) -> u32 {
        puncture_period(self.rate)
    }

    pub const fn bin(&self, c: usize) -> usize {
        (self.layout.first + self.layout.step * c as u32) as usize
    }

    pub fn bins(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.nc()).map(|c| self.bin(c))
    }

    pub const fn frame_ticks(&self) -> u32 {
        (self.layout.spf - 1) * 9
    }

    pub fn net_bps(&self) -> f64 {
        if self.record.data_level().is_none() {
            return self.bw.level(DataLevel::ONE).net_bps();
        }
        let num = (self.frame_bytes() as f64 * BIN_HZ * 8.0).round_ties_even();
        num / (SYM_RATIO * self.nsym() as f64)
    }

    pub fn gain(&self, drive_db: f64) -> f64 {
        gain(f64::from(self.papr_db), drive_db)
    }
}

pub fn gain(papr_db: f64, drive_db: f64) -> f64 {
    crate::dsp::db_amp(-(papr_db + 2.0 + (10.0 - papr_db) / 1000.0 - drive_db))
}

pub const PILOT_PERIOD: u32 = 9;

pub const fn qam_pilot(bin: usize, s: usize) -> bool {
    s % PILOT_PERIOD as usize == 2 + bin % 2
}
