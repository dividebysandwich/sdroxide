//! Host-command handling: VARA's command language turned into station calls,
//! and the station's reports turned back into VARA's replies.
//!
//! A port of upstream xara's `crates/xara/src/tnc.rs` (MIT). What is gone is
//! what belonged to xara the program rather than to the protocol: the web UI's
//! telemetry and controls, the restart request, the KISS port, and stdout
//! logging. What is new is [`Observed`]: every reply also goes to the operator's
//! panel, whoever the host is, so the panel shows a Pat session as well as its
//! own.

use crate::host::{HostMux, Input};
use crate::station::{HostConfig, Modem, Report, TX_RING_BYTES};
use xara_arq::compression::Compression;
use xara_arq::host::{HostEvent, LinkBw};
use xara_arq::time::{SECOND, Time};
use xara_host::command::{Command, Comp, DRIVE_MIN_DB, Drive, Invalid};
use xara_host::response::{LinkWidth, Response};

const REGISTERED_AFTER: Time = 5 * SECOND;
pub const HOST_TUNE_LIMIT: Time = 60 * SECOND;
const ALIVE_EVERY: Time = 60 * SECOND;
const HOST_INPUTS_PER_CALL: usize = 32;

/// What the panel hears about, beside the replies the host gets.
#[derive(Clone, Debug, PartialEq)]
pub enum Observed {
    /// A reply or an asynchronous line, as the host saw it.
    Reply(Response),
    /// Payload the far station sent.
    Data(Vec<u8>),
    /// Payload the host handed us to send.
    Sent(Vec<u8>),
    /// A command the host issued, as text.
    Command(String),
    /// Station-internal trace (frame-level), for the log.
    Trace(String),
    UnableToConnect,
    TimedOut,
}

pub struct Tnc<M: Modem> {
    station: M,
    host: HostMux,
    keyed: bool,
    alive_at: Time,
    registered_at: Option<Time>,
    deferred_mycall: Option<Result<Vec<String>, Invalid>>,
    saved_retries: u32,
    busy: bool,
    tune_until: Option<Time>,
    observed: Vec<Observed>,
    trace: bool,
}

impl<M: Modem> Tnc<M> {
    pub fn new(station: M, host: HostMux) -> Self {
        Self {
            alive_at: station.now() + ALIVE_EVERY,
            station,
            host,
            keyed: false,
            registered_at: None,
            deferred_mycall: None,
            saved_retries: 0,
            busy: false,
            tune_until: None,
            observed: Vec::new(),
            trace: false,
        }
    }

    pub fn station(&self) -> &M {
        &self.station
    }

    pub fn station_mut(&mut self) -> &mut M {
        &mut self.station
    }

