use super::Report;
use std::collections::VecDeque;
use xara_arq::time::Time;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Session,
    Digi,
    Announce,
    Fragment { last: bool },
}

struct Burst {
    samples: Vec<f64>,
    tail: usize,
    pos: usize,
    earliest: Time,
    keyed: Vec<Report>,
    kind: Kind,
}

pub enum Next {
    Idle,
    Sample(f64),
    Last { sample: f64, kind: Kind, start: Time, end: Time },
}

#[derive(Clone, Copy)]
struct Unkey {
    at: Time,
    ready: usize,
}

#[derive(Default)]
pub struct TxScheduler {
    bursts: VecDeque<Burst>,
    hold_until: Time,
    ptt: bool,
    latency: Time,
    lag: Time,
    unkey: Option<Unkey>,
    tune: Option<(Vec<f64>, usize)>,
}

impl TxScheduler {
    pub fn new(latency: Time, lag: Time) -> Self {
        Self { latency, lag, ..Self::default() }
    }

    pub fn latency(&self) -> Time {
        self.latency
    }

    pub fn set_latency(&mut self, latency: Time, lag: Time) {
        (self.latency, self.lag) = (latency, lag);
    }

    pub fn ptt(&self) -> bool {
        self.ptt
    }

    pub fn is_empty(&self) -> bool {
        self.bursts.is_empty() && self.unkey.is_none()
    }

    pub fn tuning(&self) -> bool {
        self.tune.is_some()
    }

    pub fn tune(&mut self, tone: Vec<f64>) {
        self.tune = Some((tone, 0));
    }

    pub fn retune(&mut self, tone: impl FnOnce() -> Vec<f64>) {
        if let Some((t, _)) = &mut self.tune {
            *t = tone();
        }
    }

    pub fn stop_tune(&mut self) {
        self.tune = None;
    }

    pub fn untune(&mut self, now: Time, reports: &mut VecDeque<Report>) {
        if self.tune.take().is_some() {
            self.unkey_if_empty(now, reports);
        }
    }

    pub fn hold(&mut self, until: Time) {
        self.hold_until = self.hold_until.max(until - self.latency);
    }

    pub fn after_hold(&self, now: Time) -> Time {
        now.max(self.hold_until)
    }

    pub fn next_start(&self, now: Time) -> Time {
        if !self.bursts.is_empty() || (self.ptt && self.unkey.is_none()) { now } else { self.after_hold(now) }
    }

    pub fn push(&mut self, samples: Vec<f64>, tail: usize, earliest: Time, keyed: Vec<Report>, kind: Kind) {
        self.bursts.push_back(Burst { samples, tail, pos: 0, earliest, keyed, kind });
    }

    pub fn key(&mut self, reports: &mut VecDeque<Report>) {
        self.unkey = None;
        if !self.ptt {
            self.ptt = true;
            reports.push_back(Report::Ptt(true));
        }
    }

    pub fn unkey_if_empty(&mut self, now: Time, reports: &mut VecDeque<Report>) {
        if self.bursts.is_empty() && self.ptt {
            let mark = reports.len();
            self.unkey_at(now, reports, mark);
        }
    }

    fn unkey_at(&mut self, end: Time, reports: &mut VecDeque<Report>, mark: usize) {
        let hold = self.latency - self.lag;
        if hold <= 0 {
            self.ptt = false;
            reports.insert(mark, Report::Ptt(false));
        } else {
            self.unkey = Some(Unkey { at: end + hold, ready: mark });
        }
    }

    pub fn pop(&mut self, reports: &mut VecDeque<Report>) -> Option<Report> {
        match &mut self.unkey {
            Some(u) if u.ready == 0 => None,
            Some(u) => {
                u.ready -= 1;
                reports.pop_front()
            }
            None => reports.pop_front(),
        }
    }

    pub fn next(&mut self, t: Time, reports: &mut VecDeque<Report>) -> Next {
        match self.bursts.front() {
            Some(b) if b.pos > 0 || t >= b.earliest => {}
            _ => {
                if let Some(u) = self.unkey.filter(|u| t >= u.at) {
                    self.unkey = None;
                    self.ptt = false;
                    reports.insert(u.ready, Report::Ptt(false));
                }
                let Some((tone, pos)) = &mut self.tune else { return Next::Idle };
                let v = tone[*pos];
                *pos = (*pos + 1) % tone.len();
                self.key(reports);
                return Next::Sample(v);
            }
        }
        self.key(reports);
        let b = self.bursts.front_mut().unwrap();
        if b.pos == 0 {
            reports.extend(b.keyed.drain(..));
        }
        let sample = b.samples.get(b.pos).copied().unwrap_or(0.0);
        let len = b.samples.len() + b.tail;
        b.pos += 1;
        if b.pos < len {
            return Next::Sample(sample);
        }
        let b = self.bursts.pop_front().unwrap();
        let end = t + 1;
        Next::Last { sample, kind: b.kind, start: end - len as Time, end }
    }

    pub fn release(&mut self, end: Time, reports: &mut VecDeque<Report>, mark: usize) {
        if self.bursts.front().is_none_or(|b| b.earliest > end) {
            self.unkey_at(end, reports, mark);
        }
    }
}

pub(crate) trait TxFront: super::Modem {
    fn burst_done(&mut self, kind: Kind, start: Time, end: Time);

    fn key_block(&mut self, t0: Time, input: &[f64], output: &mut [f64]) -> Vec<f64> {
        let mut fed = input.to_vec();
        for (i, (o, x)) in output.iter_mut().zip(fed.iter_mut()).enumerate() {
            let c = self.common_mut();
            *o = match c.tx.next(t0 + i as Time, &mut c.reports) {
                Next::Idle => {
                    *o = 0.0;
                    continue;
                }
                Next::Sample(v) => v,
                Next::Last { sample, kind, start, end } => {
                    self.burst_done(kind, start, end);
                    sample
                }
            };
            *x = 0.0;
        }
        fed
    }
}
