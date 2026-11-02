pub use crate::link::Margins;
use crate::link::queue_hint_by;
use crate::num::{NO_SNR, dec1};
use crate::textcomp::fits_frames;
use xara_phy::fm::ctrl::code::{
    ACK_DOWN, ACK_FIRST_ONLY, ACK_SECOND_ONLY, ACK_STAY, ACK_UP1, ACK_UP2, ACK_UP3, ACK_UP6, NACK,
};
use xara_phy::fm::frame::payload_len;
use xara_phy::fm::level::{DataLevel, FmBandwidth, Record};

pub const SNR_OFFSET_DB: f64 = -3.5;

pub fn payload(bw: FmBandwidth, record: impl Into<Record>, vias: bool) -> usize {
    let record = record.into();
    payload_len(record, bw.level(record).frame_bytes(), vias).expect("level frame too short")
}

pub fn thr(bw: FmBandwidth, record: impl Into<Record>) -> f64 {
    dec1(bw.level(record).snr_thr_db)
}

pub fn wide_penalty(bw: FmBandwidth, record: impl Into<Record>) -> f64 {
    dec1(bw.level(record).wide_penalty_db)
}

pub fn decode_allow(bw: FmBandwidth, record: impl Into<Record>) -> i64 {
    i64::from(bw.level(record).decode_allow)
}

pub fn frame_ticks(bw: FmBandwidth, record: impl Into<Record>) -> i64 {
    i64::from(bw.level(record).frame_ticks())
}

pub fn qam_cap(bw: FmBandwidth) -> DataLevel {
    bw.levels().filter(|&l| bw.level(l).layout.bps < 7).max().unwrap_or(DataLevel::ONE)
}

pub fn as_level(record: Record) -> DataLevel {
    record.level().unwrap_or(DataLevel::ONE)
}

pub const HINT_MAX: u8 = 7;

fn hint_by(bw: FmBandwidth, record: Record, fits: impl Fn(Record, usize) -> bool) -> u8 {
    const UP: [(usize, u8); 4] = [(1, 4), (2, 5), (3, 6), (6, 7)];
    let i = if record == Record::Start { 1 } else { record.index() };
    let fits = |i, k| Record::from_index(i).is_some_and(|r| fits(r, k));
    queue_hint_by(i, bw.num_levels(), [(0, 2, &UP), (1, 3, &UP[..3])], HINT_MAX, fits)
}

pub fn queue_hint(bw: FmBandwidth, record: Record, q: usize, vias: bool) -> u8 {
    hint_by(bw, record, |l, k| q <= k * payload(bw, l, vias))
}

pub fn queue_hint_text(bw: FmBandwidth, record: Record, queued: &[u8], state: usize, vias: bool) -> u8 {
    hint_by(bw, record, |l, k| {
        let p = payload(bw, l, vias);
        fits_frames(queued, k, p, p, state)
    })
}

#[derive(Clone, Debug)]
pub struct Adaptation {
    hist: [f64; 4],
    pub n_good: usize,
    pub climb_fail: bool,
    pub max_level: usize,
    pub prev_max: usize,
    pub good: u32,
    pub lost: u32,
}

const MARGIN: [f64; 4] = [1.0, 0.5, 0.0, 0.0];

impl Default for Adaptation {
    fn default() -> Self {
        Self { hist: [f64::from(NO_SNR); 4], n_good: 0, climb_fail: false, max_level: 0, prev_max: 0, good: 0, lost: 0 }
    }
}

impl Adaptation {
    pub fn ratio(&self) -> f64 {
        if self.lost == 0 { 100.0 } else { f64::from(self.good) / f64::from(self.lost) }
    }

    pub fn frame_ok(&mut self, snr: f64, l: DataLevel, n: usize) -> f64 {
        let l = if l.get() > n { 1 } else { l.get() };
        self.hist.rotate_left(1);
        self.hist[3] = snr;
        if self.n_good < 4 {
            self.n_good += 1;
        }
        if self.max_level < l {
            self.max_level = l;
            if l > 1 {
                self.prev_max = l - 1;
            }
        }
        self.good += 1;
        self.held()
    }

    pub fn held(&self) -> f64 {
        self.hist.iter().copied().fold(f64::INFINITY, f64::min)
    }

    pub fn frame_lost(&mut self) {
        self.climb_fail = true;
        self.lost += 1;
    }
}

pub const STEPS: [usize; 5] = [0, 1, 2, 3, 6];
const PEN: [f64; 5] = [0.5, 0.5, 1.0, 2.0, 3.0];

