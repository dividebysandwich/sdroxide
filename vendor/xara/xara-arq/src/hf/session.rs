use crate::compression::Compression;
use crate::hf::adapt::{Adaptation, SnrTrend, ack_decision, step, step_count, step_down, step_down_n};
use crate::hf::frames::{
    HINT_MAX, ReplyFormat, StartCap, UNREGISTERED_MAX_LEVEL, format_for_level, hint_table_minimum, parse, payload_len,
    queue_hint, queue_hint_text, start_level_cap,
};
use crate::hf::timing::{
    BREAK_BACKOFF_STEP, BREAK_SLACK, DISC_SLACK, FQ_WAIT_REPLIES, IDLE_DATA_WAIT, TICK, TICKS_PER_SEC, Timing,
    WATCHDOG_LINKED, WATCHDOG_UNLINKED, codeword_ticks, ctrl_ticks, lead_ticks,
};
use crate::hf::turn::{HintOverride, TurnKind, TurnStart, TurnState, turn_start};
use crate::host::{HostEvent, HostSettings, LinkBw};
use crate::link::Link;
use crate::session::Core;
use crate::textcomp::{START_STATE, compress, fill_frame, turn_compresses};
use crate::time::{DISC_TIMEOUT_LIMIT, IDLE_POLL, IDLE_POLL_LIMIT, SECOND, Time};
use std::collections::VecDeque;
use xara_phy::call::{call_hash, is_us_call};
use xara_phy::hf::ctrl::code::*;
use xara_phy::hf::ctrl::{Fmt, Role};
use xara_phy::hf::frame::{Control, Header, build_frame, term_marker};
use xara_phy::hf::level::{Bandwidth, Record};

const HOST_SINCE_RX_CAP: usize = 500;
const ANSWER_LEVEL_MAX: Record = Record::new(4).unwrap();
const SNR_REF_INIT_DB: f64 = -100.0;
const CALL_EXTRA_LEAD: u32 = 9;
const ANSWER_EXTRA_LEAD: u32 = 7;
const QRT_CWID_BLANKS: [u32; 3] = [9, 12, 10];

#[derive(Clone, Debug)]
pub struct Config {
    pub bw: Bandwidth,
    pub host: HostSettings,
    pub p2p: bool,
    pub cwid: bool,
    pub seed: u64,
    pub offer_level: Option<Record>,
}

