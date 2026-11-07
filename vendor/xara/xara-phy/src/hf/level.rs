use crate::fec::{puncture_period, punctured_len};

use std::fmt;

pub const RECORDS: usize = 17;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Record(u8);

impl Record {
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(RECORDS as u8);

    pub const fn new(n: usize) -> Option<Self> {
        if n >= 1 && n <= Self::MAX.0 as usize { Some(Self(n as u8)) } else { None }
    }

    pub const fn get(self) -> usize {
        self.0 as usize
    }

    pub const fn index(self) -> usize {
        self.0 as usize - 1
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

    pub fn all() -> impl DoubleEndedIterator<Item = Self> {
        Self::range(Self::ONE, Self::MAX)
    }
}

impl Default for Record {
    fn default() -> Self {
        Self::ONE
    }
}

impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.get(), f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bandwidth {
    Hz500,
    Hz2300,
    Hz2750,
}

const PASSBAND_CENTRE_HZ: f64 = 1500.0;

impl Bandwidth {
    pub const ALL: [Bandwidth; 3] = [Bandwidth::Hz500, Bandwidth::Hz2300, Bandwidth::Hz2750];

    pub fn passband(self) -> (f64, f64) {
        let half = f64::from(self.hz()) / 2.0;
        (PASSBAND_CENTRE_HZ - half, PASSBAND_CENTRE_HZ + half)
    }

    pub const fn hz(self) -> u32 {
        match self {
            Bandwidth::Hz500 => 500,
            Bandwidth::Hz2300 => 2300,
            Bandwidth::Hz2750 => 2750,
        }
    }

    pub const fn num_levels(self) -> usize {
        match self {
            Bandwidth::Hz500 => 13,
            Bandwidth::Hz2300 => 16,
            Bandwidth::Hz2750 => 17,
        }
    }

    pub const fn top_level(self) -> Record {
        Record(self.num_levels() as u8)
    }

    pub fn levels(self) -> impl DoubleEndedIterator<Item = Record> {
        Record::range(Record::ONE, self.top_level())
    }

    pub fn table(self) -> &'static [Level; RECORDS] {
        match self {
            Bandwidth::Hz500 => &BW500,
            Bandwidth::Hz2300 => &BW2300,
            Bandwidth::Hz2750 => &BW2750,
        }
    }

    pub fn level(self, record: Record) -> &'static Level {
        &self.table()[record.index()]
    }

    pub fn amp_norm(self, record: Record) -> f64 {
        f64::from(
            match self {
                Bandwidth::Hz500 => AMP_NORM_500,
                Bandwidth::Hz2300 => AMP_NORM_2300,
                Bandwidth::Hz2750 => AMP_NORM_2750,
            }[record.index()],
        )
    }
}

impl std::str::FromStr for Bandwidth {
    type Err = String;

    fn from_str(b: &str) -> Result<Self, String> {
        Bandwidth::ALL.into_iter().find(|w| w.hz().to_string() == b).ok_or_else(|| format!("bad bandwidth {b}"))
    }
}

const AMP_NORM_500: [f32; RECORDS] =
    [1.41, 1.41, 1.41, 0.61, 0.61, 1.22, 4.9678, 4.9678, 4.9678, 4.9678, 4.9678, 4.21, 4.67, 0.61, 1.41, 4.67, 4.67];
