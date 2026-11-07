use crate::fm::adapt::{decode_allow, frame_ticks};
use crate::num::rint;
use crate::time::{SECOND, Time};
use std::collections::VecDeque;
use xara_phy::fm::level::{FmBandwidth, RECORDS, Record};

pub const TICK: Time = 128;
pub(crate) const CODE_GATE: i64 = 80;
pub const DETECT_HOLD: i64 = 4;
pub(crate) const IDLE_DATA_WAIT: i64 = 101;
pub(crate) const FQ_WAIT_REPLIES: i64 = 4;

const VIA_ACK_ADD: [f64; 3] = [250.0, 353.553_390_593_273_8, 433.012_701_892_219_3];
const VIA_CTRL_ADD: [f64; 3] = [75.0, 433.012_701_892_219_3, 559.016_994_374_947_4];
const VIA_SPAN: [f64; 3] = [246.499_999_999_999_97, 739.499_999_999_999_9, 1_232.499_999_999_999_8];
const VIA_GATES: [f64; 3] = [80.0, 160.0, 240.0];
const DIRECT_CONNECT_SPAN: f64 = 326.5;
const SYNC_SLACK: f64 = 520.0;
const ACTIVE_SCAN_SLACK: f64 = 60.0;
const BACKOFF: [f64; 3] = [846.5, 1499.5, 2152.5];
const WATCHDOG_MS: [i64; 3] = [16_400, 22_400, 31_400];
// deviates from VARA behavior
const RECENT: usize = 8;
const MISS_TICKS: i64 = 38;
const MISS_LIMIT: i64 = 4;

pub fn watchdog(vias: usize) -> Time {
    WATCHDOG_MS[vias.min(2)] * SECOND / 1000
}

pub(crate) fn state2_backoff(vias: usize, mult: u64) -> i64 {
    rint(BACKOFF[vias.min(2)] * mult as f64)
}

pub fn caller_connect_window(bw: FmBandwidth, vias: usize, active_scan: bool) -> i64 {
    let (u, f) = ((decode_allow(bw, Record::Connect) - CODE_GATE) as f64, frame_ticks(bw, Record::Connect) as f64);
    let x = match vias {
        0 => u + DIRECT_CONNECT_SPAN,
        1 => 2.0 * u + f + VIA_SPAN[1] + VIA_GATES[1],
        _ => 3.0 * u + 2.0 * f + VIA_SPAN[2] + VIA_GATES[2],
    };
    rint(x + if active_scan { ACTIVE_SCAN_SLACK } else { SYNC_SLACK })
}

pub fn receiver_sync_window(bw: FmBandwidth, record: Record, vias: usize, two: bool) -> (f64, f64) {
    let (f, u) = (frame_ticks(bw, record) as f64, decode_allow(bw, record) as f64);
    if two {
        return (f - 40.0, 2.0 * f + VIA_SPAN[0] + SYNC_SLACK);
    }
    let lo = (vias as f64 + 1.0) * f - 40.0;
    let g = CODE_GATE as f64;
    let hi = match vias {
        0 => f + VIA_SPAN[0] + SYNC_SLACK,
        1 => (u - g) + 2.0 * f + VIA_SPAN[1] + VIA_GATES[0] + SYNC_SLACK,
        _ => 2.0 * (u - g) + 3.0 * f + VIA_SPAN[2] + VIA_GATES[1] + SYNC_SLACK,
    };
    (lo, hi)
}

#[derive(Clone, Debug, Default)]
struct Recent(VecDeque<i64>);

impl Recent {
    fn push(&mut self, ticks: i64) -> i64 {
        if self.0.len() == RECENT {
            self.0.pop_front();
        }
        self.0.push_back(ticks);
        rint(self.0.iter().sum::<i64>() as f64 / self.0.len() as f64)
    }
}

#[derive(Clone, Debug)]
pub struct Timing {
    bw: FmBandwidth,
    vias: usize,
    pub count: [i64; RECORDS],
    recent: [Recent; RECORDS],
    pub mean: [i64; RECORDS],
    pub window: [i64; RECORDS],
    pub ctrl_n: i64,
    ctrl_recent: Recent,
    pub ctrl_mean: i64,
    ctrl_win: i64,
    misses: i64,
}

impl Timing {
    pub fn new(bw: FmBandwidth, vias: usize) -> Self {
        Self {
            bw,
            vias: vias.min(2),
            count: [0; RECORDS],
            recent: Default::default(),
            mean: [0; RECORDS],
            window: [0; RECORDS],
            ctrl_n: 0,
            ctrl_recent: Recent::default(),
            ctrl_mean: 0,
            ctrl_win: 0,
            misses: 0,
        }
    }

    fn ack_window(&self, mean: i64) -> i64 {
        if self.vias == 0 { mean + 250 } else { rint(mean as f64 + VIA_ACK_ADD[self.vias]) }
    }

