use crate::compression::Compression;
use crate::fm::adapt::{
    Adaptation, HINT_MAX, Heard, SNR_OFFSET_DB, as_level, decide, max_level, payload, queue_hint, queue_hint_text,
    recommend_level, sender_level_after, step_down,
};
use crate::fm::timing::{
    CODE_GATE, DETECT_HOLD, FQ_WAIT_REPLIES, IDLE_DATA_WAIT, TICK, Timing, caller_connect_window, receiver_sync_window,
    state2_backoff, watchdog,
};
use crate::host::{HostEvent, HostSettings, LinkBw, QRT_RECEIVED};
use crate::link::Link;
use crate::session::Core;
use crate::textcomp::{START_STATE, fill_frame, turn_compresses};
use crate::time::{DISC_TIMEOUT_LIMIT, IDLE_POLL, IDLE_POLL_LIMIT, Time};
use std::collections::VecDeque;
use xara_phy::call::call_hash;
use xara_phy::fm::ctrl::code::*;
use xara_phy::fm::ctrl::{LEAD_ROWS, LeadIn};
use xara_phy::fm::frame::{DataFrame, Kind, Record0, Trailer, build_data, parse_data};
use xara_phy::fm::level::{DataLevel, FmBandwidth, Record};
use xara_phy::fm::tx::CONNECT_LEAD_IN;

pub(crate) const RX_GRACE: Time = 2400;
const REPEAT_LIMIT: u32 = 100;
pub const NO_LEAD_HEARD: u8 = LEAD_ROWS as u8 + 1;

#[derive(Clone, Debug)]
pub struct Config {
    pub bw: FmBandwidth,
    pub host: HostSettings,
    pub seed: u64,
}