const AMP_NORM_2300: [f32; RECORDS] = [
    1.41, 1.41, 1.41, 1.41, 0.75, 0.86, 1.73, 2.0, 2.23, 10.44435, 10.44435, 10.44435, 10.44435, 10.44435, 8.82035,
    10.01805, 4.67,
];
const AMP_NORM_2750: [f32; RECORDS] = [
    1.41, 1.41, 1.41, 1.41, 0.75, 0.86, 1.73, 2.0, 2.23, 2.5422, 11.4219, 11.4219, 11.4219, 11.4219, 11.4219, 9.6459,
    10.9557,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Waveform {
    Fsk { symbols: u32, sps: u32, first: u32, last: u32, step: u32 },
    Psk { spf: u32, markers: u32, carriers_hz: &'static [u32], pulse: Pulse },
    Ofdm { spf: u32, markers: u32, nc: u32, first: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pulse {
    Rc,
    Rrc { span: f32 },
}

impl Pulse {
    pub fn span(self) -> f32 {
        match self {
            Pulse::Rc => 0.5,
            Pulse::Rrc { span } => span,
        }
    }
}

const RC: Pulse = Pulse::Rc;
const RRC1: Pulse = Pulse::Rrc { span: 1.0 };
const RRC2: Pulse = Pulse::Rrc { span: 2.0 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    pub waveform: Waveform,
    pub bps: u32,
    pub rate: u32,
    pub snr_thr_db: f32,
    pub papr_db: f32,
    pub skip_on_step_down: bool,
    pub extra: bool,
    pub(crate) snr_corr_db: f32,
    pub k: u32,
}

pub const FSK_PILOTS: u32 = 24;

impl Level {
    pub const fn is_fsk(&self) -> bool {
        matches!(self.waveform, Waveform::Fsk { .. })
    }

    pub const fn is_qam(&self) -> bool {
        matches!(self.waveform, Waveform::Ofdm { .. }) && self.bps >= 4
    }

    pub const fn spf(&self) -> u32 {
        match self.waveform {
            Waveform::Fsk { symbols, .. } => symbols,
            Waveform::Psk { spf, .. } | Waveform::Ofdm { spf, .. } => spf,
        }
    }

    pub const fn sps(&self) -> u32 {
        match self.waveform {
            Waveform::Fsk { sps, .. } => sps,
            Waveform::Psk { .. } => crate::hf::psk::SPS as u32,
            Waveform::Ofdm { .. } => 64,
        }
    }

    pub const fn nc(&self) -> u32 {
        match self.waveform {
            Waveform::Fsk { .. } => 1,
            Waveform::Psk { carriers_hz, .. } => carriers_hz.len() as u32,
            Waveform::Ofdm { nc, .. } => nc,
        }
    }

    pub const fn markers(&self) -> u32 {
        match self.waveform {
            Waveform::Fsk { .. } => FSK_PILOTS,
            Waveform::Psk { markers, .. } | Waveform::Ofdm { markers, .. } => markers,
        }
    }

    pub const fn data_cells(&self) -> u32 {
        let spf = self.spf();
        match self.waveform {
            Waveform::Fsk { .. } => spf - 1 - FSK_PILOTS,
            _ if self.is_qam() => {
                self.nc() * (spf - 1 - self.markers() - ((spf - 3) / crate::hf::ofdm::PILOT_PERIOD + 1))
            }
            _ => self.nc() * (spf - 2 - self.markers()),
        }
    }

    pub const fn puncture_period(&self) -> u32 {
        puncture_period(self.rate)
    }

    pub const fn n(&self) -> u32 {
        self.data_cells() * self.bps
    }

    pub const fn other_len(&self) -> u32 {
        match self.waveform {
            Waveform::Fsk { symbols, .. } => 3 * (symbols - 1) + FSK_PILOTS,
            _ => self.nc() * (self.markers() + self.spf() - 1),
        }
    }

    pub const fn pad(&self) -> u32 {
        self.n() - punctured_len(self.k, puncture_period(self.rate))
    }

    pub const fn frame_bytes(&self) -> u32 {
        self.k / 8
    }

    pub const fn payload(&self) -> u32 {
        self.frame_bytes() - 3
    }

    pub const fn data_symbols(&self) -> u32 {
        self.spf() - 1
    }

    pub fn symbol_period(&self) -> f64 {
        let ofdm = matches!(self.waveform, Waveform::Ofdm { .. });
        f64::from(self.sps()) * if ofdm { crate::ofdm::SYM_RATIO } else { 1.0 } / 3000.0
    }

    pub fn lead_in(&self) -> u32 {
        let t = if self.bps < 4 { 0.080 } else { 0.120 };
        (t / self.symbol_period() - 1e-9).ceil() as u32
    }

    pub fn flush_symbols(&self) -> u32 {
        match self.waveform {
            Waveform::Psk { pulse, .. } => (2.0 * pulse.span()).round() as u32,
            _ => 0,
        }
    }

    pub fn airtime(&self) -> f64 {
        f64::from(self.lead_in() + self.data_symbols() + self.flush_symbols()) * self.symbol_period()
    }

    pub fn net_bps(&self) -> f64 {
        f64::from(8 * self.frame_bytes()) / (f64::from(self.data_symbols()) * self.symbol_period())
    }
}

pub fn dat_offsets(bw: Bandwidth) -> [u32; RECORDS] {
    let mut out = [0; RECORDS];
    let mut p = 0;
    for (o, l) in out.iter_mut().zip(bw.table()) {
        *o = p;
        p += l.n() + l.k + l.other_len();
    }
    out
}

pub fn qam_limit(bw: Bandwidth) -> Record {
    Record::all().filter(|&r| !bw.level(r).is_qam()).max().unwrap()
}

pub const fn start_level_max(bw: Bandwidth) -> Record {
    match bw {
        Bandwidth::Hz500 => Record(7),
        Bandwidth::Hz2300 | Bandwidth::Hz2750 => Record(6),
    }
}

struct Fsk {
    symbols: u32,
    bps: u32,
    rate: u32,
    sps: u32,
    first: u32,
    last: u32,
    step: u32,
    thr: f32,
    skip: bool,
    k: u32,
}

struct Psk {
    spf: u32,
    bps: u32,
    markers: u32,
    fc: &'static [u32],
    pulse: Pulse,
    thr: f32,
    papr: f32,
    skip: bool,
    k: u32,
}

struct Ofdm {
    spf: u32,
    bps: u32,
    rate: u32,
    markers: u32,
    nc: u32,
    first: u32,
    thr: f32,
    papr: f32,
    skip: bool,
    k: u32,
}

const FSK: Fsk = Fsk { symbols: 0, bps: 0, rate: 0, sps: 0, first: 0, last: 0, step: 0, thr: 0.0, skip: false, k: 0 };
const PSK: Psk = Psk { spf: 0, bps: 0, markers: 0, fc: &[], pulse: RC, thr: 0.0, papr: 0.0, skip: false, k: 0 };
const OFDM: Ofdm =
    Ofdm { spf: 150, bps: 0, rate: 0, markers: 0, nc: 0, first: 0, thr: 0.0, papr: 0.0, skip: false, k: 0 };
const O500: Ofdm = Ofdm { markers: 4, nc: 11, first: 27, ..OFDM };
const O2300: Ofdm = Ofdm { markers: 1, nc: 49, first: 8, ..OFDM };
const O2750: Ofdm = Ofdm { markers: 1, nc: 59, first: 3, ..OFDM };

const fn fsk(f: Fsk) -> Level {
    Level {
        waveform: Waveform::Fsk { symbols: f.symbols, sps: f.sps, first: f.first, last: f.last, step: f.step },
        bps: f.bps,
        rate: f.rate,
        snr_thr_db: f.thr,
        papr_db: 3.0,
        skip_on_step_down: f.skip,
        extra: false,
        snr_corr_db: 0.0,
        k: f.k,
    }
}

const fn psk(p: Psk) -> Level {
    Level {
        waveform: Waveform::Psk { spf: p.spf, markers: p.markers, carriers_hz: p.fc, pulse: p.pulse },
        bps: p.bps,
        rate: 3,
        snr_thr_db: p.thr,
        papr_db: p.papr,
        skip_on_step_down: p.skip,
        extra: false,
        snr_corr_db: 0.0,
        k: p.k,
    }
}

const fn ofdm(o: Ofdm) -> Level {
    Level {
        waveform: Waveform::Ofdm { spf: o.spf, markers: o.markers, nc: o.nc, first: o.first },
        bps: o.bps,
        rate: o.rate,
        snr_thr_db: o.thr,
        papr_db: o.papr,
        skip_on_step_down: o.skip,
        extra: false,
        snr_corr_db: 0.0,
        k: o.k,
    }
}

const fn extra(l: Level) -> Level {
    Level { extra: true, ..l }
}

const fn snr_corr(l: Level, snr_corr_db: f32) -> Level {
    Level { snr_corr_db, ..l }
}

const OFDM_500_X: Level =
    extra(snr_corr(ofdm(Ofdm { spf: 51, bps: 5, rate: 6, thr: 5.4, papr: 8.7, k: 1704, ..O500 }), -0.4));

static BW500: [Level; RECORDS] = [
    fsk(Fsk { symbols: 125, bps: 3, rate: 2, sps: 128, first: 54, last: 74, step: 2, thr: -13.6, k: 96, ..FSK }),
    fsk(Fsk { symbols: 229, bps: 3, rate: 2, sps: 64, first: 27, last: 37, step: 1, thr: -11.2, skip: true, k: 200 }),
    fsk(Fsk { symbols: 227, bps: 3, rate: 3, sps: 64, first: 27, last: 37, step: 1, thr: -10.4, k: 297, ..FSK }),
    psk(Psk { spf: 394, bps: 1, markers: 18, fc: &[1350, 1650], pulse: RC, thr: -8.4, papr: 4.4, k: 368, ..PSK }),
    psk(Psk { spf: 395, bps: 2, markers: 18, fc: &[1350, 1650], pulse: RC, thr: -5.4, papr: 4.4, k: 744, ..PSK }),
    psk(Psk {
        spf: 376,
        bps: 2,
        markers: 12,
        fc: &[1320, 1500, 1680],
        pulse: RRC2,
        thr: -3.8,
        papr: 7.2,
        k: 1080,
        ..PSK
    }),
    ofdm(Ofdm { bps: 2, rate: 3, thr: -1.2, papr: 8.3, k: 1578, ..O500 }),
    ofdm(Ofdm { bps: 2, rate: 4, thr: 0.6, papr: 8.3, k: 2104, ..O500 }),
    ofdm(Ofdm { bps: 2, rate: 6, thr: 2.6, papr: 8.3, k: 2524, ..O500 }),
    ofdm(Ofdm { bps: 3, rate: 4, thr: 5.0, papr: 8.2, skip: true, k: 3160, ..O500 }),
    ofdm(Ofdm { bps: 3, rate: 6, thr: 6.4, papr: 8.2, k: 3792, ..O500 }),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 4, rate: 7, thr: 8.0, papr: 8.2, k: 4755, ..O500 }), -1.59),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 5, rate: 6, thr: 10.6, papr: 8.2, k: 5708, ..O500 }), -0.4),
    extra(psk(Psk {
        spf: 451,
        bps: 2,
        markers: 18,
        fc: &[1350, 1650],
        pulse: RC,
        thr: -5.4,
        papr: 4.4,
        k: 856,
        ..PSK
    })),
    extra(fsk(Fsk { symbols: 93, bps: 3, rate: 2, sps: 128, first: 54, last: 74, step: 2, thr: -13.6, k: 64, ..FSK })),
    OFDM_500_X,
    OFDM_500_X,
];