impl Config {
    pub fn new(bw: Bandwidth, mycall: &str) -> Self {
        Self { bw, host: HostSettings::new(mycall, 15), p2p: false, cwid: false, seed: 1, offer_level: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Listen,
    Answering,
    Calling,
    HeaderSent,
    Receiving,
    Break,
    Sending,
    Qrt,
    Closing,
    IdleSent,
    Idle,
    FqWait,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tx {
    Ctrl { code: u32, fmt: Fmt, extra: u32 },
    Data { level: Record, frames: Vec<Vec<u8>> },
    CwId { call: String, blanks: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keys {
    pub seed: u32,
    pub mycalls: [u32; 5],
    pub role: Role,
    pub connect: Option<(u32, Role)>,
}

#[derive(Clone, Copy, Debug)]
struct HeaderTx {
    level: Record,
    offer: Record,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listen {
    pub bw: Bandwidth,
    pub keys: Keys,
    pub codes: Vec<u32>,
    pub levels: Vec<Record>,
    pub nframes: usize,
    pub linked: bool,
}

pub type Output = crate::session::Output<Tx, Listen>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CtrlIn {
    pub code: u32,
    pub fmt: Fmt,
    pub hits: usize,
    pub snr_norm: f64,
    pub t_end: Time,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameIn {
    pub ok: bool,
    pub bytes: Vec<u8>,
    pub snr_db: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataIn {
    pub level: Record,
    pub frames: Vec<FrameIn>,
    pub t_end: Time,
}

#[derive(Default, Debug)]
struct TxBuf {
    buf: VecDeque<u8>,
    inflight: Vec<usize>,
    resend: Option<usize>,
    final_sent: bool,
    text_turn: Option<bool>,
    state: usize,
    states: Vec<usize>,
}

impl TxBuf {
    fn drain_front(&mut self, n: usize) {
        self.buf.drain(..n.min(self.buf.len()));
    }

    fn accept_all(&mut self) -> Vec<usize> {
        let mut left = Vec::new();
        for n in std::mem::take(&mut self.inflight) {
            self.drain_front(n);
            if n > 0 {
                left.push(self.buf.len());
            }
        }
        if let Some(&s) = self.states.last() {
            self.state = s;
        }
        self.states.clear();
        left
    }

    fn first_only(&mut self) {
        let a = self.inflight.first().copied().unwrap_or(0);
        self.drain_front(a);
        if let Some(&s) = self.states.first() {
            self.state = s;
        }
        self.inflight.clear();
    }

    fn second_only(&mut self) {
        if let [a, b] = self.inflight[..] {
            self.buf.drain(a..a + b);
            self.resend = Some(a);
        }
        self.inflight.clear();
    }
}

struct Chunk {
    data: Vec<u8>,
    take: usize,
    last: bool,
    compressed: bool,
}

impl Chunk {
    fn raw(q: &[u8], take: usize, last: bool) -> Self {
        Chunk { data: q[..take].to_vec(), take, last, compressed: false }
    }
}

#[derive(Clone, Copy, Debug)]
struct FrameMeta {
    ctrl: Control,
    last: bool,
    full: bool,
    next_level: Option<u8>,
}

#[derive(Debug)]
struct Good {
    meta: FrameMeta,
    data: Vec<u8>,
}

struct Carry {
    marker: u8,
    break_draw: Option<u32>,
    rx_level: Record,
    offered: Record,
    reply_fmt: Fmt,
    ctrl_snr_held: f32,
    snr_ref_db: f64,
    timing: Timing,
}

pub struct Session {
    core: Core<Tx, Listen>,
    carry: Carry,
    cfg: Config,
    bw: Bandwidth,
    state: State,
    mycall: String,
    remote: String,
    role: Role,
    connected: bool,
    registered: bool,
    header: Option<HeaderTx>,

    level: Record,
    prev_level: Record,
    upstep_from: Record,
    upsteps: usize,
    fallback: Vec<Record>,
    next_tx_level: Record,
    offered_by_peer: Record,
    hint_override: HintOverride,
    stays: Option<u32>,
    trend: SnrTrend,
    adapt: Adaptation,

    fmt: Fmt,
    ctrl_snr: ReplyFormat,
    alt: bool,
    last_code: Option<u32>,
    req_fmt: Option<Fmt>,
    attempts: u32,

    hold_from: Option<Time>,

    dropped: bool,

    tx: TxBuf,
    link: Link,
    host_kind: TurnKind,
    host_since_rx: Vec<u8>,
    last_held: usize,
}

impl Session {
    pub fn new(cfg: Config) -> Self {
        let mut s = Self::fresh(cfg);
        s.announce();
        s
    }

    fn fresh(cfg: Config) -> Self {
        let (bw, registered) = (cfg.bw, cfg.host.registered);
        Self {
            core: Core::new(cfg.seed),
            carry: Carry {
                marker: 1,
                break_draw: None,
                rx_level: Record::ONE,
                offered: Record::ONE,
                reply_fmt: Fmt::F0,
                ctrl_snr_held: 0.0,
                snr_ref_db: SNR_REF_INIT_DB,
                timing: Timing::new(bw, cfg.host.chat || cfg.p2p),
            },
            mycall: cfg.host.primary_call(),
            adapt: Adaptation::new(bw),
            cfg,
            bw,
            state: State::Listen,
            remote: String::new(),
            role: Role::Answerer,
            connected: false,
            level: Record::ONE,
            prev_level: Record::ONE,
            fallback: Vec::new(),
            next_tx_level: Record::ONE,
            fmt: Fmt::F0,
            ctrl_snr: ReplyFormat::default(),
            alt: false,
            stays: Some(0),
            upsteps: 0,
            upstep_from: Record::ONE,
            trend: SnrTrend::default(),
            tx: TxBuf::default(),
            last_code: None,
            attempts: 0,
            req_fmt: None,
            dropped: false,
            header: None,
            registered,
            hold_from: None,
            host_kind: TurnKind::Partial,
            host_since_rx: Vec::new(),
            last_held: 0,
            offered_by_peer: Record::ONE,
            hint_override: HintOverride::Off,
            link: Link::default(),
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn level(&self) -> Record {
        self.level
    }

    pub fn peer(&self) -> &str {
        &self.remote
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn pop(&mut self) -> Option<Output> {
        self.core.pop()
    }

    pub fn bandwidth(&self) -> Bandwidth {
        self.bw
    }

    pub fn two_frames(&self) -> bool {
        self.stays.is_some_and(|n| n > 5)
    }

    fn nframes(&self) -> usize {
        if self.two_frames() { 2 } else { 1 }
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.cfg
    }

    pub fn reset_timing(&mut self) {
        self.carry.timing.reset(self.cfg.host.chat || self.cfg.p2p);
    }

    pub fn buffered(&self) -> usize {
        self.tx.buf.len() + self.link.files.len()
    }

    pub fn next_deadline(&self) -> Option<Time> {
        self.core.next_deadline(&self.link)
    }

    pub fn max_start_level(&self) -> Record {
        self.start_cap(true)
    }

    fn start_cap(&self, header_sent: bool) -> Record {
        let c = StartCap { registered: self.registered, fs: self.link.b2f.fs(), text: self.link.text(), header_sent };
        start_level_cap(self.bw, c).min(self.bw.top_level())
    }

    fn max_level(&self) -> Record {
        if self.registered { self.bw.top_level() } else { UNREGISTERED_MAX_LEVEL }
    }

    fn keys(&self) -> Keys {
        let mut mycalls = [0; 5];
        for (h, c) in mycalls.iter_mut().zip(&self.cfg.host.mycalls) {
            *h = call_hash(c.as_bytes());
        }
        let (seed, connect) = match (self.state, self.role) {
            (State::Listen, _) => (0, None),
            (State::Calling, _) => (call_hash(self.remote.as_bytes()), None),
            (_, Role::Caller) => (call_hash(self.remote.as_bytes()), Some(call_hash(self.mycall.as_bytes()))),
            (_, Role::Answerer) => (
                call_hash(self.mycall.as_bytes()),
                (!self.remote.is_empty()).then(|| call_hash(self.remote.as_bytes())),
            ),
        };
        let role = if self.state == State::Listen { Role::Answerer } else { self.role };
        Keys { seed, mycalls, role, connect: connect.map(|h| (h, role)) }
    }

    fn accepts(&self) -> Vec<u32> {
        let chat = self.cfg.host.chat;
        match self.state {
            State::Listen => self.request_codes(),
            State::Answering => {
                [QRT, ACK_REQ_0, ACK_REQ_1, QRT_NO_CWID].into_iter().chain(self.request_codes()).collect()
            }
            State::Calling => match self.bw {
                Bandwidth::Hz2750 => {
                    connect_replies(Bandwidth::Hz2300).chain(connect_replies(Bandwidth::Hz2750)).collect()
                }
                bw => connect_replies(bw).collect(),
            },
            State::HeaderSent => vec![NACK, QRT, QRT_NO_CWID, BREAK, IDLE],
            State::Receiving => vec![QRT, ACK_REQ_0, ACK_REQ_1, QRT_NO_CWID, BREAK, IDLE],
            State::Break if chat => vec![QRT, ACK_REQ_0, ACK_REQ_1, GO_AHEAD, QRT_NO_CWID, BREAK, IDLE],
            State::Break => {
                [QRT, ACK_REQ_0, ACK_REQ_1, GO_AHEAD, QRT_NO_CWID, IDLE].into_iter().chain(start_levels()).collect()
            }
            State::Sending => vec![
                ACK_STAY,
                NACK,
                QRT,
                ACK_UP2,
                ACK_DOWN,
                ACK_UP1,
                ACK_UP3,
                ACK_UP4,
                ACK_SECOND_ONLY,
                ACK_FIRST_ONLY,
                NACK_MISSED,
                QRT_NO_CWID,
                BREAK,
                IDLE,
            ],
            State::Qrt | State::Closing => Vec::new(),
            State::IdleSent => vec![QRT, ACK_REQ_0, ACK_REQ_1, NO_TRAFFIC, QRT_NO_CWID, BREAK],
            State::Idle => vec![QRT, QRT_NO_CWID, BREAK, IDLE],
            State::FqWait => vec![QRT, ACK_REQ_0, ACK_REQ_1, QRT_NO_CWID],
        }
    }

    fn ack_req(&self) -> u32 {
        if self.alt { ACK_REQ_1 } else { ACK_REQ_0 }
    }

    fn request_codes(&self) -> Vec<u32> {
        let k = self.cfg.host.mycalls.len().min(ALIASES as usize) as u32;
        match self.bw {
            Bandwidth::Hz500 => connect_reqs(Bandwidth::Hz500, k).collect(),
            _ => connect_reqs(Bandwidth::Hz2300, k).chain(connect_reqs(Bandwidth::Hz2750, k)).collect(),
        }
    }

    fn expected_levels(&self) -> Vec<Record> {
        let mut v: Vec<Record> = [self.level, self.prev_level]
            .into_iter()
            .chain(self.fallback.iter().copied())
            .filter(|&l| l <= self.bw.top_level())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn announce(&mut self) {
        let receiving = matches!(self.state, State::Receiving | State::Answering);
        let l = Listen {
            bw: self.bw,
            keys: self.keys(),
            codes: self.accepts(),
            levels: if receiving { self.expected_levels() } else { Vec::new() },
            nframes: self.nframes(),
            linked: !matches!(self.state, State::Listen | State::Answering | State::Calling | State::HeaderSent),
        };
        self.core.out.push_back(Output::Listen(l));
    }

    fn enter(&mut self, s: State) {
        if s != self.state {
            if !(self.state == State::Calling && s == State::HeaderSent) {
                self.alt = false;
            }
            if s == State::Sending {
                self.tx.text_turn = None;
                self.tx.state = START_STATE;
            }
            if s == State::Receiving {
                self.link.rx.turn_compressed = None;
                self.link.rx.slots.clear();
                self.level = self.carry.rx_level;
                self.prev_level = self.level;
                self.fallback.clear();
            }
            self.state = s;
            self.stays = Some(0);
            self.upsteps = 0;
            self.req_fmt = None;
        }
        self.announce();
    }

    fn set_state(&mut self, s: State) {
        self.state = s;
        self.announce();
    }

    fn listen(&mut self) {
        let fresh = Self::fresh(self.cfg.clone());
        let old = std::mem::replace(self, fresh);
        (self.core, self.carry) = (old.core, old.carry);
        self.carry.timing.set_bw(self.bw);
        self.carry.timing.reset(self.cfg.host.chat || self.cfg.p2p);
        self.announce();
    }

    fn send_ctrl(&mut self, code: u32, fmt: Fmt, extra: u32) {
        self.fmt = fmt;
        if code == IDLE {
            self.hold_from = None;
        }
        if !ACK_REQ.contains(&code) {
            self.last_code = Some(code);
        }
        self.link.idle_timer = None;
        self.core.transmitting = true;
        self.link.deadline = None;
        self.core.out.push_back(Output::Tx(Tx::Ctrl { code, fmt, extra }));
    }

    fn resend_last(&mut self) {
        if let Some(c) = self.last_code {
            self.send_ctrl(c, Fmt::F0, 0);
        }
    }

    fn feed_watchdog(&mut self, now: Time) {
        self.link.watchdog = Some(
            now + if self.connected || self.state == State::HeaderSent { WATCHDOG_LINKED } else { WATCHDOG_UNLINKED },
        );
    }

    fn level_set(&mut self, new: Record) {
        self.upsteps = if new <= self.upstep_from { 0 } else { self.upsteps + new.get() - self.upstep_from.get() };
        self.upstep_from = new;
        self.prev_level = self.level;
        self.level = new;
    }

    fn qrt(&mut self) {
        let code = if self.cwid() { QRT } else { QRT_NO_CWID };
        let fmt = if matches!(self.state, State::Receiving | State::Answering) { self.fmt } else { Fmt::F0 };
        self.send_ctrl(code, fmt, 0);
        self.set_state(State::Qrt);
    }

    fn cwid(&self) -> bool {
        self.cfg.cwid || is_us_call(&self.mycall)
    }

    fn close(&mut self, blanks: u32) {
        if self.role == Role::Answerer && !self.connected {
            self.core.host(HostEvent::CancelPending);
            self.listen();
        } else if self.cwid() {
            let call = self.mycall.split('-').next().unwrap_or("").to_string();
            (self.link.deadline, self.link.watchdog, self.link.idle_timer) = (None, None, None);
            self.core.transmitting = true;
            self.core.out.push_back(Output::Tx(Tx::CwId { call, blanks }));
            self.set_state(State::Closing);
        } else {
            self.core.host(HostEvent::Disconnected);
            self.listen();
        }
    }

    pub fn connect(&mut self, now: Time, mycall: Option<&str>, remote: &str) {
        if self.state != State::Listen {
            return;
        }
        self.mycall = mycall.map_or_else(|| self.cfg.host.primary_call(), str::to_string);
        self.remote = remote.to_string();
        self.role = Role::Caller;
        self.carry.marker = term_marker(self.mycall.as_bytes());
        self.attempts = 0;
        self.set_state(State::Calling);
        self.feed_watchdog(now);
        self.request(now);
    }

    fn request(&mut self, now: Time) {
        self.feed_watchdog(now);
        let code = connect_req(self.bw, 0);
        self.attempts += 1;
        self.send_ctrl(code, Fmt::F0, if self.attempts == 1 { CALL_EXTRA_LEAD } else { 0 });
    }

    pub fn send(&mut self, now: Time, data: &[u8]) {
        if self.hold_from.is_some() && self.role == Role::Answerer {
            let room = payload_len(self.bw, self.next_tx_level.min(self.bw.top_level()));
            let keep = HOST_SINCE_RX_CAP.saturating_sub(self.host_since_rx.len()).min(data.len());
            self.host_since_rx.extend_from_slice(&data[..keep]);
            let acc = &self.host_since_rx;
            self.host_kind = if data.len() >= 6 && data.ends_with(b">\r") {
                TurnKind::Prompt
            } else if acc.len() > room - 1
                || self.link.text() && compress(acc, room - 1, START_STATE).consumed < acc.len()
            {
                TurnKind::Overflow
            } else {
                TurnKind::Partial
            };
        }
        self.link.queue(now, self.connected, data, &mut self.tx.buf);
        if self.state == State::Idle && !self.core.transmitting && self.link.deadline.is_none() {
            self.link.deadline = Some(now.max(self.core.tx_end + IDLE_DATA_WAIT * TICK));
        }
    }

    pub fn disconnect(&mut self, now: Time) {
        if self.state == State::FqWait {
            return;
        }
        let linking = self.connected || matches!(self.state, State::Calling | State::Answering | State::HeaderSent);
        if linking && self.buffered() > 0 {
            self.link.disc_after_tx = true;
        } else {
            self.abort(now);
        }
    }

    pub fn abort(&mut self, now: Time) {
        match self.state {
            State::Listen | State::Closing | State::Qrt => {}
            State::Calling => self.attempts = self.cfg.host.retries,
            _ => {
                self.link.disc_now = true;
                match self.state {
                    State::Idle if !self.core.transmitting => self.link.deadline = Some(now),
                    State::Receiving if !self.core.transmitting => {
                        self.link.deadline = Some(now.max(self.disc_window()))
                    }
                    _ => {}
                }
            }
        }
    }

    fn disc_window(&self) -> Time {
        let l = self.bw.level(self.level);
        let frame = (l.airtime() * TICKS_PER_SEC) as i64;
        let more = if self.two_frames() {
            frame - (f64::from(l.lead_in()) * l.symbol_period() * TICKS_PER_SEC) as i64
        } else {
            0
        };
        self.core.tx_end + (frame + more + (DISC_SLACK * self.carry.timing.base_allowance() as f64) as i64) * TICK
    }

    pub fn drop_link(&mut self) {
        match self.state {
            State::Listen | State::Calling | State::Qrt | State::Closing => {}
            _ if self.core.transmitting => self.dropped = true,
            _ => self.close(0),
        }
    }

    pub fn set_break_draw(&mut self, r: u32) {
        self.carry.break_draw = Some(r);
    }

    pub fn on_tx_done(&mut self, now: Time) {
        self.core.transmitting = false;
        self.core.tx_end = now;
        if self.dropped {
            self.dropped = false;
            self.link.deadline = None;
            self.close(0);
            return;
        }
        let draw = self.carry.break_draw.take();
        let w = match self.state {
            State::Calling => Some(self.carry.timing.reply_window(Fmt::F1)),
            State::HeaderSent | State::Sending => Some(match self.req_fmt {
                Some(_) => self.carry.timing.reply_window(Fmt::F0),
                None => {
                    let f = self.fmt.min(self.carry.reply_fmt);
                    let code = if self.tx.final_sent { BREAK } else { ACK_STAY };
                    self.carry.timing.ack_window(self.level) + lead_ticks(f, code) + codeword_ticks(f)
                }
            }),
            State::Break => {
                let r = draw.map_or_else(|| self.core.rng.upto(2), u64::from);
                Some(
                    (BREAK_SLACK * self.carry.timing.base_allowance() as f64) as i64
                        + (r as i64 + 1) * BREAK_BACKOFF_STEP,
                )
            }
            State::IdleSent => Some(self.carry.timing.reply_window(Fmt::F0)),
            State::FqWait => Some(FQ_WAIT_REPLIES * self.carry.timing.reply_window(Fmt::F0)),
            State::Receiving if self.link.disc_now => Some((self.disc_window() - now) / TICK),
            State::Idle if self.link.disc_now || !self.tx.buf.is_empty() => Some(IDLE_DATA_WAIT),
            State::Qrt => {
                self.close(0);
                None
            }
            State::Closing => {
                self.core.host(HostEvent::Disconnected);
                self.listen();
                None
            }
            _ => None,
        };
        self.link.deadline = w.map(|t| now + t * TICK);
    }

    pub fn poll(&mut self, now: Time) {
        if self.core.transmitting {
            return;
        }
        if self.link.watchdog_fired(now) {
            if self.state != State::Answering {
                self.core.host(HostEvent::TimedOut);
            }
            self.close(0);
            return;
        }
        if self.link.files.due(now) {
            self.link.files.flush(&mut self.tx.buf);
            if self.state == State::Idle && self.link.deadline.is_none() && !self.tx.buf.is_empty() {
                self.link.deadline = Some(now.max(self.core.tx_end + IDLE_DATA_WAIT * TICK));
            }
        }
        if let Some(ticks) = self.link.idle_fired(now) {
            if !self.cfg.host.chat && ticks >= IDLE_POLL_LIMIT {
                self.qrt();
            } else {
                self.send_ctrl(IDLE, Fmt::F0, 0);
                self.enter(State::IdleSent);
            }
            return;
        }
        if !self.link.deadline_fired(now) {
            return;
        }
        if matches!(self.state, State::HeaderSent | State::Sending | State::Break | State::IdleSent)
            && self.link.disc_now
        {
            self.link.disc_timeouts += 1;
            if self.link.disc_timeouts >= DISC_TIMEOUT_LIMIT {
                self.qrt();
                return;
            }
        }
        match self.state {
            State::Calling => {
                if self.attempts < self.cfg.host.retries {
                    self.request(now);
                } else {
                    self.core.host(HostEvent::UnableToConnect);
                    self.close(0);
                }
            }
            State::HeaderSent | State::Sending => {
                let f = match self.req_fmt {
                    Some(f) => f.more_robust(),
                    None if self.state == State::Sending => format_for_level(self.bw, self.level.down()),
                    None => self.fmt.min(self.carry.reply_fmt),
                };
                self.req_fmt = Some(f);
                self.send_ctrl(self.ack_req(), f, 0);
            }
            State::Break => self.send_ctrl(BREAK, Fmt::F0, 0),
            State::IdleSent => self.send_ctrl(IDLE, Fmt::F0, 0),
            State::Receiving if self.link.disc_now => self.qrt(),
            State::FqWait => self.close(0),
            State::Idle if self.link.disc_now || !self.tx.buf.is_empty() => self.take_turn(),
            _ => {}
        }
    }

    fn ticks_since_tx(&self, c: &CtrlIn) -> i64 {
        (c.t_end - self.core.tx_end) / TICK - ctrl_ticks(c.fmt, c.code)
    }

    pub fn on_ctrl(&mut self, now: Time, c: CtrlIn) {
        if self.core.transmitting || !self.accepts().contains(&c.code) {
            return;
        }
        self.feed_watchdog(now);
        self.link.idle_timer = None;
        let (snr, fmt) = self.ctrl_snr.update(c.snr_norm, c.hits, c.fmt);
        self.carry.ctrl_snr_held = snr;
        self.carry.snr_ref_db = f64::from(snr);
        self.carry.reply_fmt = fmt;
        if QRT_ANY.contains(&c.code) {
            let blanks = if c.code == QRT { QRT_CWID_BLANKS[c.fmt.index()] } else { 0 };
            self.close(blanks);
            return;
        }
        if self.link.disc_now && !matches!(self.state, State::Listen | State::Calling) {
            if self.state == State::Sending {
                match c.code {
                    ACK_STAY | ACK_UP2 | ACK_DOWN | ACK_UP1 | ACK_UP3 | ACK_UP4 | BREAK => {
                        self.tx.accept_all();
                    }
                    ACK_SECOND_ONLY => self.tx.second_only(),
                    ACK_FIRST_ONLY => self.tx.first_only(),
                    _ => {}
                }
            }
            self.qrt();
            return;
        }
        if matches!(self.state, State::HeaderSent | State::Sending) {
            self.alt = !self.alt;
            if self.req_fmt.is_none() {
                self.carry.timing.ack(self.level, self.ticks_since_tx(&c));
            }
        }
        let measured = match self.state {
            State::Calling => true,
            State::Break => c.code == GO_AHEAD || start_level(c.code).is_some(),
            State::IdleSent => c.code == NO_TRAFFIC,
            _ => false,
        };
        if measured {
            self.carry.timing.ctrl(self.ticks_since_tx(&c), self.state == State::Calling);
        }
        match self.state {
            State::Listen => self.on_request(now, c),
            State::Answering => self.ctrl_answering(now, c),
            State::Calling => self.on_reply(c),
            State::HeaderSent => self.ctrl_header_sent(now, c),
            State::Receiving => self.ctrl_receiving(c),
            State::Break => self.ctrl_break(now, c),
            State::Sending => self.ctrl_sending(c),
            State::IdleSent => match c.code {
                NO_TRAFFIC if self.tx.buf.is_empty() => {
                    self.link.idle_timer = Some(now + IDLE_POLL);
                    self.enter(State::Idle);
                }
                NO_TRAFFIC => self.take_turn(),
                BREAK => self.grant(),
                _ => self.resend_last(),
            },
            State::FqWait => self.resend_last(),
            State::Idle => match c.code {
                BREAK => self.grant(),
                IDLE => {
                    self.fmt = Fmt::F0;
                    self.answer_idle();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn on_request(&mut self, now: Time, c: CtrlIn) {
        let Some((req_bw, alias)) = connect_req_alias(c.code) else { return };
        let bw = match req_bw {
            Bandwidth::Hz2300 if self.bw == Bandwidth::Hz500 => self.bw,
            Bandwidth::Hz2750 => self.bw,
            b => b,
        };
        self.switch_bw(bw);
        self.mycall = self.cfg.host.mycalls[alias as usize].clone();
        self.role = Role::Answerer;
        let q = self.adapt.level_from_snr(c.snr_norm, ANSWER_LEVEL_MAX, 6.0);
        (self.level, self.prev_level, self.upstep_from, self.carry.rx_level) = (q, q, q, q);
        self.fallback.clear();
        let reply = connect_reply(self.bw, q.get() as u32);
        if self.state == State::Listen {
            self.core.host(HostEvent::Pending);
            self.state = State::Answering;
            self.feed_watchdog(now);
        }
        self.send_ctrl(reply, Fmt::F1, if self.cfg.host.chat { 0 } else { ANSWER_EXTRA_LEAD });
        self.announce();
    }

    fn switch_bw(&mut self, bw: Bandwidth) {
        if bw != self.bw {
            self.bw = bw;
            self.carry.timing.set_bw(bw);
            self.adapt = Adaptation::new(bw);
        }
    }

    fn ctrl_answering(&mut self, now: Time, c: CtrlIn) {
        match c.code {
            ACK_REQ_0 | ACK_REQ_1 if c.code == self.ack_req() => {
                self.alt = !self.alt;
                let new = step_down(self.bw, self.level);
                self.header_nack(new);
            }
            ACK_REQ_0 | ACK_REQ_1 => self.resend_last(),
            _ => self.on_request(now, c),
        }
    }

    fn header_nack(&mut self, new: Record) {
        let old = self.level;
        self.prev_level = old;
        self.level = new;
        self.fallback = fallback_below(old);
        self.send_ctrl(NACK, self.fmt, 0);
        self.announce();
    }

    fn on_header(&mut self, now: Time, d: DataIn) {
        let f = &d.frames[0];
        let Some(h) = Header::parse(&f.bytes).filter(|_| f.ok) else {
            self.alt = !self.alt;
            let new = step_down(self.bw, d.level);
            self.level = d.level;
            self.header_nack(new);
            return;
        };
        self.remote = h.call;
        self.carry.marker = term_marker(self.remote.as_bytes());
        self.carry.snr_ref_db = self.trend.push(f.snr_db);
        self.adapt.frame_ok(d.level);
        self.core.host(HostEvent::Sn(f.snr_db));
        self.fmt = Fmt::from_bits(h.reply);
        self.carry.rx_level = d.level;
        self.next_tx_level = Record::clamped(usize::from(h.offer), self.bw.top_level());
        self.offered_by_peer = self.next_tx_level;
        if !h.valid {
            self.send_ctrl(REJECT, self.fmt, 0);
            self.set_state(State::Qrt);
            return;
        }
        self.registered = h.registered;
        self.link.comp = Compression::from_bits(h.comp);
        (self.host_kind, self.last_held) = (TurnKind::Partial, 0);
        self.host_since_rx.clear();
        self.link_up(now);
        self.core.host(HostEvent::Registered(self.registered));
        if self.link.disc_now {
            self.qrt();
        } else if self.cfg.host.chat {
            self.send_ctrl(IDLE, self.fmt, 0);
            self.enter(State::IdleSent);
        } else {
            self.hold_from = Some(now);
            self.send_ctrl(BREAK, self.fmt, 0);
            self.enter(State::Break);
        }
    }

    fn on_reply(&mut self, c: CtrlIn) {
        let Some((reply_bw, q)) = connect_reply_q(c.code) else { return };
        let bw = if reply_bw == Bandwidth::Hz2300 && self.bw == Bandwidth::Hz500 { self.bw } else { reply_bw };
        self.switch_bw(bw);
        let q = Record::clamped(q as usize, Record::MAX);
        self.link.comp = if self.cfg.host.registered { self.cfg.host.compression } else { Compression::Off };
        let n = match self.cfg.offer_level {
            Some(o) => o.min(self.start_cap(true)),
            None => self.adapt.level_from_snr(f64::from(self.carry.ctrl_snr_held), self.start_cap(true), 3.0),
        };
        self.level = q;
        self.upstep_from = q;
        self.carry.rx_level = n;
        self.next_tx_level = q;
        self.carry.offered = n;
        self.header = Some(HeaderTx { level: q, offer: n });
        self.set_state(State::HeaderSent);
        self.feed_watchdog(c.t_end);
        self.send_header();
    }

    fn send_header(&mut self) {
        let Some(HeaderTx { level: q, offer: n }) = self.header else { return };
        let h = Header {
            call: self.mycall.clone(),
            valid: true,
            registered: self.cfg.host.registered,
            comp: self.link.comp.bits(),
            reply: self.carry.reply_fmt.bits(),
            offer: n.get() as u8,
        };
        let fb = h.to_frame(self.bw.level(q).frame_bytes() as usize, queue_hint(self.bw, q, 0), self.carry.marker);
        self.fmt = format_for_level(self.bw, q);
        self.tx.final_sent = true;
        self.core.transmitting = true;
        self.link.deadline = None;
        self.core.out.push_back(Output::Tx(Tx::Data { level: q, frames: vec![fb] }));
    }

    fn ctrl_header_sent(&mut self, now: Time, c: CtrlIn) {
        match c.code {
            BREAK => {
                self.link_up(now);
                let fmt = format_for_level(self.bw, self.level);
                self.send_ctrl(start_level_code(self.carry.offered.get() as u32), fmt, 0);
                self.enter(State::Receiving);
            }
            IDLE => {
                self.link_up(now);
                self.answer_idle();
            }
            NACK => {
                if let Some(h) = &mut self.header {
                    h.level = h.level.down();
                    self.level = h.level;
                    self.next_tx_level = self.level;
                }
                self.req_fmt = None;
                self.send_header();
            }
            _ => {}
        }
    }

    fn link_up(&mut self, now: Time) {
        self.connected = true;
        self.link.start_files(&mut self.tx.buf);
        self.feed_watchdog(now);
        let (me, peer) = (self.mycall.clone(), self.remote.clone());
        let (caller, answerer) = if self.role == Role::Caller { (me, peer) } else { (peer, me) };
        self.core.host(HostEvent::Connected { caller, answerer, vias: Vec::new(), bw: LinkBw::Hf(self.bw) });
    }

    fn wants_qrt(&self) -> bool {
        self.link.wants_qrt(self.tx.buf.len())
    }

    fn answer_idle(&mut self) {
        if self.wants_qrt() {
            self.qrt();
        } else if self.tx.buf.is_empty() {
            self.send_ctrl(NO_TRAFFIC, self.fmt, 0);
            self.enter(State::Idle);
        } else {
            self.take_turn();
        }
    }

    fn take_turn(&mut self) {
        self.send_ctrl(BREAK, self.fmt, 0);
        self.enter(State::Break);
    }

    fn grant(&mut self) {
        self.send_ctrl(GO_AHEAD, self.fmt, 0);
        self.enter(State::Receiving);
    }

    fn ctrl_break(&mut self, now: Time, c: CtrlIn) {
        match c.code {
            code if code == GO_AHEAD || start_level(code).is_some() => {
                if let Some(n) = start_level(code) {
                    self.next_tx_level = Record::clamped(n as usize, Record::MAX);
                    self.offered_by_peer = self.next_tx_level;
                }
                if self.link.disc_now {
                    self.qrt();
                    return;
                }
                let t = TurnState {
                    answerer_hold: self.hold_from.is_some() && self.role == Role::Answerer,
                    chat: self.cfg.host.chat,
                    queued: self.tx.buf.len(),
                    since_header_ms: self.hold_from.map_or(0, |t| (now - t) * 1000 / SECOND),
                    kind: self.host_kind,
                    offered: self.offered_by_peer,
                    next_tx: self.next_tx_level,
                    text: self.link.text(),
                    last_held: self.last_held,
                };
                let (start, held) = turn_start(self.bw, &t);
                self.last_held = held;
                match start {
                    TurnStart::Hold => self.send_ctrl(BREAK, self.fmt, 0),
                    TurnStart::Idle => {
                        self.link.idle_ticks = 0;
                        self.send_ctrl(IDLE, self.fmt, 0);
                        self.enter(State::IdleSent);
                    }
                    TurnStart::Data(o) => {
                        self.hint_override = o;
                        self.level = self.next_tx_level.min(self.bw.top_level());
                        self.upstep_from = self.level;
                        self.hold_from = None;
                        self.enter(State::Sending);
                        self.transmit();
                    }
                }
            }
            BREAK if self.cfg.host.chat && self.role == Role::Answerer => self.grant(),
            _ => self.send_ctrl(BREAK, Fmt::F0, 0),
        }
    }

    fn transmit(&mut self) {
        let bw = self.bw;
        let level = self.level;
        let flen = bw.level(level).frame_bytes() as usize;
        let room = payload_len(bw, level);
        let nf = if self.tx.resend.is_none() { self.nframes() } else { 1 };
        let mut pos = 0;
        let mut frames = Vec::with_capacity(nf);
        self.tx.inflight.clear();
        self.tx.final_sent = false;
        let n = match self.cfg.offer_level {
            Some(o) => o.min(bw.top_level()),
            None => self.adapt.offer_level(self.carry.snr_ref_db, self.start_cap(false)),
        };
        let text = self.link.text();
        let mut state = self.tx.state.max(START_STATE);
        self.tx.states.clear();
        self.tx.buf.make_contiguous();
        for i in 0..nf {
            let rest = self.tx.buf.len() - pos;
            let limit = self.tx.resend.take().unwrap_or(usize::MAX);
            let lastable = nf == 1 || i == 1;
            let raw_last = lastable && rest < room && limit >= rest;
            let q = &self.tx.buf.as_slices().0[pos..];
            let squeeze = text && nf == 1 && !raw_last && *self.tx.text_turn.get_or_insert_with(|| turn_compresses(q));
            let Chunk { data, take, last, compressed } = if squeeze {
                let e = fill_frame(&q[..rest.min(limit)], room, state);
                state = e.end_state;
                let last = lastable && e.consumed == rest && e.bytes.len() < room;
                Chunk { last, compressed: true, take: e.consumed, data: e.bytes }
            } else if nf == 2 {
                Chunk::raw(q, rest.min(room), i == 1 && rest == 0)
            } else {
                Chunk::raw(q, rest.min(if raw_last { room - 1 } else { room }).min(limit), raw_last)
            };
            let after = &q[take..];
            let hint = if text && self.tx.text_turn == Some(true) {
                queue_hint_text(bw, level, after, state)
            } else {
                queue_hint(bw, level, after.len())
            };
            let hint = match std::mem::take(&mut self.hint_override) {
                HintOverride::Max => HINT_MAX,
                HintOverride::TableMinimum => hint.max(hint_table_minimum(bw, level, text)),
                HintOverride::Off => hint,
            };
            let ctrl = Control { reply: self.carry.reply_fmt.bits(), compressed, hint };
            frames.push(build_frame(flen, &data, ctrl, last.then_some(n.get() as u8), self.carry.marker));
            self.tx.inflight.push(take);
            self.tx.states.push(state);
            self.tx.final_sent |= last;
            pos += take;
        }
        if self.tx.final_sent {
            self.carry.offered = n;
            self.carry.rx_level = n;
        }
        self.fmt = format_for_level(bw, level);
        self.req_fmt = None;
        self.core.transmitting = true;
        self.link.deadline = None;
        self.core.out.push_back(Output::Tx(Tx::Data { level, frames }));
    }

    fn ctrl_sending(&mut self, c: CtrlIn) {
        let top = self.bw.top_level();
        let before = self.tx.buf.len();
        match (c.code, step(c.code)) {
            (code, Some(d)) => {
                for n in self.tx.accept_all() {
                    self.core.host(HostEvent::Buffer(n + self.link.files.len()));
                }
                let new = Record::clamped((self.level.get() as i64 + d).max(1) as usize, top);
                self.stays =
                    Some(if code == ACK_STAY && self.tx.text_turn != Some(true) { stay_bump(self.stays) } else { 0 });
                self.level_set(new);
            }
            (ACK_SECOND_ONLY, _) => {
                self.tx.second_only();
                self.stays = Some(0);
                let new = step_down(self.bw, self.level);
                self.level_set(new);
            }
            (ACK_FIRST_ONLY, _) => {
                self.tx.first_only();
                self.stays = Some(0);
                let new = step_down(self.bw, self.level);
                self.level_set(new);
            }
            (NACK | NACK_MISSED, _) => {
                self.tx.inflight.clear();
                let n = step_count(c.code, self.upsteps, self.level);
                let new = step_down_n(self.bw, self.level, n);
                self.stays = Some(0);
                if c.code == NACK_MISSED {
                    self.fmt = self.fmt.more_robust();
                }
                self.level_set(new);
            }
            (BREAK | IDLE, _) => {
                for n in self.tx.accept_all() {
                    self.core.host(HostEvent::Buffer(n + self.link.files.len()));
                }
                self.link.turn_quit(&mut self.tx.buf);
                if self.wants_qrt() {
                    self.qrt();
                } else if c.code == BREAK {
                    self.grant();
                } else {
                    self.answer_idle();
                }
                return;
            }
            (ACK_REQ_0 | ACK_REQ_1, _) => {
                self.resend_last();
                return;
            }
            _ => return,
        }
        if matches!(c.code, ACK_SECOND_ONLY | ACK_FIRST_ONLY) && self.tx.buf.len() != before {
            self.core.host(HostEvent::Buffer(self.buffered()));
        }
        if self.link.disc_now {
            self.qrt();
        } else {
            self.transmit();
        }
    }

    fn ctrl_receiving(&mut self, c: CtrlIn) {
        match c.code {
            ACK_REQ_0 | ACK_REQ_1 if c.code == self.ack_req() => {
                for _ in 0..self.nframes() {
                    self.adapt.frame_lost(self.level);
                }
                let n = step_count(NACK_MISSED, self.upsteps, self.level);
                let new = step_down_n(self.bw, self.level, n);
                self.alt = !self.alt;
                self.stays = Some(0);
                self.level_set(new);
                self.fallback = fallback_below(self.prev_level);
                self.send_ctrl(NACK_MISSED, self.fmt.more_robust(), 0);
                self.announce();
            }
            ACK_REQ_0 | ACK_REQ_1 | BREAK => self.resend_last(),
            IDLE => self.answer_idle(),
            _ => {}
        }
    }

    fn reply(&mut self, code: u32, new_level: Record) {
        let old = self.level;
        if code == ACK_STAY {
            self.stays = Some(if self.link.rx.turn_compressed == Some(true) { 0 } else { stay_bump(self.stays) });
        } else if new_level != old || matches!(code, NACK | ACK_SECOND_ONLY | ACK_FIRST_ONLY | NACK_MISSED) {
            self.stays = Some(0);
        }
        self.level_set(new_level);
        self.fallback = if code == NACK { fallback_below(old) } else { Vec::new() };
        if !matches!(code, BREAK | IDLE) {
            self.alt = !self.alt;
        }
        self.send_ctrl(code, self.fmt, 0);
    }

    pub fn on_data(&mut self, now: Time, d: DataIn) {
        if self.core.transmitting || d.frames.is_empty() {
            return;
        }
        if self.state == State::Answering {
            self.feed_watchdog(now);
            self.on_header(now, d);
            return;
        }
        if self.state != State::Receiving || d.t_end < self.core.tx_end {
            return;
        }
        self.feed_watchdog(now);
        if self.link.disc_now {
            self.qrt();
            return;
        }
        self.host_since_rx.clear();
        self.last_held = 0;
        let l = d.level;
        self.carry.rx_level = l;
        let two = d.frames.len() == 2;
        let good = self.parse_frames(&d.frames);
        if let Some(Some(g)) = good.first() {
            self.link.rx.turn_compressed.get_or_insert(g.meta.ctrl.compressed);
        }
        let ok: Vec<bool> = good.iter().map(Option::is_some).collect();
        let s = self.rx_stats(&d.frames, &ok, l);
        if two && !ok[0] {
            self.adapt.frame_lost(l);
        }
        if let Some(s) = s {
            self.carry.snr_ref_db = s;
        }
        let newest = good.iter().rev().flatten().next().map(|g| (g.meta, g.data == b"FQ\r"));
        let (Some((lastf, fq_data)), Some(s)) = (newest, s) else {
            self.adapt.frame_lost(l);
            let n = step_count(NACK, self.upsteps, l);
            self.level = l;
            let new = step_down_n(self.bw, l, n);
            self.reply(NACK, new);
            self.announce();
            return;
        };
        self.fmt = Fmt::from_bits(lastf.ctrl.reply);
        let skip = payload_len(self.bw, l);
        for (i, g) in good.iter().enumerate() {
            if let Some(g) = g {
                self.core.deliver(&mut self.link, &g.data, if i == 1 && !ok[0] { skip } else { 0 });
            }
        }
        self.level = l;
        if lastf.last && ok.iter().all(|&o| o) && (two || !self.two_frames()) {
            let fq = !two && !self.two_frames() && !lastf.full && fq_data;
            self.end_turn(lastf, fq);
        } else {
            self.ack_data(s, lastf.ctrl.hint, &ok);
        }
    }

    fn parse_frames(&mut self, frames: &[FrameIn]) -> Vec<Option<Good>> {
        frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let p = f.ok.then(|| parse(&f.bytes, self.carry.marker)).flatten()?;
                let meta = FrameMeta { ctrl: p.ctrl, last: p.last, full: p.full, next_level: p.next_level };
                let data = self.link.inflate(i == 1, meta.ctrl.compressed, p.data?)?;
                Some(Good { meta, data })
            })
            .collect()
    }

    fn end_turn(&mut self, lastf: FrameMeta, fq: bool) {
        self.next_tx_level = Record::clamped(usize::from(lastf.next_level.unwrap_or(1)), self.bw.top_level());
        self.offered_by_peer = self.next_tx_level;
        let code = if self.cfg.host.chat && self.tx.buf.is_empty() { IDLE } else { BREAK };
        if code == IDLE {
            self.link.idle_ticks = 0;
        }
        if self.wants_qrt() {
            self.qrt();
            return;
        }
        self.reply(code, self.level);
        self.enter(match code {
            BREAK if fq => State::FqWait,
            BREAK => State::Break,
            _ => State::IdleSent,
        });
    }

    fn ack_data(&mut self, s: f64, hint: u8, ok: &[bool]) {
        let (l, two) = (self.level, ok.len() == 2);
        if (two && ok[0] && !ok[1]) || (!two && self.two_frames()) {
            self.adapt.frame_lost(l);
            let new = step_down(self.bw, l);
            self.reply(ACK_FIRST_ONLY, new);
        } else {
            let complete = ok.iter().all(|&o| o) && ok.len() >= self.nframes();
            let m = self.adapt.margins(s, l);
            let d = ack_decision(self.bw, l, &m, hint, complete, self.max_level());
            if d.stay_reset {
                self.stays = None;
            }
            self.reply(d.code, d.level);
        }
        if self.link.disc_now {
            self.link.deadline = None;
        }
        self.announce();
    }

    fn rx_stats(&mut self, frames: &[FrameIn], ok: &[bool], level: Record) -> Option<f64> {
        let mut s = None;
        for (i, f) in frames.iter().enumerate().filter(|&(i, _)| ok[i]) {
            s = Some(self.trend.push(f.snr_db));
            self.adapt.frame_ok(level);
            if i + 1 == frames.len() {
                self.core.host(HostEvent::Sn(f.snr_db));
            }
        }
        s
    }
}

fn stay_bump(s: Option<u32>) -> u32 {
    s.map_or(0, |n| n + 1)
}

fn fallback_below(level: Record) -> Vec<Record> {
    (1..=3).filter_map(|d| level.get().checked_sub(d).and_then(Record::new)).collect()
}