    pub fn host(&self) -> &HostMux {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut HostMux {
        &mut self.host
    }

    /// Whether the station wants the transmitter keyed.
    pub fn keyed(&self) -> bool {
        self.keyed
    }

    pub fn set_trace(&mut self, on: bool) {
        self.trace = on;
    }

    pub fn take_observed(&mut self) -> Vec<Observed> {
        std::mem::take(&mut self.observed)
    }

    pub fn set_tx_latency(&mut self, t: Time, lag: Time) {
        self.station.set_tx_latency(t, lag);
    }

    /// Run the station for one block: `input` is what the receiver heard,
    /// `output` what goes to the transmitter (zeros while unkeyed).
    pub fn process(&mut self, input: &[f64], output: &mut [f64]) {
        self.service();
        self.station.process(input, output);
        let now = self.station.now();
        if now >= self.alive_at {
            self.alive_at += ALIVE_EVERY;
            self.say(Response::IAmAlive);
        }
        if self.registered_at.is_some_and(|t| now >= t) {
            self.registered_at = None;
            self.say(registered(self.station.config()));
            self.say(Response::EncryptionDisabled);
        }
        self.flush();
        if self.station.busy() != self.busy {
            self.report_busy(self.station.busy());
        }
        if self.tune_until.is_some_and(|t| now >= t) || (self.tune_until.is_some() && !self.station.tuning()) {
            self.stop_tune();
        }
    }

    /// Operator's abort: tear the link down now, whoever the host is.
    pub fn abort(&mut self) {
        self.stop_tune();
        self.station.abort();
        self.flush();
    }

    /// Operator's tune: a steady tone for an antenna tuner, capped like a
    /// host's.
    pub fn tune(&mut self, on: bool) -> bool {
        if on { self.start_tune() } else { self.stop_tune() }
    }

    pub fn tuning(&self) -> bool {
        self.station.tuning()
    }

    pub fn set_drive(&mut self, db: f64) -> f64 {
        let db = db.clamp(DRIVE_MIN_DB, 0.0).round_ties_even();
        self.station.set_drive(db);
        db
    }

    fn service(&mut self) {
        self.hold_data();
        for _ in 0..HOST_INPUTS_PER_CALL {
            let Some(i) = self.host.try_recv() else { break };
            match i {
                Input::Command(c) => {
                    self.observed.push(Observed::Command(format!("{c:?}")));
                    self.command(c);
                }
                Input::Wrong(l) => {
                    self.observed.push(Observed::Command(l));
                    self.say(Response::Wrong);
                }
                Input::Data(d) => {
                    self.observed.push(Observed::Sent(d.clone()));
                    self.station.send(&d);
                    self.hold_data();
                }
                Input::Connected => self.toggle_busy(),
                Input::Closed => self.host_dropped(),
            }
        }
        self.flush();
    }

    fn start_tune(&mut self) -> bool {
        let ok = self.station.tune(true);
        if ok {
            self.tune_until = Some(self.station.now() + HOST_TUNE_LIMIT);
        }
        ok
    }

    fn stop_tune(&mut self) -> bool {
        self.tune_until = None;
        self.station.tune(false)
    }

    fn host_tune(&mut self, drive: Drive) {
        match drive {
            Drive::OutOfRange => return,
            Drive::Db(db) => {
                self.set_drive(f64::from(db));
            }
            Drive::NotNumeric => {}
        }
        if !self.station.tuning() {
            self.start_tune();
        }
    }

    fn say(&mut self, r: Response) {
        self.host.line(&r.to_string());
        self.observed.push(Observed::Reply(r));
    }

    fn report_busy(&mut self, on: bool) {
        self.busy = on;
        self.say(Response::Busy(on));
    }

    fn toggle_busy(&mut self) {
        self.report_busy(!self.busy);
    }

    fn hold_data(&self) {
        self.host.hold_data(!self.idle() && self.station.buffered() >= TX_RING_BYTES);
    }

    fn idle(&self) -> bool {
        self.station.idle()
    }

    fn reconfigure(&mut self, f: impl Fn(&mut M::Config)) -> bool {
        let mut cfg = self.station.config().clone();
        f(&mut cfg);
        self.station.reconfigure(cfg)
    }

    /// Apply a configuration change from the operator's settings. Refused (and
    /// worth retrying) while a link is up.
    pub fn reconfigure_with(&mut self, f: impl Fn(&mut M::Config)) -> bool {
        self.reconfigure(f)
    }

    fn set_mycalls(&mut self, calls: Vec<String>) {
        if self.reconfigure(|c| c.host_mut().mycalls = calls.clone()) {
            self.registered_at = Some(self.station.now() + REGISTERED_AFTER);
        }
    }

    fn set_chat(&mut self, on: bool) {
        if !self.reconfigure(|c| c.host_mut().chat = on) {
            self.station.update(|c| c.host_mut().chat = on);
        }
    }

    fn host_dropped(&mut self) {
        if self.tune_until.is_some() {
            self.stop_tune();
        }
        self.station.set_listen(false);
        self.station.drop_link();
        self.set_chat(false);
    }

    fn command(&mut self, c: Command) {
        let idle = self.idle();
        let ok = match c {
            Command::Connect { mycall, remote } => !self.station.ptt() && self.station.connect(&mycall, &remote, &[]),
            Command::ConnectVia { mycall, remote, vias } => {
                M::VIAS
                    && !self.station.ptt()
                    && !vias.contains(&mycall)
                    && self.station.connect(&mycall, &remote, &vias)
            }
            Command::Mycall(calls) if !idle => {
                self.deferred_mycall = Some(calls);
                false
            }
            Command::Mycall(Ok(calls)) => {
                self.deferred_mycall = None;
                self.set_mycalls(calls);
                true
            }
            Command::Mycall(Err(Invalid)) => {
                self.say(Response::Wrong);
                true
            }
            Command::CqFrame { call, bw } => idle && self.station.cq_frame(&call, bw.as_deref()),
            Command::Disconnect => {
                self.station.disconnect();
                true
            }
            Command::Abort => {
                self.station.abort();
                true
            }
            Command::Chat(on) if idle => {
                if on {
                    self.toggle_busy();
                }
                self.set_chat(on && self.host.has_cmd_client() && self.host.has_data_client());
                if on {
                    self.station.set_listen(true);
                }
                true
            }
            Command::Chat(_) => false,
            Command::WinlinkSession | Command::P2pSession => {
                self.station.set_p2p(c == Command::P2pSession);
                true
            }
            Command::Retries(v) => {
                let cur = self.station.config().host().retries;
                let new = match v {
                    Ok(0) if self.saved_retries > 0 => Some(self.saved_retries),
                    Ok(0) | Err(Invalid) => None,
                    Ok(n) => {
                        self.saved_retries = cur;
                        Some(u32::from(n))
                    }
                };
                if let Some(n) = new {
                    self.station.update(|c| c.host_mut().retries = n);
                }
                true
            }
            Command::Listen(on) => {
                let linked = !idle;
                self.station.set_listen(on);
                self.say(Response::Ok);
                if linked {
                    self.station.drop_link();
                }
                return;
            }
            Command::Compression(m) => {
                let m = match m {
                    Comp::Off => Compression::Off,
                    Comp::Text => Compression::Text,
                    Comp::Files => Compression::Files,
                };
                self.station.update(|c| c.host_mut().compression = m);
                true
            }
            Command::Bw(b) => idle && self.station.set_bw(b),
            Command::Gateway | Command::Cwid(_) => M::CWID_GATEWAY,
            Command::Public(_) => true,
            Command::TuneOff => {
                self.stop_tune();
                true
            }
            Command::Tune(d) => {
                self.say(Response::Ok);
                self.host_tune(d);
                return;
            }
            Command::DriveLevel(d) => {
                if let Drive::Db(db) = d {
                    self.set_drive(f64::from(db));
                }
                true
            }
            Command::IgnoreKissDcd(on) => {
                self.station.set_ignore_dcd(on);
                true
            }
            Command::TuneQuery => {
                self.say(Response::Tune(self.station.options().drive_db.round()));
                return;
            }
            Command::CleanTxBuffer => {
                self.say(Response::CleanTxBuffer { empty: self.station.buffered() == 0 });
                return;
            }
            Command::Version => {
                self.say(Response::Version(M::VERSION));
                return;
            }
        };
        self.say(if ok { Response::Ok } else { Response::Wrong });
    }

    fn flush(&mut self) {
        while let Some(r) = self.station.pop() {
            match r {
                Report::Trace(t) => {
                    if self.trace {
                        self.observed.push(Observed::Trace(t));
                    }
                }
                Report::Ptt(on) => {
                    self.keyed = on;
                    self.say(Response::Ptt(on));
                }
                Report::Busy(on) => {
                    if on != self.busy {
                        self.report_busy(on);
                    }
                }
                Report::UnencryptedLink => self.say(Response::UnencryptedLink),
                Report::Line(l) => self.say(Response::Line(l)),
                // No KISS port is served; unproto frames heard are dropped.
                Report::Kiss(..) => {}
                Report::Bitrate { level, bps, tx } => self.say(Response::Bitrate { level, bps, tx }),
                Report::Host(HostEvent::Data(d)) => {
                    self.host.data(&d);
                    self.observed.push(Observed::Data(d));
                }
                Report::Host(HostEvent::Disconnected) => {
                    let deferred = self.deferred_mycall.take();
                    if let Some(Err(Invalid)) = deferred {
                        self.say(Response::Wrong);
                    }
                    self.say(Response::Disconnected);
                    if let Some(Ok(calls)) = deferred {
                        self.set_mycalls(calls);
                    }
                }
                Report::Host(HostEvent::UnableToConnect) => self.observed.push(Observed::UnableToConnect),
                Report::Host(HostEvent::TimedOut) => self.observed.push(Observed::TimedOut),
                Report::Host(h) => {
                    if let Some(r) = response(h) {
                        self.say(r);
                    }
                }
            }
        }
    }
}

fn registered(cfg: &impl HostConfig) -> Response {
    Response::Registered(cfg.host().registered.then(|| cfg.host().mycalls.clone()))
}

pub fn response(h: HostEvent) -> Option<Response> {
    Some(match h {
        HostEvent::Pending => Response::Pending,
        HostEvent::CancelPending => Response::CancelPending,
        HostEvent::Connected { caller, answerer, vias, bw } => {
            let width = match bw {
                LinkBw::Hf(b) => LinkWidth::Hz(b.hz()),
                LinkBw::Fm(xara_phy::fm::level::FmBandwidth::Narrow) => LinkWidth::Narrow,
                LinkBw::Fm(xara_phy::fm::level::FmBandwidth::Wide) => LinkWidth::Wide,
            };
            Response::Connected { caller, answerer, vias, width }
        }
        HostEvent::Disconnected => Response::Disconnected,
        HostEvent::Registered(r) => Response::LinkRegistered(r),
        HostEvent::Buffer(n) => Response::Buffer(n),
        HostEvent::Sn(s) => Response::Sn(s),
        HostEvent::UnableToConnect
        | HostEvent::TimedOut
        | HostEvent::Data(_)
        | HostEvent::Unencrypted
        | HostEvent::RxFrame(_)
        | HostEvent::Log(_) => return None,
    })
}