static BW2300: [Level; RECORDS] = [
    fsk(Fsk { symbols: 125, bps: 3, rate: 2, sps: 128, first: 33, last: 96, step: 2, thr: -13.6, k: 96, ..FSK }),
    fsk(Fsk { symbols: 229, bps: 3, rate: 2, sps: 64, first: 17, last: 48, step: 2, thr: -11.2, k: 200, ..FSK }),
    fsk(Fsk { symbols: 229, bps: 4, rate: 3, sps: 64, first: 17, last: 48, step: 2, thr: -9.4, k: 402, ..FSK }),
    fsk(Fsk { symbols: 396, bps: 4, rate: 3, sps: 32, first: 9, last: 24, step: 1, thr: -6.6, k: 736, ..FSK }),
    psk(Psk { spf: 376, bps: 2, markers: 12, fc: &[900, 1500, 2100], pulse: RC, thr: -3.6, papr: 6.1, k: 1080, ..PSK }),
    psk(Psk {
        spf: 365,
        bps: 2,
        markers: 9,
        fc: &[750, 1250, 1750, 2250],
        pulse: RC,
        thr: -2.4,
        papr: 7.1,
        k: 1410,
        ..PSK
    }),
    psk(Psk {
        spf: 355,
        bps: 2,
        markers: 6,
        fc: &[700, 1020, 1340, 1660, 1980, 2300],
        pulse: RRC2,
        thr: -0.6,
        papr: 8.5,
        skip: true,
        k: 2076,
    }),
    psk(Psk {
        spf: 352,
        bps: 2,
        markers: 5,
        fc: &[702, 930, 1158, 1386, 1614, 1842, 2070, 2298],
        pulse: RRC2,
        thr: 0.4,
        papr: 8.6,
        skip: true,
        k: 2754,
    }),
    psk(Psk {
        spf: 349,
        bps: 2,
        markers: 4,
        fc: &[555, 765, 975, 1185, 1395, 1605, 1815, 2025, 2235, 2445],
        pulse: RRC1,
        thr: 1.3,
        papr: 8.9,
        k: 3424,
        ..PSK
    }),
    ofdm(Ofdm { bps: 2, rate: 3, thr: 5.4, papr: 9.4, k: 7197, ..O2300 }),
    ofdm(Ofdm { bps: 2, rate: 4, thr: 7.2, papr: 9.4, skip: true, k: 9596, ..O2300 }),
    ofdm(Ofdm { bps: 2, rate: 6, thr: 8.5, papr: 9.4, k: 11512, ..O2300 }),
    ofdm(Ofdm { bps: 3, rate: 4, thr: 11.2, papr: 9.4, k: 14398, ..O2300 }),
    snr_corr(ofdm(Ofdm { bps: 3, rate: 6, thr: 13.2, papr: 9.4, skip: true, k: 17276, ..O2300 }), -1.7),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 4, rate: 7, thr: 14.2, papr: 9.3, k: 21710, ..O2300 }), -1.59),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 5, rate: 6, thr: 17.2, papr: 9.3, k: 26056, ..O2300 }), -0.4),
    OFDM_500_X,
];

