pub use crate::link::Margins;
use crate::num::{NO_SNR, dec1};
use xara_phy::hf::ctrl::code::{ACK_DOWN, ACK_SECOND_ONLY, ACK_STAY, ACK_UP1, ACK_UP2, ACK_UP3, ACK_UP4, NACK_MISSED};
use xara_phy::hf::level::{Bandwidth, Level, Record, qam_limit};

pub const DISABLED: f64 = -100.0;
pub const SN_NORM_DB: f64 = 9.4;

pub fn step_thr(l: &Level) -> f64 {
    dec1(l.snr_thr_db) - (SN_NORM_DB - dec1(l.papr_db))
}

fn skip_down(bw: Bandwidth, lv: Record) -> Record {
    if lv > Record::ONE && bw.level(lv).skip_on_step_down { lv.down() } else { lv }
}

pub fn step_down(bw: Bandwidth, level: Record) -> Record {
    skip_down(bw, level.down())
}

#[derive(Clone, Debug)]
pub struct SnrTrend {
    prev: f64,
}

impl Default for SnrTrend {
    fn default() -> Self {
        Self { prev: f64::from(NO_SNR) }
    }
}

impl SnrTrend {
    pub fn push(&mut self, snr_db: f64) -> f64 {
        let out = if snr_db > self.prev { 0.5 * (self.prev + snr_db) } else { snr_db };
        self.prev = snr_db;
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qam {
    None,
    Received,
    LostFirst,
}

#[derive(Clone, Debug)]
pub struct Adaptation {
    bw: Bandwidth,
    pub margin: f64,
    pub margin_decay: f64,
    pub margin_prev: f64,
    pub ok_run: u32,
    pub qam: Qam,
    pub last: Record,
    pub failed_climbs: u32,
    pub last_good: Record,
}

impl Adaptation {
    pub fn new(bw: Bandwidth) -> Self {
        let mut s = Self {
            bw,
            margin: 0.0,
            margin_decay: 0.0,
            margin_prev: 0.0,
            ok_run: 0,
            qam: Qam::None,
            last: Record::ONE,
            failed_climbs: 0,
            last_good: Record::ONE,
        };
        s.reset();
        s
    }

    pub fn reset(&mut self) {
        (self.margin, self.margin_decay, self.margin_prev) = (2.0, 0.5 / 15.0, 0.0);
        (self.ok_run, self.qam, self.last, self.failed_climbs, self.last_good) =
            (1, Qam::None, Record::ONE, 0, Record::ONE);
    }

    pub fn frame_ok(&mut self, level: Record) {
        self.ok_run += 1;
        if self.bw.level(level).is_qam() {
            self.qam = Qam::Received;
        }
        self.margin = (self.margin - self.margin_decay).max(1.5);
        self.last_good = level;
        if self.last < level {
            self.failed_climbs = 0;
        }
        self.last = level;
    }

    pub fn frame_lost(&mut self, level: Record) {
        if self.last_good <= level && level > Record::ONE && self.ok_run > 0 {
            if self.qam == Qam::None && self.bw.level(level).is_qam() {
                self.qam = Qam::LostFirst;
            }
            self.margin_prev = self.margin;
            self.margin += 1.0;
            if self.last < level {
                self.margin += 4.0 * f64::from(self.failed_climbs);
            }
            if self.ok_run < 15 {
                self.margin += 3.0 * f64::from(15 - self.ok_run) / 15.0;
            }
            self.margin_decay = (self.margin - self.margin_prev) / 15.0;
        }
        self.ok_run = 0;
        if self.last < level {
            self.failed_climbs += 1;
        }
        self.last = level;
    }

    pub fn margins(&self, snr_db: f64, level: Record) -> Margins {
        let top = self.bw.top_level();
        let lim = qam_limit(self.bw);
        std::array::from_fn(|d| match level.up(d) {
            Some(l)
                if !(l > top
                    || (d >= 2 && (self.failed_climbs != 0 || l > lim))
                    || (d == 1 && l > lim && self.qam == Qam::LostFirst)) =>
            {
                snr_db - step_thr(self.bw.level(l)) - self.margin - d.saturating_sub(1) as f64
            }
            _ => DISABLED,
        })
    }

    pub fn level_from_snr(&mut self, snr_db: f64, hi: Record, margin: f64) -> Record {
        self.climb_from_snr(snr_db, hi, margin, |_| 0.0)
    }

    pub fn offer_level(&mut self, snr_db: f64, hi: Record) -> Record {
        let good = self.last_good.get();
        self.climb_from_snr(snr_db, hi, 3.0, |l| l.get().saturating_sub(good).saturating_sub(1).min(3) as f64)
    }

    fn climb_from_snr(&mut self, snr_db: f64, hi: Record, margin: f64, extra: impl Fn(Record) -> f64) -> Record {
        let m = self.margin.max(margin);
        self.margin = self.margin.max(2.0);
        let mut out = Record::ONE;
        for l in Record::range(Record::ONE, hi).skip(1) {
            if snr_db - step_thr(self.bw.level(l)) <= m + extra(l) {
                break;
            }
            out = l;
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub code: u32,
    pub level: Record,
    pub stay_reset: bool,
}

pub fn ack_decision(bw: Bandwidth, level: Record, m: &Margins, hint: u8, complete: bool, max: Record) -> Decision {
    let d = |code, level| Decision { code, level, stay_reset: false };
    for (s, code, need) in [(4, ACK_UP4, 6), (3, ACK_UP3, 5), (2, ACK_UP2, 4), (1, ACK_UP1, 3)] {
        if let Some(up) = level.up(s).filter(|&up| complete && up <= max && m[s] > 0.0 && hint >= need) {
            return d(code, up);
        }
    }
    if !complete {
        return d(ACK_SECOND_ONLY, step_down(bw, level));
    }
    if level > Record::ONE && (m[0] < 0.0 || hint == 0) {
        return d(ACK_DOWN, level.down());
    }
    Decision { code: ACK_STAY, level, stay_reset: hint == 2 }
}

pub(crate) fn step(code: u32) -> Option<i64> {
    match code {
        ACK_STAY => Some(0),
        ACK_UP1 => Some(1),
        ACK_UP2 => Some(2),
        ACK_UP3 => Some(3),
        ACK_UP4 => Some(4),
        ACK_DOWN => Some(-1),
        _ => None,
    }
}

pub(crate) fn step_count(code: u32, upsteps: usize, level: Record) -> usize {
    match upsteps {
        6.. => 3,
        4 | 5 => 2,
        _ if code == NACK_MISSED && level > Record::ONE => 2,
        _ => 1,
    }
}

pub(crate) fn step_down_n(bw: Bandwidth, level: Record, n: usize) -> Record {
    skip_down(bw, if n <= level.get() { Record::clamped(level.get() - n, level) } else { level })
}