impl Config {
    pub fn new(bw: FmBandwidth, mycall: &str) -> Self {
        Self { bw, host: HostSettings::new(mycall, 10), seed: 1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Listen,
    Receiving,
    AwaitGo,
    Calling,
    Sending,
    Qrt,
    IdleSent,
    Idle,
    FqWait,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tx {
    Ctrl { code: u32, lead: usize },
    Data { record: Record, frames: Vec<Vec<u8>>, lead: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listen {
    pub bw: FmBandwidth,
    pub key: u32,
    pub ctrl: Option<(u32, u32)>,
    pub codes: Vec<u32>,
    pub records: Vec<Record>,
    pub nframes: usize,
    pub linked: bool,
}

pub type Output = crate::session::Output<Tx, Listen>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CtrlIn {
    pub code: u32,
    pub lead_row: u8,
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
    pub record: Record,
    pub frames: Vec<FrameIn>,
    pub t_end: Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastTx {
    None,
    Ctrl,
    Req,
    Data(Record),
}

#[derive(Default, Debug)]
struct TxBuf {
    buf: VecDeque<u8>,
    inflight: Vec<usize>,
    states: Vec<usize>,
    state: usize,
    text_turn: Option<bool>,
    last_hint: u8,
}

impl TxBuf {
    fn take(&mut self, keep: &[bool]) -> Vec<usize> {
        let inflight = std::mem::take(&mut self.inflight);
        let mut left = Vec::new();
        let mut pos = 0;
        for (i, &n) in inflight.iter().enumerate() {
            if keep.get(i).copied().unwrap_or(false) {
                self.buf.drain(pos..(pos + n).min(self.buf.len()));
                if n > 0 {
                    left.push(self.buf.len());
                }
                if let Some(&s) = self.states.get(i) {
                    self.state = s;
                }
            } else {
                pos += n;
            }
        }
        self.states.clear();
        left
    }
}

pub struct Session {
    core: Core<Tx, Listen>,
    cfg: Config,
    bw: FmBandwidth,
    state: State,
    caller: String,
    answerer: String,
    mycall: String,
    vias: Vec<String>,
    key_d: u32,
    key_u: u32,
    connected: bool,
    registered: bool,
    first_turn: bool,

    level: Record,
    prev_level: Record,
    offer_rx: Option<DataLevel>,
    offer_tx: Option<DataLevel>,
    stays: u32,
    stay_level: Option<DataLevel>,
    two: bool,
    adapt: Adaptation,
    nack_run: u32,

    alt: bool,
    last_code: Option<u32>,
    last_tx: LastTx,
    lead: LeadIn,
    heard_row: u8,
    attempts: u32,
    repeats: u32,

    timing: Timing,
    window: Option<i64>,

    qrt_left: u32,

    tx: TxBuf,
    link: Link,
    last_held: usize,
    data_stored: bool,
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
            mycall: cfg.host.primary_call(),
            cfg,
            bw,
            state: State::Listen,
            caller: String::new(),
            answerer: String::new(),
            vias: Vec::new(),
            key_d: 0,
            key_u: 0,
            connected: false,
            registered,
            level: Record::Start,
            prev_level: Record::Start,
            offer_rx: None,
            offer_tx: None,
            alt: false,
            stays: 0,
            stay_level: None,
            two: false,
            adapt: Adaptation::default(),
            timing: Timing::new(bw, 0),
            lead: LeadIn::default(),
            heard_row: NO_LEAD_HEARD,
            tx: TxBuf::default(),
            last_code: None,
            last_tx: LastTx::None,
            window: None,
            attempts: 0,
            qrt_left: 0,
            nack_run: 0,
            repeats: 0,
            first_turn: false,
            data_stored: false,
            last_held: 0,
            link: Link::default(),
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn level(&self) -> Record {
        self.level
    }

    fn ack_req(&self) -> u32 {
        if self.alt { ACK_REQ_1 } else { ACK_REQ_0 }
    }

    pub fn peer(&self) -> &str {
        if self.caller == self.mycall { &self.answerer } else { &self.caller }
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn pop(&mut self) -> Option<Output> {
        self.core.pop()
    }

    pub fn bandwidth(&self) -> FmBandwidth {
        self.bw
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.cfg
    }

    pub fn key(&self) -> u32 {
        self.key_d
    }

    pub fn vias(&self) -> usize {
        self.vias.len()
    }

    pub fn buffered(&self) -> usize {
        self.tx.buf.len() + self.link.files.len()
    }

    pub fn next_deadline(&self) -> Option<Time> {
        self.core.next_deadline(&self.link)
    }

    fn top(&self) -> DataLevel {
        max_level(self.bw, self.registered)
    }

    fn codes(&self) -> &'static [u32] {
        match self.state {
            State::Listen | State::Qrt => &[],
            State::Calling => &[QRT, CONNECT_REPLY_NARROW, CONNECT_REPLY_WIDE],
            State::Receiving if self.data_stored => &[QRT, BREAK, ACK_REQ_0, ACK_REQ_1, IDLE, RELAY_FAILED],
            State::Receiving => {
                &[QRT, BREAK, ACK_REQ_0, ACK_REQ_1, IDLE, RELAY_FAILED, CONNECT_REPLY_NARROW, CONNECT_REPLY_WIDE]
            }
            State::AwaitGo => &[QRT, ACK_REQ_0, ACK_REQ_1, GO_AHEAD, IDLE],
            State::Sending => &[
                ACK_STAY,
                NACK,
                QRT,
                BREAK,
                ACK_UP2,
                ACK_DOWN,
                ACK_UP1,
                ACK_UP3,
                ACK_UP6,
                ACK_SECOND_ONLY,
                ACK_FIRST_ONLY,
                IDLE,
            ],
            State::IdleSent => &[QRT, BREAK, ACK_REQ_0, ACK_REQ_1, NO_TRAFFIC],
            State::Idle => &[QRT, BREAK, IDLE],
            State::FqWait => &[QRT, ACK_REQ_0, ACK_REQ_1],
        }
    }

    fn records(&self) -> Vec<Record> {
        match self.state {
            State::Listen => vec![Record::Connect],
            State::AwaitGo if self.first_turn => vec![Record::Connect],
            State::Receiving => {
                let mut v: Vec<Record> =
                    [Record::Start, Record::Short, self.level, self.prev_level, DataLevel::ONE.into()]
                        .into_iter()
                        .filter(|&r| match r {
                            Record::Start | Record::Short => true,
                            Record::Data(l) => l <= self.top(),
                            Record::Connect | Record::Broadcast => false,
                        })
                        .collect();
                v.sort_unstable();
                v.dedup();
                v
            }
            _ => Vec::new(),
        }
    }

    fn announce(&mut self) {
        let listening = self.state == State::Listen;
        let codes = self.codes().to_vec();
        let l = Listen {
            bw: self.bw,
            key: if listening { 0 } else { self.key_d },
            ctrl: (!listening).then_some((self.key_d, self.key_u)),
            codes,
            records: self.records(),
            nframes: if self.state == State::Receiving { self.nframes() } else { 1 },
            linked: self.connected,
        };
        self.core.out.push_back(Output::Listen(l));
    }

    fn enter(&mut self, s: State) {
        if s != self.state {
            self.alt = false;
            self.stays = 0;
            self.two = false;
            if s == State::Sending {
                self.tx.text_turn = None;
                self.tx.state = START_STATE;
            }
            if s == State::Receiving {
                self.link.rx.turn_compressed = None;
                let start = self.offer_tx.take().map_or(Record::Start, Record::Data);
                (self.level, self.prev_level) = (start, start);
            }
            self.state = s;
        }
        self.announce();
    }

    fn set_state(&mut self, s: State) {
        self.state = s;
        self.announce();
    }

    fn listen(&mut self) {
        let fresh = Self::fresh(self.cfg.clone());
        self.core = std::mem::replace(self, fresh).core;
        self.announce();
    }

    fn close(&mut self) {
        if self.connected || self.state == State::Calling {
            self.core.host(HostEvent::Disconnected);
        }
        self.core.host(HostEvent::CancelPending);
        self.listen();
    }

    fn feed_watchdog(&mut self, now: Time) {
        self.link.watchdog = Some(now + watchdog(self.vias.len()));
    }

    fn send_ctrl(&mut self, code: u32) {
        if !ACK_REQ.contains(&code) {
            self.last_code = Some(code);
        }
        self.last_tx = if ACK_REQ.contains(&code) { LastTx::Req } else { LastTx::Ctrl };
        self.core.transmitting = true;
        (self.link.deadline, self.window, self.link.idle_timer) = (None, None, None);
        let lead = usize::from(self.lead.len);
        self.core.out.push_back(Output::Tx(Tx::Ctrl { code, lead }));
    }

    fn say(&mut self, code: u32, next: State) {
        self.send_ctrl(code);
        self.enter(next);
    }

    fn resend_last(&mut self) {
        self.repeats += 1;
        if let Some(c) = self.last_code {
            self.send_ctrl(c);
        }
    }

    fn wants_qrt(&self) -> bool {
        self.link.wants_qrt(self.tx.buf.len())
    }

    fn qrt(&mut self) {
        self.qrt_left = u32::from(self.vias.is_empty() && self.connected);
        self.send_ctrl(QRT);
        self.set_state(State::Qrt);
    }

    pub fn connect(&mut self, now: Time, mycall: Option<&str>, remote: &str, vias: &[String]) -> bool {
        if self.state != State::Listen || vias.len() > 2 {
            return false;
        }
        self.mycall = mycall.map_or_else(|| self.cfg.host.primary_call(), str::to_string);
        self.caller = self.mycall.clone();
        self.answerer = remote.to_string();
        self.vias = vias.to_vec();
        let d = self.vias.first().map_or(remote, String::as_str);
        (self.key_d, self.key_u) = (call_hash(d.as_bytes()), call_hash(self.mycall.as_bytes()));
        self.timing = Timing::new(self.bw, self.vias.len());
        self.link.comp = if self.cfg.host.registered { self.cfg.host.compression } else { Compression::Off };
        self.lead.new_session();
        self.attempts = 0;
        self.set_state(State::Calling);
        self.feed_watchdog(now);
        self.send_connect();
        true
    }

    fn send_connect(&mut self) {
        self.attempts += 1;
        let via = |i: usize| self.vias.get(i).cloned().unwrap_or_default();
        let wide = self.cfg.bw == FmBandwidth::Wide;
        let mut r = Record0::connect(&self.caller, &self.answerer, &via(0), &via(1), self.cfg.host.registered, wide);
        r.compression = self.link.comp.bits();
        let Some(bytes) = r.to_bytes() else {
            self.core.host(HostEvent::Disconnected);
            self.listen();
            return;
        };
        self.core.host(HostEvent::Unencrypted);
        self.last_tx = LastTx::Data(Record::Connect);
        self.core.transmitting = true;
        (self.link.deadline, self.window) = (None, None);
        let lead = CONNECT_LEAD_IN;
        self.core.out.push_back(Output::Tx(Tx::Data { record: Record::Connect, frames: vec![bytes], lead }));
    }

    pub fn send(&mut self, now: Time, data: &[u8]) {
        self.link.queue(now, self.connected, data, &mut self.tx.buf);
        self.idle_wake(now);
    }

    fn idle_wake(&mut self, now: Time) {
        if self.state == State::Idle && !self.core.transmitting && self.link.deadline.is_none() && self.buffered() > 0 {
            self.link.deadline = Some(now.max(self.core.tx_end + IDLE_DATA_WAIT * TICK));
        }
    }

    pub fn disconnect(&mut self, now: Time) {
        if self.buffered() > 0 && self.connected {
            self.link.disc_after_tx = true;
        } else {
            self.abort(now);
        }
        self.disc_wake(now);
    }

    pub fn abort(&mut self, now: Time) {
        if matches!(self.state, State::Listen | State::Qrt) {
            return;
        }
        self.link.disc_now = true;
        self.disc_wake(now);
    }

    fn disc_wake(&mut self, now: Time) {
        if self.core.transmitting || !self.wants_qrt() {
            return;
        }
        match self.state {
            State::Idle => self.link.deadline = Some(now.max(self.core.tx_end + IDLE_DATA_WAIT * TICK)),
            State::Receiving => self.link.deadline = Some(now.max(self.sync_end())),
            _ => {}
        }
    }

    fn sync_end(&self) -> Time {
        let (_, hi) = receiver_sync_window(self.bw, self.level, self.vias.len(), self.two);
        self.core.tx_end + hi as i64 * TICK
    }

    pub fn drop_link(&mut self) {
        match self.state {
            State::Listen => {}
            State::Qrt => {}
            _ => {
                self.link.disc_now = true;
                if !self.core.transmitting {
                    self.close();
                }
            }
        }
    }

    pub fn on_tx_done(&mut self, now: Time) {
        self.core.transmitting = false;
        self.core.tx_end = now;
        let vias = self.vias.len();
        let w = match self.state {
            State::Calling => Some(caller_connect_window(self.bw, vias, false)),
            State::Sending => Some(match self.last_tx {
                LastTx::Data(r) => self.timing.data_window(r),
                _ => self.timing.ctrl_window(),
            }),
            State::IdleSent => Some(self.timing.ctrl_window()),
            State::FqWait => Some(FQ_WAIT_REPLIES * self.timing.ctrl_window()),
            State::AwaitGo => {
                let m = self.core.rng.upto(2) + 1;
                Some(state2_backoff(vias, m))
            }
            State::Receiving if self.wants_qrt() => Some((self.sync_end() - now) / TICK),
            State::Idle if self.wants_qrt() || self.buffered() > 0 => Some(IDLE_DATA_WAIT),
            State::Qrt => {
                if self.qrt_left > 0 {
                    self.qrt_left -= 1;
                    self.send_ctrl(QRT);
                } else {
                    self.close();
                }
                None
            }
            _ => None,
        };
        self.window = w;
        self.link.deadline = w.map(|t| now + t * TICK + RX_GRACE);
    }

    pub fn poll(&mut self, now: Time) {
        if self.core.transmitting {
            return;
        }
        if self.link.watchdog_fired(now) {
            self.core.host(HostEvent::TimedOut);
            self.close();
            return;
        }
        if self.link.files.due(now) {
            self.link.files.flush(&mut self.tx.buf);
            self.idle_wake(now);
        }
        if let Some(ticks) = self.link.idle_fired(now) {
            if !self.cfg.host.chat && ticks >= IDLE_POLL_LIMIT {
                self.core.host(HostEvent::Log("stopped sending due to CMS failure".into()));
                self.qrt();
            } else {
                self.say(IDLE, State::IdleSent);
            }
            return;
        }
        if !self.link.deadline_fired(now) {
            return;
        }
        self.window = None;
        if self.repeats >= REPEAT_LIMIT {
            self.core.host(HostEvent::TimedOut);
            self.close();
            return;
        }
        match self.state {
            State::Calling => {
                if self.link.disc_now {
                    self.close();
                } else if self.attempts < self.cfg.host.retries {
                    self.send_connect();
                } else {
                    self.core.host(HostEvent::UnableToConnect);
                    self.close();
                }
            }
            State::Sending => {
                if self.wants_qrt() {
                    self.link.disc_timeouts += 1;
                    self.core.host(HostEvent::Log(format!(
                        "disconnecting {}/{DISC_TIMEOUT_LIMIT}",
                        self.link.disc_timeouts
                    )));
                    if self.link.disc_timeouts >= DISC_TIMEOUT_LIMIT {
                        self.qrt();
                        return;
                    }
                }
                self.repeats += 1;
                self.timing.miss();
                let a = self.ack_req();
                self.send_ctrl(a);
            }
            State::IdleSent | State::AwaitGo => {
                self.timing.miss();
                self.resend_last();
            }
            State::FqWait => {
                self.core.host(HostEvent::Disconnected);
                self.core.host(HostEvent::CancelPending);
                self.listen();
            }
            State::Receiving if self.wants_qrt() => self.qrt(),
            State::Idle if self.wants_qrt() || self.buffered() > 0 => {
                self.say(BREAK, State::AwaitGo);
            }
            _ => {}
        }
    }

    pub(crate) fn code_ticks(&self, t_end: Time) -> i64 {
        (t_end - self.core.tx_end).div_euclid(TICK) + DETECT_HOLD
    }

    pub fn on_ctrl(&mut self, now: Time, c: CtrlIn) {
        if self.core.transmitting || !self.codes().contains(&c.code) {
            return;
        }
        let ticks = self.code_ticks(c.t_end);
        if ticks < CODE_GATE || self.window.is_some_and(|w| ticks > w) {
            return;
        }
        if self.state != State::Calling {
            self.feed_watchdog(now);
        }
        self.heard_row = c.lead_row;
        match self.last_tx {
            LastTx::Data(r) if !(c.code == NACK && !self.vias.is_empty()) => self.timing.ack(ticks, r),
            LastTx::Ctrl if !(self.state == State::Sending && matches!(c.code, BREAK | GO_AHEAD)) => {
                self.timing.ctrl(ticks)
            }
            _ => {}
        }
        if c.code == QRT {
            self.core.host(HostEvent::Log(QRT_RECEIVED.into()));
            self.close();
            return;
        }
        match self.state {
            State::Calling => self.on_connect_reply(now, c.code),
            State::Receiving => self.ctrl_receiving(c.code),
            State::AwaitGo => self.ctrl_await_go(c.code),
            State::Sending => self.ctrl_sending(c.code),
            State::IdleSent => match c.code {
                NO_TRAFFIC if self.wants_qrt() => self.qrt(),
                NO_TRAFFIC if self.buffered() == 0 => {
                    self.repeats = 0;
                    self.enter(State::Idle);
                    self.link.idle_timer = Some(now + IDLE_POLL);
                }
                NO_TRAFFIC => {
                    self.say(BREAK, State::AwaitGo);
                }
                BREAK => self.grant(),
                _ => self.resend_last(),
            },
            State::Idle => match c.code {
                IDLE => self.answer_poll(),
                BREAK => self.grant(),
                _ => {}
            },
            State::FqWait => self.resend_last(),
            State::Listen | State::Qrt => {}
        }
    }

    fn grant(&mut self) {
        self.repeats = 0;
        self.say(GO_AHEAD, State::Receiving);
    }

    fn answer_poll(&mut self) {
        self.repeats = 0;
        if self.wants_qrt() || self.buffered() > 0 {
            self.say(BREAK, State::AwaitGo);
        } else {
            self.say(NO_TRAFFIC, State::Idle);
        }
    }

    fn on_connect_reply(&mut self, now: Time, code: u32) {
        if !matches!(code, CONNECT_REPLY_NARROW | CONNECT_REPLY_WIDE) {
            return;
        }
        self.bw = if code == CONNECT_REPLY_WIDE { FmBandwidth::Wide } else { FmBandwidth::Narrow };
        if self.bw != self.cfg.bw {
            let t = std::mem::replace(&mut self.timing, Timing::new(self.bw, self.vias.len()));
            self.timing.ack(t.mean[Record::Connect.index()], Record::Connect);
        }
        self.lead.new_session();
        self.connected = true;
        self.link.start_files(&mut self.tx.buf);
        self.feed_watchdog(now);
        self.core.host(HostEvent::Pending);
        let (caller, answerer, vias, bw) = (self.caller.clone(), self.answerer.clone(), self.vias.clone(), self.bw);
        self.core.host(HostEvent::Connected { caller, answerer, vias, bw: LinkBw::Fm(bw) });
        if self.link.disc_now {
            self.qrt();
            return;
        }
        self.grant();
    }

    pub fn on_record0(&mut self, now: Time, bytes: &[u8], snr_db: f64) {
        let Some(r) = Record0::parse(bytes) else { return };
        if r.kind != Kind::Connect || usize::from(r.hop) != r.vias() {
            return;
        }
        let Some(me) = self.cfg.host.mycalls.iter().find(|c| c.eq_ignore_ascii_case(&r.target)).cloned() else {
            return;
        };
        match self.state {
            State::Listen => {}
            State::AwaitGo if self.first_turn && r.source == self.caller => {
                self.feed_watchdog(now);
                self.resend_last();
                return;
            }
            _ => return,
        }
        self.mycall = me.clone();
        (self.caller, self.answerer) = (r.source.clone(), me.clone());
        self.vias = [&r.via1, &r.via2].into_iter().filter(|v| !v.is_empty()).cloned().collect();
        let up = self.vias.last().cloned().unwrap_or_else(|| r.source.clone());
        (self.key_d, self.key_u) = (call_hash(me.as_bytes()), call_hash(up.as_bytes()));
        self.registered = r.registered;
        self.link.comp = Compression::from_bits(r.compression);
        self.lead.new_session();
        self.feed_watchdog(now);
        self.core.host(HostEvent::Unencrypted);
        let sn = if self.vias.is_empty() { snr_db + SNR_OFFSET_DB } else { f64::from(r.chain_snr) / 7.0 };
        self.core.host(HostEvent::Sn(sn));
        self.core.host(HostEvent::Pending);
        if !self.vias.is_empty() && !r.registered {
            self.timing = Timing::new(self.cfg.bw, self.vias.len());
            self.qrt();
            return;
        }
        self.bw = if r.wide && self.cfg.bw == FmBandwidth::Wide { FmBandwidth::Wide } else { FmBandwidth::Narrow };
        self.timing = Timing::new(self.bw, self.vias.len());
        self.connected = true;
        self.first_turn = true;
        self.link.start_files(&mut self.tx.buf);
        let (caller, answerer, vias, bw) = (self.caller.clone(), self.answerer.clone(), self.vias.clone(), self.bw);
        self.core.host(HostEvent::Connected { caller, answerer, vias, bw: LinkBw::Fm(bw) });
        self.core.host(HostEvent::Registered(self.registered));
        self.say(if self.bw == FmBandwidth::Wide { CONNECT_REPLY_WIDE } else { CONNECT_REPLY_NARROW }, State::AwaitGo);
    }

    fn ctrl_await_go(&mut self, code: u32) {
        match code {
            GO_AHEAD => {
                self.repeats = 0;
                if self.wants_qrt() {
                    self.qrt();
                    return;
                }
                if std::mem::take(&mut self.first_turn) {
                    self.start_turn();
                    return;
                }
                let q = self.buffered();
                if q == 0 {
                    self.link.idle_ticks = 0;
                    self.say(IDLE, State::IdleSent);
                } else if self.offer_rx.is_some_and(|l| q <= payload(self.bw, l, !self.vias.is_empty()))
                    && q != self.last_held
                {
                    self.last_held = q;
                    self.send_ctrl(BREAK);
                    self.announce();
                } else {
                    self.start_turn();
                }
            }
            IDLE => self.answer_poll(),
            _ => self.resend_last(),
        }
    }

    pub fn has_vias(&self) -> bool {
        !self.vias.is_empty()
    }

    fn start_turn(&mut self) {
        self.link.files.flush(&mut self.tx.buf);
        let start = self.offer_rx.take();
        self.enter(State::Sending);
        self.level = start.map_or(Record::Start, |l| l.min(self.top()).into());
        self.prev_level = self.level;
        self.offer_tx = self.link.b2f.fs().then(|| recommend_level(self.bw, &self.adapt, self.registered)).flatten();
        self.transmit();
    }

    fn transmit(&mut self) {
        let (bw, rec, vias) = (self.bw, self.level, !self.vias.is_empty());
        let flen = bw.level(rec).frame_bytes();
        let p = payload(bw, rec, vias);
        let text = self.link.text();
        let nf = self.nframes();
        let level_byte = match (rec, self.offer_tx) {
            (Record::Start, Some(l)) => l.get() as u8,
            _ => 0,
        };
        self.tx.buf.make_contiguous();
        self.tx.inflight.clear();
        self.tx.states.clear();
        let mut state = self.tx.state.max(START_STATE);
        let mut pos = 0;
        let mut frames = Vec::with_capacity(nf);
        for i in 0..nf {
            let q = &self.tx.buf.as_slices().0[pos..];
            let rest = q.len();
            let squeeze = text && nf == 1 && rest > p && *self.tx.text_turn.get_or_insert_with(|| turn_compresses(q));
            let (data, take) = if squeeze {
                let e = fill_frame(q, p, state);
                state = e.end_state;
                (e.bytes, e.consumed)
            } else {
                let t = rest.min(p);
                (q[..t].to_vec(), t)
            };
            let after = &q[take..];
            let last = after.is_empty() && (nf == 1 || i == 1);
            let hint = if level_byte != 0 {
                HINT_MAX
            } else if self.tx.text_turn == Some(true) {
                queue_hint_text(bw, rec, after, state, vias)
            } else {
                queue_hint(bw, rec, after.len(), vias)
            };
            let trailer = Trailer { lead_report: self.heard_row, level: level_byte, chain_snr: vias.then_some(0) };
            frames.push(build_data(rec, flen, &data, squeeze, hint, last, trailer));
            self.tx.inflight.push(take);
            self.tx.states.push(state);
            self.tx.last_hint = hint;
            pos += take;
        }
        if rec != Record::Start {
            self.offer_tx = None;
        }
        self.last_tx = LastTx::Data(rec);
        self.core.transmitting = true;
        (self.link.deadline, self.window) = (None, None);
        let lead = self.lead.data_symbols();
        self.core.out.push_back(Output::Tx(Tx::Data { record: rec, frames, lead }));
    }

    fn compressed_turn(&self) -> bool {
        match self.state {
            State::Sending => self.tx.text_turn == Some(true),
            _ => self.link.rx.turn_compressed == Some(true),
        }
    }

    fn count_frame(&mut self, record: Record) {
        let l = Some(as_level(record));
        if self.compressed_turn() {
            self.stays = 0;
        } else if l > self.stay_level {
            self.stays = 1;
        } else {
            self.stays += 1;
        }
        self.stay_level = l;
    }

    fn nframes(&self) -> usize {
        if self.two { 2 } else { 1 }
    }

    fn update_two(&mut self) {
        self.two = self.vias.is_empty() && self.stays > 1 && !self.compressed_turn();
    }

    fn accept(&mut self, keep: &[bool]) {
        let rec = self.level;
        let n = self.tx.inflight.len();
        for i in 0..n {
            if keep.get(i).copied().unwrap_or(false) {
                self.count_frame(rec);
            } else {
                self.stays = 0;
            }
        }
        for left in self.tx.take(keep) {
            let n = left + self.link.files.len();
            self.core.host(HostEvent::Buffer(n));
        }
    }

    fn ctrl_sending(&mut self, code: u32) {
        self.alt = !self.alt;
        self.repeats = 0;
        let was = self.level;
        match code {
            ACK_STAY | ACK_UP2 | ACK_DOWN | ACK_UP1 | ACK_UP3 | ACK_UP6 => self.accept(&[true, true]),
            ACK_SECOND_ONLY => self.accept(&[false, true]),
            ACK_FIRST_ONLY => self.accept(&[true, false]),
            NACK => self.accept(&[]),
            BREAK | IDLE => {
                self.accept(&[true, true]);
                self.link.turn_quit(&mut self.tx.buf);
                if self.wants_qrt() {
                    self.qrt();
                } else if code == BREAK {
                    self.grant();
                } else {
                    self.answer_poll();
                }
                return;
            }
            _ => return,
        }
        if code == NACK && as_level(was) == DataLevel::ONE {
            self.nack_run += 1;
        } else {
            self.nack_run = 0;
        }
        let h = self.tx.last_hint;
        if matches!(code, ACK_UP2 | ACK_UP1 | ACK_UP3 | ACK_UP6 | ACK_FIRST_ONLY)
            || code == ACK_DOWN && h == 0
            || code == ACK_STAY && h == 2
        {
            self.stays = 0;
        }
        self.prev_level = was;
        self.level = match sender_level_after(was, code) {
            r @ (Record::Short | Record::Start) => r,
            Record::Data(l) => l.min(self.bw.top_level()).into(),
            Record::Connect | Record::Broadcast => self.bw.top_level().into(),
        };
        self.update_two();
        if self.nack_run >= 4 {
            self.core.host(HostEvent::Log("disconnecting due to poor conditions".into()));
            self.qrt();
        } else if self.wants_qrt() {
            self.qrt();
        } else {
            self.transmit();
        }
    }

    fn ctrl_receiving(&mut self, code: u32) {
        match code {
            ACK_REQ_0 | ACK_REQ_1 | RELAY_FAILED if code == self.ack_req() || code == RELAY_FAILED => {
                self.alt = !self.alt;
                for _ in 0..if code == RELAY_FAILED { 1 } else { self.nframes() } {
                    self.adapt.frame_lost();
                }
                self.stays = 0;
                self.two = false;
                self.prev_level = self.level;
                self.level = step_down(self.level).into();
                self.send_ctrl(NACK);
                self.announce();
            }
            ACK_REQ_0 | ACK_REQ_1 => self.resend_last(),
            BREAK | CONNECT_REPLY_NARROW | CONNECT_REPLY_WIDE => self.send_ctrl(GO_AHEAD),
            IDLE => self.answer_poll(),
            _ => {}
        }
    }

    pub fn on_data(&mut self, now: Time, d: DataIn) {
        if self.core.transmitting || d.frames.is_empty() || d.t_end < self.core.tx_end {
            return;
        }
        if d.record == Record::Connect {
            if let Some(f) = d.frames.iter().find(|f| f.ok) {
                self.on_record0(now, &f.bytes, f.snr_db);
            }
            return;
        }
        if self.state != State::Receiving || !self.records().contains(&d.record) {
            return;
        }
        self.feed_watchdog(now);
        if self.wants_qrt() {
            self.qrt();
            return;
        }
        self.receive(d);
    }

    fn receive(&mut self, d: DataIn) {
        let (rec, vias) = (d.record, !self.vias.is_empty());
        let two = d.frames.len() == 2;
        let parsed = self.parse_frames(&d);
        if let Some(Some(p)) = parsed.first() {
            self.link.rx.turn_compressed.get_or_insert(p.control.compressed);
        }
        let ok: Vec<bool> = parsed.iter().map(Option::is_some).collect();
        self.data_stored = true;
        self.alt = !self.alt;
        self.repeats = 0;
        let lastok = ok.iter().rposition(|&o| o);
        let held = self.rx_stats(&d, &parsed, lastok);
        let skip = payload(self.bw, rec, vias);
        for (i, p) in parsed.iter().enumerate() {
            if let Some(p) = p {
                self.core.deliver(&mut self.link, &p.data, if i == 1 && !ok[0] { skip } else { 0 });
            }
        }
        let (Some(li), Some(s)) = (lastok, held) else {
            self.step_back(rec, NACK);
            return;
        };
        let lastf = parsed[li].as_ref().unwrap();
        if (two && ok[0] && !ok[1]) || (!two && self.two && ok[0]) {
            if !two {
                self.adapt.frame_lost();
                self.stays = 0;
            }
            self.step_back(rec, ACK_FIRST_ONLY);
            return;
        }
        if lastf.control.last && ok.iter().all(|&o| o) {
            let fq = !two && !lastf.control.full && lastf.data == b"FQ\r";
            self.end_turn(rec, lastf.trailer.level, fq);
            return;
        }
        let h = Heard { record: rec, hint: lastf.control.hint, complete: ok[0] };
        let dec = decide(self.bw, &self.adapt, h, s, self.registered);
        if dec.stay_reset {
            self.stays = 0;
        }
        self.prev_level = rec;
        self.level = dec.level;
        self.update_two();
        self.send_ctrl(dec.code);
        self.announce();
    }

    fn parse_frames(&mut self, d: &DataIn) -> Vec<Option<DataFrame>> {
        let vias = !self.vias.is_empty();
        d.frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut p = f.ok.then(|| parse_data(d.record, &f.bytes, vias)).flatten()?;
                p.data = self.link.inflate(i == 1, p.control.compressed, std::mem::take(&mut p.data))?;
                Some(p)
            })
            .collect()
    }

    fn rx_stats(&mut self, d: &DataIn, parsed: &[Option<DataFrame>], lastok: Option<usize>) -> Option<f64> {
        let (rec, n) = (d.record, self.bw.top_level().get());
        let mut held = None;
        for (i, p) in parsed.iter().enumerate() {
            match p {
                Some(p) => {
                    self.lead.report(p.trailer.lead_report);
                    let snr = d.frames[i].snr_db + SNR_OFFSET_DB;
                    held = Some(self.adapt.frame_ok(snr, as_level(rec), n));
                    self.count_frame(rec);
                    if Some(i) == lastok {
                        self.core.host(HostEvent::Sn(snr));
                    }
                    self.core.host(HostEvent::RxFrame(rec));
                }
                None => {
                    self.adapt.frame_lost();
                    self.stays = 0;
                }
            }
        }
        held
    }

    fn step_back(&mut self, rec: Record, code: u32) {
        self.prev_level = rec;
        self.level = step_down(rec).into();
        self.two = false;
        self.send_ctrl(code);
        self.announce();
    }

    fn end_turn(&mut self, rec: Record, offer: u8, fq: bool) {
        if rec == Record::Start {
            self.offer_rx = DataLevel::new(usize::from(offer).min(self.top().get()));
        }
        if self.wants_qrt() {
            self.qrt();
            return;
        }
        let code = if self.cfg.host.chat && self.buffered() == 0 { IDLE } else { BREAK };
        self.level = rec;
        if code == IDLE {
            self.link.idle_ticks = 0;
        }
        self.say(
            code,
            match code {
                BREAK if fq => State::FqWait,
                BREAK => State::AwaitGo,
                _ => State::IdleSent,
            },
        );
    }
}
