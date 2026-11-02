pub mod hf;
pub mod scheduler;

use scheduler::TxScheduler;
use std::collections::VecDeque;
use xara_arq::host::{HostEvent, HostSettings};
use xara_arq::kiss::Command;
use xara_arq::time::{SECOND, Time};
use xara_host::command::Bw;
use xara_phy::busy::Busy;

pub const DEFAULT_DRIVE_DB: f64 = -5.0;
pub const TX_RING_BYTES: usize = 600_000;
pub(crate) const MIN_GAP: Time = 4000;

pub(crate) fn secs(t: Time) -> f64 {
    t as f64 / SECOND as f64
}

#[derive(Clone, Debug)]
pub struct Options {
    pub drive_db: f64,
    pub tx_latency: Time,
    pub rx_lag: Time,
    pub ptt_tail: Time,
}

impl Default for Options {
    fn default() -> Self {
        Self { drive_db: DEFAULT_DRIVE_DB, tx_latency: 0, rx_lag: 0, ptt_tail: 0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Report {
    Host(HostEvent),
    Ptt(bool),
    Bitrate { level: usize, bps: u32, tx: bool },
    Busy(bool),
    UnencryptedLink,
    Trace(String),
    Line(String),
    Kiss(Command, Vec<u8>),
}

pub trait HostConfig: Clone {
    fn host(&self) -> &HostSettings;
    fn host_mut(&mut self) -> &mut HostSettings;
}

impl HostConfig for xara_arq::hf::Config {
    fn host(&self) -> &HostSettings {
        &self.host
    }
    fn host_mut(&mut self) -> &mut HostSettings {
        &mut self.host
    }
}


pub trait Arq {
    type Config: HostConfig;
    type Listen: Clone;

    fn listening(&self) -> bool;
    fn buffered(&self) -> usize;
    fn config_mut(&mut self) -> &mut Self::Config;
    fn send(&mut self, now: Time, data: &[u8]);
    fn disconnect(&mut self, now: Time);
    fn abort(&mut self, now: Time);
    fn drop_link(&mut self);
}

pub struct Common<S: Arq> {
    pub(crate) session: S,
    pub(crate) cfg: S::Config,
    pub(crate) opts: Options,
    pub(crate) listen_on: bool,
    pub(crate) listen: Option<S::Listen>,
    pub(crate) now: Time,
    pub(crate) tx: TxScheduler,
    pub(crate) reports: VecDeque<Report>,
    pub(crate) pending: Vec<u8>,
    pub(crate) busy: Busy,
}

impl<S: Arq> Common<S> {
    pub(crate) fn new(session: S, cfg: S::Config, opts: Options, busy: Busy) -> Self {
        Self {
            session,
            cfg,
            tx: TxScheduler::new(opts.tx_latency, opts.rx_lag),
            opts,
            listen_on: true,
            listen: None,
            now: 0,
            reports: VecDeque::new(),
            pending: Vec::new(),
            busy,
        }
    }
}

/// What the link is doing, for the panel. Upstream's `xara_ui::telemetry::Link`,
/// kept local so the web-UI crate does not come along with it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Link {
    pub state: String,
    pub level: usize,
    pub peer: String,
    pub bw: String,
    pub passband: (f64, f64),
    pub attempts: u32,
    pub afc_hz: Option<f64>,
}

pub trait Modem {
    type Config: HostConfig;
    type Session: Arq<Config = Self::Config>;
    const VERSION: &'static str;
    const VIAS: bool;
    const KISS_RX_FOLLOWS_CLIENTS: bool;
    const CWID_GATEWAY: bool;

    fn common(&self) -> &Common<Self::Session>;
    fn common_mut(&mut self) -> &mut Common<Self::Session>;
    fn apply(&mut self, l: <Self::Session as Arq>::Listen);
    fn drain_at(&mut self, now: Time);
    fn tune_tone(drive_db: f64) -> Vec<f64>;

    fn idle(&self) -> bool;
    fn link(&self) -> Link;
    fn reconfigure(&mut self, cfg: Self::Config) -> bool;
    fn process(&mut self, input: &[f64], output: &mut [f64]);
    fn connect(&mut self, mycall: &str, remote: &str, vias: &[String]) -> bool;
    fn kiss(&mut self, cmd: Command, data: Vec<u8>) -> bool;
    fn cq_frame(&mut self, call: &str, bw: Option<&str>) -> bool;
    fn set_ignore_dcd(&mut self, on: bool);
    fn set_bw(&mut self, bw: Bw) -> bool;
    fn set_p2p(&mut self, on: bool);
    fn set_kiss_rx(&mut self, on: bool);

    fn drain(&mut self) {
        let now = self.now();
        self.drain_at(now);
    }

    fn now(&self) -> Time {
        self.common().now
    }

    fn ptt(&self) -> bool {
        self.common().tx.ptt()
    }

    fn busy(&self) -> bool {
        self.common().busy.is_on()
    }

    fn buffered(&self) -> usize {
        let c = self.common();
        c.pending.len() + c.session.buffered()
    }

    fn tune(&mut self, on: bool) -> bool {
        if !on {
            let c = self.common_mut();
            c.tx.untune(c.now, &mut c.reports);
            return true;
        }
        if !self.idle() || self.ptt() {
            return false;
        }
        let c = self.common_mut();
        c.tx.tune(Self::tune_tone(c.opts.drive_db));
        true
    }

    fn tuning(&self) -> bool {
        self.common().tx.tuning()
    }

    fn set_drive(&mut self, db: f64) {
        let c = self.common_mut();
        c.opts.drive_db = db;
        c.tx.retune(|| Self::tune_tone(db));
    }

    fn set_tx_latency(&mut self, t: Time, lag: Time) {
        let c = self.common_mut();
        (c.opts.tx_latency, c.opts.rx_lag) = (t, lag);
        c.tx.set_latency(t, lag);
    }

    fn options(&self) -> &Options {
        &self.common().opts
    }

    fn config(&self) -> &Self::Config {
        &self.common().cfg
    }

    fn update(&mut self, f: impl Fn(&mut Self::Config)) {
        let c = self.common_mut();
        f(&mut c.cfg);
        f(c.session.config_mut());
    }

    fn pop(&mut self) -> Option<Report> {
        let c = self.common_mut();
        c.tx.pop(&mut c.reports)
    }

    fn send(&mut self, data: &[u8]) {
        if self.common().session.listening() {
            let room = TX_RING_BYTES.saturating_sub(self.buffered());
            let c = self.common_mut();
            if data.len() > room {
                c.reports.push_back(Report::Trace("buffer limit exceeded".into()));
            }
            c.pending.extend_from_slice(&data[..data.len().min(room)]);
            return;
        }
        let c = self.common_mut();
        c.session.send(c.now, data);
        self.drain();
    }

    fn disconnect(&mut self) {
        let c = self.common_mut();
        c.session.disconnect(c.now);
        self.drain();
    }

    fn abort(&mut self) {
        let c = self.common_mut();
        c.session.abort(c.now);
        self.drain();
    }

    fn drop_link(&mut self) {
        self.common_mut().session.drop_link();
        self.drain();
    }

    fn set_listen(&mut self, on: bool) {
        let c = self.common_mut();
        c.listen_on = on;
        if let Some(l) = c.listen.clone() {
            self.apply(l);
        }
    }
}