static BW2750: [Level; RECORDS] = [
    fsk(Fsk { symbols: 125, bps: 3, rate: 2, sps: 128, first: 24, last: 103, step: 2, thr: -13.6, k: 96, ..FSK }),
    fsk(Fsk { symbols: 229, bps: 3, rate: 2, sps: 64, first: 12, last: 51, step: 2, thr: -11.2, k: 200, ..FSK }),
    fsk(Fsk { symbols: 229, bps: 4, rate: 3, sps: 64, first: 12, last: 51, step: 2, thr: -9.4, k: 402, ..FSK }),
    fsk(Fsk { symbols: 396, bps: 4, rate: 3, sps: 32, first: 6, last: 25, step: 1, thr: -6.6, k: 736, ..FSK }),
    psk(Psk { spf: 376, bps: 2, markers: 12, fc: &[600, 1500, 2400], pulse: RC, thr: -3.6, papr: 6.1, k: 1080, ..PSK }),
    psk(Psk {
        spf: 365,
        bps: 2,
        markers: 9,
        fc: &[510, 1170, 1830, 2490],
        pulse: RC,
        thr: -2.4,
        papr: 7.1,
        k: 1410,
        ..PSK
    }),
    psk(Psk {
        spf: 355,
        bps: 2,
        markers: 6,
        fc: &[450, 870, 1290, 1710, 2130, 2550],
        pulse: RRC2,
        thr: -0.6,
        papr: 8.4,
        skip: true,
        k: 2076,
    }),
    psk(Psk {
        spf: 352,
        bps: 2,
        markers: 5,
        fc: &[380, 700, 1020, 1340, 1660, 1980, 2300, 2620],
        pulse: RRC2,
        thr: 0.4,
        papr: 8.7,
        skip: true,
        k: 2754,
    }),
    psk(Psk {
        spf: 349,
        bps: 2,
        markers: 4,
        fc: &[375, 625, 875, 1125, 1375, 1625, 1875, 2125, 2375, 2625],
        pulse: RRC1,
        thr: 1.3,
        papr: 8.8,
        skip: true,
        k: 3424,
    }),
    psk(Psk {
        spf: 349,
        bps: 2,
        markers: 3,
        fc: &[240, 450, 660, 870, 1080, 1290, 1500, 1710, 1920, 2130, 2340, 2550, 2760],
        pulse: RRC1,
        thr: 2.3,
        papr: 8.8,
        k: 4466,
        ..PSK
    }),
    ofdm(Ofdm { bps: 2, rate: 3, thr: 6.0, papr: 9.3, k: 8667, ..O2750 }),
    ofdm(Ofdm { bps: 2, rate: 4, thr: 8.0, papr: 9.3, skip: true, k: 11556, ..O2750 }),
    ofdm(Ofdm { bps: 2, rate: 6, thr: 9.3, papr: 9.4, k: 13864, ..O2750 }),
    ofdm(Ofdm { bps: 3, rate: 4, thr: 11.8, papr: 9.5, k: 17338, ..O2750 }),
    snr_corr(ofdm(Ofdm { bps: 3, rate: 6, thr: 14.0, papr: 9.5, skip: true, k: 20804, ..O2750 }), -1.7),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 4, rate: 7, thr: 15.0, papr: 9.3, k: 26145, ..O2750 }), -1.59),
    snr_corr(ofdm(Ofdm { spf: 155, bps: 5, rate: 6, thr: 18.0, papr: 9.3, k: 31376, ..O2750 }), -0.4),
];
