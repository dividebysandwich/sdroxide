use crate::num::rint;
use crate::time::{SECOND, Time};
use xara_phy::hf::ctrl::{Fmt, leadin_symbols};
use xara_phy::hf::level::{Bandwidth, Record};

pub const TICK: Time = 64;
pub(crate) const TICKS_PER_SEC: f64 = (SECOND / TICK) as f64;
const SYM_TICKS: i64 = 32;
pub(crate) const WATCHDOG_LINKED: Time = 60 * SECOND;
pub(crate) const WATCHDOG_UNLINKED: Time = 30 * SECOND;
pub(crate) const DISC_SLACK: f64 = 1.25;
pub(crate) const BREAK_SLACK: f64 = 1.5;
pub(crate) const FQ_WAIT_REPLIES: i64 = 9;
pub(crate) const IDLE_DATA_WAIT: i64 = 201;
pub(crate) const BREAK_BACKOFF_STEP: i64 = 1024;

const DECODE_ALLOW_500: [i64; 13] = [168, 135, 154, 142, 150, 150, 199, 204, 204, 212, 230, 236, 259];
const DECODE_ALLOW_2300: [i64; 16] = [162, 140, 148, 148, 150, 164, 172, 176, 190, 263, 303, 319, 365, 415, 480, 556];
const DECODE_ALLOW_2750: [i64; 17] =
    [150, 118, 140, 136, 136, 144, 152, 164, 172, 180, 276, 315, 340, 415, 450, 535, 636];

fn decode_allow_at(bw: Bandwidth, i: usize) -> i64 {
    let t: &[i64] = match bw {
        Bandwidth::Hz500 => &DECODE_ALLOW_500,
        Bandwidth::Hz2300 => &DECODE_ALLOW_2300,
        Bandwidth::Hz2750 => &DECODE_ALLOW_2750,
    };
    t[i]
}

pub(crate) fn decode_allow(bw: Bandwidth, level: Record) -> i64 {
    decode_allow_at(bw, level.index())
}

pub(crate) fn codeword_ticks(fmt: Fmt) -> i64 {
    (i64::from(fmt.params().nsym) - 1) * SYM_TICKS
}

pub(crate) fn lead_ticks(fmt: Fmt, code: u32) -> i64 {
    i64::from(leadin_symbols(fmt, code, 0)) * SYM_TICKS
}

pub(crate) fn ctrl_ticks(fmt: Fmt, code: u32) -> i64 {
    codeword_ticks(fmt) + lead_ticks(fmt, code)
}

fn rint_frac(bits: u64, k: i64) -> i64 {
    let (m, e) = ((bits & ((1 << 52) - 1)) | (1 << 52), 1075 - ((bits >> 52) & 0x7ff) as i64);
    let x = i128::from(m) * i128::from(k);
    let (q, r) = (x >> e, x & ((1i128 << e) - 1));
    let half = 1i128 << (e - 1);
    (q + i128::from(r > half || (r == half && q & 1 == 1))) as i64
}

#[derive(Clone, Debug)]
pub struct Timing {
    bw: Bandwidth,
    base: i64,
    pub ctrl_delay: i64,
    ctrl_allowance: i64,
    ctrl_n: i64,
    ctrl_sum: i64,
    pub ack_delay: Vec<i64>,
    pub ack_windows: Vec<i64>,
    sum: Vec<i64>,
    pub count: Vec<i64>,
    pub unset: bool,
}

impl Timing {
    pub fn new(bw: Bandwidth, chat_or_p2p: bool) -> Self {
        let n = bw.num_levels();
        let mut t = Self {
            bw,
            base: 0,
            ctrl_delay: 0,
            ctrl_allowance: 0,
            ctrl_n: 0,
            ctrl_sum: 0,
            ack_delay: vec![0; n],
            ack_windows: vec![0; n],
            sum: vec![0; n],
            count: vec![0; n],
            unset: true,
        };
        t.reset(chat_or_p2p);
        t
    }

    pub fn reset(&mut self, chat_or_p2p: bool) {
        self.sum.fill(0);
        self.count.fill(0);
        self.ack_windows.fill(0);
        (self.ctrl_n, self.ctrl_sum) = (0, 0);
        self.base = if chat_or_p2p { 850 } else { 438 };
        self.ctrl_allowance = self.base;
        self.unset = true;
    }

    pub fn ctrl_allowance(&self) -> i64 {
        self.ctrl_allowance
    }

    pub fn base_allowance(&self) -> i64 {
        self.base
    }

    pub(crate) fn reply_window(&self, fmt: Fmt) -> i64 {
        codeword_ticks(fmt) + SYM_TICKS + self.ctrl_allowance
    }

    fn wait(&self, l: usize) -> i64 {
        let d = self.ack_delay[l];
        let w = d + 400;
        if self.ctrl_delay < d { w + rint_frac(0.45f64.to_bits(), d - self.ctrl_delay) } else { w }
    }

    pub fn ctrl(&mut self, t: i64, calling: bool) {
        let t = if t < 0 { 548 } else { t };
        if calling {
            self.ctrl_delay = t;
        } else {
            self.ctrl_n += 1;
            self.ctrl_sum += t;
            self.ctrl_delay = rint(self.ctrl_sum as f64 / self.ctrl_n as f64);
        }
        self.ctrl_allowance = self.ctrl_delay + 150;
        if self.unset {
            self.unset = false;
            for l in 0..self.ack_delay.len() {
                if self.ack_windows[l] == 0 {
                    self.ack_delay[l] = decode_allow_at(self.bw, l) + self.ctrl_delay - 100;
                    self.ack_windows[l] = self.wait(l);
                }
            }
        }
    }

    pub fn ack(&mut self, level: Record, t: i64) {
        if t < 0 || level.get() > self.ack_delay.len() {
            return;
        }
        let i = level.index();
        self.count[i] += 1;
        self.sum[i] += t;
        self.ack_delay[i] = rint(self.sum[i] as f64 / self.count[i] as f64);
        for l in i + 1..self.ack_delay.len() {
            if self.count[l] != 0 {
                break;
            }
            self.ack_delay[l] = self.ack_delay[i] + decode_allow_at(self.bw, l) - decode_allow(self.bw, level);
        }
        for l in 0..self.ack_windows.len() {
            self.ack_windows[l] = self.wait(l);
        }
    }

    pub(crate) fn ack_window(&self, level: Record) -> i64 {
        match self.ack_windows.get(level.index()) {
            Some(&w) if w != 0 => w,
            _ => decode_allow(self.bw, level) + self.ctrl_delay - 100 + 400,
        }
    }

    pub fn set_bw(&mut self, bw: Bandwidth) {
        if bw == self.bw {
            return;
        }
        self.bw = bw;
        let n = bw.num_levels();
        for v in [&mut self.ack_delay, &mut self.ack_windows, &mut self.sum, &mut self.count] {
            v.resize(n, 0);
        }
        for l in 0..n {
            if self.count[l] == 0 {
                self.ack_delay[l] = decode_allow_at(bw, l) + self.ctrl_delay - 100;
            }
            self.ack_windows[l] = self.wait(l);
        }
    }
}
