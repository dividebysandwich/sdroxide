use crate::cq;
use crate::hf::frames::queue_hint;
use crate::kiss::{self, BW, Command, MAX_QUEUED_BYTES, Push, Reassembler};
use crate::rng::XorShift;
use crate::time::{SECOND, Time};
use std::collections::VecDeque;
use xara_phy::hf::ctrl::Fmt;
use xara_phy::hf::ctrl::code::CQ;
use xara_phy::hf::frame::Control;
use xara_phy::hf::level::{Bandwidth, Record};

pub(crate) const PERSIST_TICK: Time = SECOND / 5;
pub(crate) const DCD_HOLD: Time = SECOND / 5;
pub const DEFAULT_PERSISTENCE: f64 = 0.3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub code: u32,
    pub level: Record,
    pub frames: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Pending,
    CancelPending,
    Send(Plan),
    Expect(Option<Record>),
    Sn(f64),
    Kiss(Command, Vec<u8>),
    Cq(cq::Cq),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ctx {
    pub idle: bool,
    pub bw500: bool,
    pub busy: bool,
    pub reply: Fmt,
    pub marker: u8,
    pub queued: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Tx,
    Rx { level: Record, until: Time },
}

pub struct Broadcast {
    pub persistence: f64,
    pub ignore_dcd: bool,
    pub kiss_rx: bool,
    pub cq_rx: bool,
    queue: VecDeque<(Command, Vec<u8>)>,
    queued: usize,
    cq: Option<Vec<u8>>,
    phase: Phase,
    reasm: Reassembler,
    last_busy: Option<Time>,
    next_try: Time,
    rng: XorShift,
    out: VecDeque<Event>,
}

pub fn window(level: Record) -> Time {
    let l = BW.level(level);
    let lead = f64::from(l.lead_in()) * l.symbol_period();
    ((4.0 * lead + l.airtime()) * SECOND as f64) as Time
}

impl Broadcast {
    pub fn new(seed: u64) -> Self {
        Self {
            persistence: DEFAULT_PERSISTENCE,
            ignore_dcd: false,
            kiss_rx: false,
            cq_rx: false,
            queue: VecDeque::new(),
            queued: 0,
            cq: None,
            phase: Phase::Idle,
            reasm: Reassembler::default(),
            last_busy: None,
            next_try: 0,
            rng: XorShift::new(seed),
            out: VecDeque::new(),
        }
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.out.pop_front()
    }

    pub fn idle(&self) -> bool {
        self.phase == Phase::Idle && self.cq.is_none()
    }

    pub fn receiving(&self) -> bool {
        matches!(self.phase, Phase::Rx { .. })
    }

    pub fn queued(&self) -> usize {
        self.queued
    }

    pub fn kiss(&mut self, cmd: Command, data: Vec<u8>) -> bool {
        if self.queued + data.len() > MAX_QUEUED_BYTES {
            return false;
        }
        self.queued += data.len();
        self.queue.push_back((cmd, data));
        true
    }

    pub fn cq_frame(&mut self, call: &str, bw: Bandwidth) -> bool {
        match (self.idle(), cq::frame(call, bw)) {
            (true, Some(f)) => {
                self.cq = Some(f);
                true
            }
            _ => false,
        }
    }

    pub fn poll(&mut self, now: Time, ctx: Ctx) {
        if ctx.busy {
            self.last_busy = Some(now);
        }
        if matches!(self.phase, Phase::Rx { until, .. } if now >= until) {
            self.end_rx();
        }
        if self.phase != Phase::Idle || !ctx.idle {
            return;
        }
        if let Some(f) = self.cq.take() {
            self.start(Plan { code: CQ, level: cq::RECORD, frames: vec![f] });
            return;
        }
        if self.queue.is_empty() || now < self.next_try {
            return;
        }
        if ctx.bw500 && !self.ignore_dcd {
            if ctx.busy || self.last_busy.is_some_and(|t| now < t + DCD_HOLD) {
                return;
            }
            if self.persistence < self.rng.uniform() {
                self.next_try = now + PERSIST_TICK;
                return;
            }
        }
        while let Some((cmd, data)) = self.queue.pop_front() {
            self.queued -= data.len();
            let Ok((level, fields)) = kiss::encode(cmd, &data).and_then(|b| kiss::fragments(&b)) else { continue };
            let frames = fields
                .iter()
                .map(|f| {
                    let hint = queue_hint(BW, level, ctx.queued.saturating_sub(f.len()));
                    kiss::frame(f, level, Control { reply: ctx.reply.bits(), compressed: false, hint }, ctx.marker)
                })
                .collect();
            self.start(Plan { code: kiss::code_for_level(level), level, frames });
            return;
        }
    }

    fn start(&mut self, p: Plan) {
        self.phase = Phase::Tx;
        self.out.push_back(Event::Pending);
        self.out.push_back(Event::Send(p));
    }

    pub fn tx_done(&mut self) {
        if self.phase == Phase::Tx {
            self.phase = Phase::Idle;
            self.out.push_back(Event::CancelPending);
        }
    }

    fn end_rx(&mut self) {
        self.phase = Phase::Idle;
        self.reasm.reset();
        self.out.push_back(Event::Expect(None));
    }

    pub fn on_ctrl(&mut self, code: u32, t_end: Time) -> bool {
        if self.phase != Phase::Idle {
            return false;
        }
        let level = match kiss::level_for_code(code) {
            Some(l) if self.kiss_rx => l,
            None if code == CQ && self.cq_rx => cq::RECORD,
            _ => return false,
        };
        self.reasm.reset();
        self.phase = Phase::Rx { level, until: t_end + window(level) };
        self.out.push_back(Event::Expect(Some(level)));
        true
    }

    pub fn on_frame(&mut self, level: Record, bytes: &[u8], snr_db: f64, t_end: Time) {
        let Phase::Rx { level: want, .. } = self.phase else { return };
        if level != want {
            return;
        }
        self.out.push_back(Event::Sn(snr_db));
        if level == cq::RECORD {
            if let Some(c) = cq::parse(bytes) {
                self.out.push_back(Event::Cq(c));
            }
            return self.end_rx();
        }
        match self.reasm.push(bytes, level) {
            Push::More => self.phase = Phase::Rx { level, until: t_end + window(level) },
            Push::Done(b) => {
                if let Some((cmd, data)) = kiss::decode(&b) {
                    self.out.push_back(Event::Kiss(cmd, data));
                }
                self.end_rx();
            }
            Push::Dropped => {}
        }
    }
}