pub fn margins(bw: FmBandwidth, ad: &Adaptation, l: DataLevel, s: f64) -> Margins {
    let n = bw.num_levels();
    let l = if l.get() > n { DataLevel::ONE } else { l };
    let a = if ad.n_good > 0 { MARGIN[ad.n_good - 1] } else { MARGIN[3] };
    let mut m = [0.0; 5];
    for (i, &d) in STEPS.iter().enumerate() {
        let room = match d {
            0 => true,
            6 => l.get() + 5 < n,
            _ => l.get() + d - 1 < n,
        };
        let v = match l.up(d).filter(|_| room) {
            Some(up) => {
                let pen = if d > 0 { wide_penalty(bw, up) - wide_penalty(bw, l) } else { 0.0 };
                s - thr(bw, up) - PEN[i] - pen
            }
            None => 0.0,
        };
        m[i] = v - a;
    }
    if ad.climb_fail {
        m[2] = 0.0;
        m[3] = 0.0;
        m[4] = 0.0;
    }
    if bw == FmBandwidth::Narrow {
        let cap = qam_cap(bw);
        for i in 2..5 {
            if l.get() + STEPS[i] > cap.get() {
                m[i] = 0.0;
            }
        }
    }
    let l = l.get();
    if l < n {
        let x = ad.ratio();
        let block = (x < 12.0 && l >= ad.max_level)
            || (l + 1 == ad.max_level && x < 10.0)
            || (l + 1 == ad.prev_max && x < 8.0)
            || x < 6.0;
        if block {
            for v in &mut m[1..] {
                *v = 0.0;
            }
        }
    }
    m
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decision {
    pub code: u32,
    pub level: Record,
    pub stay_reset: bool,
    pub held: f64,
    pub m: Margins,
}

const UNREGISTERED_TOP: DataLevel = DataLevel::new(2).unwrap();

pub fn max_level(bw: FmBandwidth, registered: bool) -> DataLevel {
    if registered { bw.top_level() } else { UNREGISTERED_TOP }
}

#[derive(Clone, Copy, Debug)]
pub struct Heard {
    pub record: Record,
    pub hint: u8,
    pub complete: bool,
}

pub fn frame_ok_and_decide(bw: FmBandwidth, ad: &mut Adaptation, h: Heard, snr: f64, registered: bool) -> Decision {
    let s = ad.frame_ok(snr, as_level(h.record), bw.num_levels());
    decide(bw, ad, h, s, registered)
}

pub fn decide(bw: FmBandwidth, ad: &Adaptation, h: Heard, s: f64, registered: bool) -> Decision {
    let Heard { record, hint, complete } = h;
    let top = max_level(bw, registered);
    let was15 = record == Record::Start;
    let l = as_level(record);
    let m = margins(bw, ad, l, s);
    let d = |code, level: Record, stay_reset| Decision { code, level, stay_reset, held: s, m };
    for (i, code, need) in [(4, ACK_UP6, 7), (3, ACK_UP3, 6), (2, ACK_UP2, 5), (1, ACK_UP1, 4)] {
        if let Some(up) = l.up(STEPS[i]).filter(|&up| complete && up <= top && m[i] > 0.0 && hint >= need) {
            return d(code, up.into(), true);
        }
    }
    if !complete {
        return d(ACK_SECOND_ONLY, l.down().into(), false);
    }
    if l > DataLevel::ONE && hint == 0 {
        return d(ACK_DOWN, l.down().into(), true);
    }
    if l > DataLevel::ONE && (hint == 1 || m[0] < 0.0) {
        return d(ACK_DOWN, l.down().into(), false);
    }
    d(ACK_STAY, if was15 { Record::Short } else { l.into() }, hint == 2)
}

pub fn sender_level_after(record: Record, code: u32) -> Record {
    let l = match record {
        Record::Start if code == ACK_STAY => return Record::Short,
        Record::Start | Record::Short => DataLevel::ONE,
        Record::Data(l) => l,
        Record::Connect | Record::Broadcast => return record,
    };
    let up = |d: usize| Record::from_index(l.get() + d).unwrap_or(Record::Data(DataLevel::MAX));
    match code {
        ACK_UP6 => up(6),
        ACK_UP3 => up(3),
        ACK_UP2 => up(2),
        ACK_UP1 => up(1),
        NACK | ACK_DOWN | ACK_SECOND_ONLY | ACK_FIRST_ONLY => l.down().into(),
        _ => l.into(),
    }
}

pub fn step_down(record: Record) -> DataLevel {
    as_level(record).down()
}

pub fn recommend_level(bw: FmBandwidth, ad: &Adaptation, registered: bool) -> Option<DataLevel> {
    let s = ad.held();
    let mut top = max_level(bw, registered);
    if bw == FmBandwidth::Narrow {
        top = top.min(qam_cap(bw));
    }
    let one = DataLevel::ONE;
    DataLevel::range(one, top).rev().find(|&l| s - thr(bw, l) - (wide_penalty(bw, l) - wide_penalty(bw, one)) > 3.0)
}