    pub fn ack(&mut self, ticks: i64, record: Record) {
        let (bw, v, i) = (self.bw, self.vias, record.index());
        let ticks = if record == Record::Connect && v > 0 { rint(ticks as f64 + 40.0 * v as f64) } else { ticks };
        self.count[i] += 1;
        self.mean[i] = self.recent[i].push(ticks);
        self.misses = 0;
        let base = record.data_level().map_or(0, |l| l.get());
        for l in bw.levels().skip(base) {
            if self.count[l.get()] != 0 {
                break;
            }
            let du = decode_allow(bw, l) - decode_allow(bw, record);
            let df = frame_ticks(bw, l) - frame_ticks(bw, record);
            self.mean[l.get()] = self.mean[i] + [du, df + 2 * du, 2 * df + 3 * du][v];
        }
        for r in [Record::Short, Record::Start] {
            if self.count[r.index()] == 0 {
                self.mean[r.index()] = self.mean[1];
            }
        }
        for l in 1..=Record::Start.index() {
            self.window[l] = self.ack_window(self.mean[l]);
        }
        if self.ctrl_win == 0 {
            let fl = (frame_ticks(bw, record) - 80) as f64;
            let m = self.mean[i] as f64;
            self.ctrl_win = rint(match v {
                0 => m + 75.0,
                1 => m - fl + VIA_CTRL_ADD[1],
                _ => m - 2.0 * fl + VIA_CTRL_ADD[2],
            });
        }
    }

    pub fn ctrl(&mut self, ticks: i64) {
        let (bw, v) = (self.bw, self.vias);
        self.ctrl_n += 1;
        self.ctrl_mean = if self.ctrl_n == 1 { ticks } else { self.ctrl_recent.push(ticks) };
        self.misses = 0;
        self.ctrl_win = if v == 0 { self.ctrl_mean + 75 } else { rint(self.ctrl_mean as f64 + VIA_CTRL_ADD[v]) };
        if self.ctrl_n != 1 {
            return;
        }
        for l in bw.levels() {
            let i = l.get();
            if self.window[i] != 0 {
                break;
            }
            let (u, f) = ((decode_allow(bw, l) - 80) as f64, (frame_ticks(bw, l) - 80) as f64);
            self.mean[i] = rint([u, 2.0 * u + f, 3.0 * u + 2.0 * f][v] + self.ctrl_mean as f64);
            self.window[i] = self.ack_window(self.mean[i]);
        }
        for r in [Record::Short, Record::Start] {
            if self.window[r.index()] == 0 {
                self.window[r.index()] = self.window[1];
            }
        }
    }

    pub fn miss(&mut self) {
        self.misses = (self.misses + 1).min(MISS_LIMIT);
    }

    pub fn data_window(&self, record: Record) -> i64 {
        let w = match self.window[record.index()] {
            0 => self.ack_window(decode_allow(self.bw, record.level().map_or(record, Record::Data)) + 300),
            w => w,
        };
        w + self.misses * MISS_TICKS
    }

    pub fn ctrl_window(&self) -> i64 {
        (if self.ctrl_win == 0 { 500 } else { self.ctrl_win }) + self.misses * MISS_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(t: &mut Timing, reply: i64) -> bool {
        let ok = reply <= t.ctrl_window();
        if ok {
            t.ctrl(reply);
        } else {
            t.miss();
        }
        ok
    }

    #[test]
    fn drifting_control_replies_stay_inside_the_window() {
        let mut t = Timing::new(FmBandwidth::Wide, 0);
        t.ctrl(372);
        let missed = (0..200).filter(|&k| !exchange(&mut t, 166 + 8 * k)).count();
        assert!(missed <= 2, "{missed}");
    }

    #[test]
    fn a_window_left_behind_widens_until_it_catches_up() {
        let mut t = Timing::new(FmBandwidth::Wide, 0);
        t.ctrl(372);
        (0..20).for_each(|_| t.ctrl(200));
        let tries = (0..10).position(|_| exchange(&mut t, 400)).unwrap();
        assert_eq!(tries, 4);
        let ok: Vec<bool> = (0..30).map(|_| exchange(&mut t, 400)).collect();
        assert!(ok[15..].iter().all(|&o| o), "{ok:?}");
    }

    #[test]
    fn a_measured_reply_clears_the_misses() {
        let mut t = Timing::new(FmBandwidth::Wide, 0);
        t.ctrl(372);
        t.ctrl(200);
        let base = t.ctrl_window();
        t.miss();
        t.miss();
        assert_eq!(t.ctrl_window(), base + 2 * MISS_TICKS);
        assert_eq!(t.data_window(Record::Start), t.window[Record::Start.index()] + 2 * MISS_TICKS);
        t.ctrl(200);
        assert_eq!(t.ctrl_window(), base);
    }
}
