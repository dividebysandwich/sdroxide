use super::scheduler::{Kind, TxFront};
use super::{Arq, Common, Link, MIN_GAP, Modem, Options, Report, secs};
use xara_arq::hf::broadcast::{self, Broadcast, Plan};
use xara_arq::hf::frames::ReplyFormat;
use xara_arq::hf::session::{
    Config, CtrlIn, DataIn, FrameIn, Keys, Listen, Output, Session, State, Tx,
};
use xara_arq::host::HostEvent;
use xara_arq::time::Time;
use xara_arq::{cq, kiss};
use xara_host::command::Bw;
use xara_phy::busy::Busy;
use xara_phy::cwid;
use xara_phy::dsp::quantise;
use xara_phy::hf::ctrl::code::{BREAK, CQ, KISS};
use xara_phy::hf::ctrl::{self, Codebook, Fmt, Symbol, frame_audio, leadin_grids};
use xara_phy::hf::frame::term_marker;
use xara_phy::hf::level::{Bandwidth, Record};
use xara_phy::hf::rx::ctrl::{CtrlEntry, rx_entries};
use xara_phy::hf::rx::receiver::{Event, Receiver, Transmission};
use xara_phy::hf::tx::{CWID_BACKOFF_DB, Modulator, tune_symbol};
use xara_phy::rnd::VbRnd;

const FRAGMENT_GAP: usize = 1920;
const KISS_SALT: u64 = 0x6B69_7373;

struct Unproto {
    bc: Broadcast,
    rx: Receiver,
    entries: Vec<CtrlEntry>,
    codes: Vec<u32>,
    cb: Codebook,
    modm: Modulator,
    busy: bool,
    marker: u8,
    reply: ReplyFormat,
    reply_fmt: Fmt,
}

impl Unproto {
    fn new(seed: u64) -> Self {
        let cb = kiss::unproto_codebook();
        let entries = kiss::unproto_entries(&cb);
        let mut rx = Receiver::idle(kiss::BW);
        rx.confirm_end = false;
        Self {
            bc: Broadcast::new(seed ^ KISS_SALT),
            rx,
            entries,
            codes: Vec::new(),
            cb,
            modm: Modulator::new(kiss::BW),
            busy: false,
            marker: 0,
            reply: ReplyFormat::default(),
            reply_fmt: Fmt::F0,
        }
    }
}

pub struct Station {
    c: Common<Session>,
    rx: Receiver,
    rx_base: Time,
    modm: Modulator,
    grids: [Vec<Symbol>; 3],
    keys: Option<Keys>,
    entries: Vec<CtrlEntry>,
    cb: Option<Codebook>,
    tx_rnd: VbRnd,
    un: Unproto,
}

impl Station {
    pub fn new(cfg: Config, opts: Options) -> Self {
        xara_phy::hf::warm(cfg.bw);
        let bw = cfg.bw;
        let mut s = Self {
            un: Unproto::new(cfg.seed),
            c: Common::new(Session::new(cfg.clone()), cfg, opts, Busy::new(bw)),
            rx: Receiver::idle(bw),
            rx_base: 0,
            modm: Modulator::new(bw),
            grids: leadin_grids(),
            keys: None,
            entries: Vec::new(),
            cb: None,
            tx_rnd: VbRnd::new(),
        };
        s.drain();
        s
    }

    pub fn state(&self) -> State {
        self.c.session.state()
    }

    pub fn session(&self) -> &Session {
        &self.c.session
    }

    pub fn is_free(&self) -> bool {
        self.idle() && !self.c.tx.ptt() && !self.c.tx.tuning() && self.un.bc.idle()
    }

    pub fn set_persistence(&mut self, p: f64) {
        self.un.bc.persistence = p;
    }

    pub fn set_busy(&mut self, on: bool) {
        self.un.busy = on;
    }

    fn reset_session(&mut self) {
        self.c.session = Session::new(self.c.cfg.clone());
        self.keys = None;
        self.drain();
    }

    fn poll_unproto(&mut self) {
        let listening = self.c.session.state() == State::Listen;
        let mut codes = Vec::new();
        if listening && self.un.bc.kiss_rx {
            codes.extend(KISS);
        }
        self.un.bc.cq_rx = self.c.cfg.host.chat;
        if listening && self.c.cfg.host.chat {
            codes.push(CQ);
        }
        if codes != self.un.codes {
            let e: Vec<CtrlEntry> =
                self.un.entries.iter().filter(|e| codes.contains(&e.code)).cloned().collect();
            self.un.rx.set_ctrl_entries((!e.is_empty()).then_some(&e[..]));
            self.un.codes = codes;
        }
        let ctx = broadcast::Ctx {
            idle: self.idle() && !self.c.tx.ptt() && !self.c.tx.tuning(),
            bw500: self.c.cfg.bw == Bandwidth::Hz500,
            busy: self.un.busy || self.c.busy.is_on(),
            reply: self.un.reply_fmt,
            marker: self.un.marker,
            queued: self.buffered(),
        };
        self.un.bc.poll(self.c.now, ctx);
        let now = self.c.now;
        self.drain_unproto(now);
    }

    fn drain_unproto(&mut self, now: Time) {
        while let Some(e) = self.un.bc.pop() {
            match e {
                broadcast::Event::Pending => {
                    self.c.reports.push_back(Report::Host(HostEvent::Pending))
                }
                broadcast::Event::CancelPending => {
                    self.c.reports.push_back(Report::Host(HostEvent::CancelPending))
                }
                broadcast::Event::Send(p) => self.send_unproto(now, p),
                broadcast::Event::Expect(level) => {
                    self.un.rx.expect(level.as_slice(), Some(1));
                    if let Some(l) = level {
                        self.c.reports.push_back(unproto_bitrate(l, false));
                    }
                }
                broadcast::Event::Sn(sn) => {
                    self.c.reports.push_back(Report::Host(HostEvent::Sn(sn)))
                }
                broadcast::Event::Kiss(cmd, data) => {
                    self.c.reports.push_back(Report::Kiss(cmd, data))
                }
                broadcast::Event::Cq(c) => self.c.reports.push_back(Report::Line(c.host_line())),
            }
        }
    }

    fn send_unproto(&mut self, now: Time, p: Plan) {
        self.c.reports.push_back(Report::Trace(format!(
            "tx c{} L{}x{} unproto",
            p.code,
            p.level,
            p.frames.len()
        )));
        self.c.tx.stop_tune();
        let d = self.c.opts.drive_db;
        let g = ctrl::gain(Fmt::F0, d);
        let ann = frame_audio(&self.grids, &self.un.cb, Fmt::F0, p.code, 0)
            .iter()
            .map(|v| quantise(v * g))
            .collect();
        let earliest = self.c.tx.after_hold(now);
        self.c.tx.push(ann, 0, earliest, Vec::new(), Kind::Announce);
        let g = self.un.modm.gain(p.level, d);
        let n = p.frames.len();
        for (k, f) in p.frames.iter().enumerate() {
            let rnd = &mut self.tx_rnd;
            let x = self.un.modm.transmission(p.level, &[f], &mut |m| rnd.draw(m));
            let samples = vec![0.0; FRAGMENT_GAP]
                .into_iter()
                .chain(x.iter().map(|v| quantise(v * g)))
                .collect();
            let keyed = vec![unproto_bitrate(p.level, true)];
            self.c.tx.push(samples, 0, now, keyed, Kind::Fragment { last: k + 1 == n });
        }
    }

    fn on_unproto_event(&mut self, ev: Event) {
        match ev {
            Event::Ctrl(c) => {
                let d = &c.det;
                if self.c.session.state() == State::Listen && self.c.tx.is_empty() {
                    let t_end = c.t_end48();
                    let took = self.un.bc.on_ctrl(d.code, t_end);
                    self.c.reports.push_back(Report::Trace(format!(
                        "rx c{} f{} hits {} unproto{}",
                        d.code,
                        d.fmt,
                        d.hits,
                        if took { "" } else { " ignored" }
                    )));
                    self.c.tx.hold(t_end + MIN_GAP);
                }
            }
            Event::Tx(t) => {
                for f in t.frames.iter().filter(|f| f.frame.ok && !f.missing) {
                    let sn = f.frame.snr_avg_db.unwrap_or(f.frame.snr_db);
                    self.un.bc.on_frame(t.record, &f.frame.bytes, sn, f.t_end48());
                }
                self.c.tx.hold(t.t_end48() + MIN_GAP);
            }
            Event::Frame(_) => {}
        }
        let now = self.c.now;
        self.drain_unproto(now);
    }

    fn on_event(&mut self, ev: Event) {
        let now = self.c.now;
        let Some(t_end) = rx_end(&ev).map(|t| self.rx_base + t) else {
            self.drain();
            return;
        };
        self.c.tx.hold(t_end + MIN_GAP);
        match ev {
            Event::Ctrl(c) => {
                let d = c.det;
                self.un.reply_fmt = self.un.reply.update(d.snr_norm, d.hits, d.fmt).1;
                if !self.un.bc.idle() {
                    return;
                }
                self.c.reports.push_back(Report::Trace(format!(
                    "rx c{} f{} hits {} sn {:.1} end {:.3} {:?}",
                    d.code,
                    d.fmt,
                    d.hits,
                    d.snr_norm,
                    secs(t_end),
                    self.c.session.state()
                )));
                self.c.session.on_ctrl(
                    now,
                    CtrlIn { code: d.code, fmt: d.fmt, hits: d.hits, snr_norm: d.snr_norm, t_end },
                );
            }
            Event::Tx(t) => {
                if !self.un.bc.idle() {
                    return;
                }
                let was = self.c.session.state();
                let decoded = t.ok();
                if was == State::Receiving && decoded.len() == 2 && decoded[0] {
                    self.report_bitrate(t.record, false);
                }
                let ok: String = t
                    .frames
                    .iter()
                    .map(|f| match f.frame.bytes.len() {
                        n if f.frame.ok && n >= 3 && f.frame.bytes[n - 3] & 0x20 != 0 => 'z',
                        _ if f.frame.ok => '+',
                        _ => '-',
                    })
                    .collect();
                self.c.reports.push_back(Report::Trace(format!(
                    "rx L{} [{ok}] end {:.3} {:?}",
                    t.record,
                    secs(t_end),
                    self.c.session.state()
                )));
                self.c.session.on_data(now, data_in(&t, t_end));
                let mark = self.c.reports.len();
                self.drain();
                if was == State::Answering
                    && let Some(i) = (mark..self.c.reports.len()).find(|&i| {
                        matches!(self.c.reports[i], Report::Host(HostEvent::Connected { .. }))
                    })
                {
                    self.c.reports.insert(i, Report::UnencryptedLink);
                }
                if was == State::Receiving
                    && decoded.last() == Some(&true)
                    && self.c.session.state() != State::Qrt
                {
                    self.report_bitrate(t.record, false);
                }
            }
            Event::Frame(_) => {}
        }
        self.drain();
    }

    fn render(&mut self, t: &Tx) -> (Vec<f64>, Vec<Report>) {
        let d = self.c.opts.drive_db;
        let mut keyed = Vec::new();
        let (mut x, g) = match t {
            Tx::Ctrl { code, fmt, extra } => {
                if *code == BREAK {
                    let r = self.tx_rnd.rand_int(2);
                    self.c.session.set_break_draw(r);
                }
                let cb = self.cb.as_ref().expect("codebook before first control frame");
                (frame_audio(&self.grids, cb, *fmt, *code, *extra), ctrl::gain(*fmt, d))
            }
            Tx::Data { level, frames } => {
                self.report_bitrate(*level, true);
                keyed.extend((1..frames.len()).map(|_| self.bitrate(*level, true)));
                let refs: Vec<&[u8]> = frames.iter().map(Vec::as_slice).collect();
                let rnd = &mut self.tx_rnd;
                (
                    self.modm.transmission(*level, &refs, &mut |m| rnd.draw(m)),
                    self.modm.gain(*level, d),
                )
            }
            Tx::CwId { call, blanks } => {
                let mut x = if *blanks == 0 { vec![0.0; cwid::UNIT] } else { Vec::new() };
                x.extend(cwid::synth(call, *blanks, false));
                (x, 10f64.powf((d - CWID_BACKOFF_DB) / 20.0))
            }
        };
        x.iter_mut().for_each(|v| *v = quantise(*v * g));
        (x, keyed)
    }

    fn bitrate(&self, level: Record, tx: bool) -> Report {
        let bps = self.c.session.bandwidth().level(level).net_bps().round() as u32;
        Report::Bitrate { level: level.get(), bps, tx }
    }

    fn report_bitrate(&mut self, level: Record, tx: bool) {
        let r = self.bitrate(level, tx);
        self.c.reports.push_back(r);
    }

    pub fn bandwidth(&self) -> Bandwidth {
        self.rx.bw
    }
}

impl TxFront for Station {
    fn burst_done(&mut self, kind: Kind, start: Time, end: Time) {
        let heard = end + self.c.tx.latency();
        self.rx.mute(start - self.rx_base, heard - self.rx_base);
        self.rx.mark_tx_end(heard - self.rx_base);
        self.un.rx.mute(start, heard);
        let mark = self.c.reports.len();
        if kind == Kind::Session {
            self.c.session.on_tx_done(heard + self.c.opts.ptt_tail);
            self.drain_at(end);
        }
        self.c.tx.release(end, &mut self.c.reports, mark);
        if kind == (Kind::Fragment { last: true }) {
            self.un.bc.tx_done();
            self.drain_unproto(end);
        }
    }
}

impl Modem for Station {
    type Config = Config;
    type Session = Session;
    const VERSION: &'static str = "VERSION VARA HF v4.9.0";
    const VIAS: bool = false;
    const KISS_RX_FOLLOWS_CLIENTS: bool = true;
    const CWID_GATEWAY: bool = true;

    fn common(&self) -> &Common<Session> {
        &self.c
    }

    fn common_mut(&mut self) -> &mut Common<Session> {
        &mut self.c
    }

    fn apply(&mut self, l: Listen) {
        if self.c.listen.as_ref().is_none_or(|o| o.levels != l.levels || o.nframes != l.nframes) {
            self.c
                .reports
                .push_back(Report::Trace(format!("expect {:?} x{}", l.levels, l.nframes)));
        }
        if self.keys.as_ref() != Some(&l.keys) {
            let k = &l.keys;
            let mut cb = Codebook::new(k.seed, k.mycalls, k.role, l.bw);
            if let Some((h, r)) = k.connect {
                self.tx_rnd = cb.set_connect_codes_tx_rnd(h, r);
            }
            self.entries = rx_entries(&cb, 0);
            self.cb = Some(cb);
            self.keys = Some(l.keys.clone());
        }
        let answering = self.c.listen_on || self.c.session.state() != State::Listen;
        let entries: Vec<CtrlEntry> = self
            .entries
            .iter()
            .filter(|e| answering && l.codes.contains(&e.code))
            .cloned()
            .collect();
        self.rx.set_ctrl_entries((!entries.is_empty()).then_some(&entries[..]));
        self.rx.expect(&l.levels, Some(l.nframes));
        self.rx.linked = l.linked;
        self.c.listen = Some(l);
    }

    fn drain_at(&mut self, now: Time) {
        if !self.c.pending.is_empty() && self.c.session.state() != State::Listen {
            let d = std::mem::take(&mut self.c.pending);
            self.c.session.send(now, &d);
        }
        let mut txs = Vec::new();
        while let Some(o) = self.c.session.pop() {
            match o {
                Output::Tx(t) => txs.push(t),
                Output::Listen(l) => self.apply(l),
                Output::Host(h) => {
                    if let HostEvent::Connected { caller, .. } = &h {
                        self.un.marker = term_marker(caller.as_bytes());
                    }
                    self.c.reports.push_back(Report::Host(h));
                }
            }
        }
        for t in txs {
            let what = match &t {
                Tx::Ctrl { code, fmt, extra } => {
                    format!("tx c{code} f{fmt}{}", if *extra > 0 { "+" } else { "" })
                }
                Tx::Data { level, frames } => format!("tx L{level}x{}", frames.len()),
                Tx::CwId { .. } => "tx cw".into(),
            };
            self.c.reports.push_back(Report::Trace(format!("{what} {:?}", self.c.session.state())));
            self.c.tx.stop_tune();
            let earliest =
                if matches!(t, Tx::CwId { .. }) { now } else { self.c.tx.next_start(now) };
            if matches!(t, Tx::Data { .. }) && self.c.session.state() == State::HeaderSent {
                self.c.reports.push_back(Report::UnencryptedLink);
            }
            let (samples, keyed) = self.render(&t);
            self.c.tx.push(samples, 0, earliest, keyed, Kind::Session);
        }
    }

    fn tune_tone(drive_db: f64) -> Vec<f64> {
        tune_symbol(drive_db).iter().map(|&v| quantise(v)).collect()
    }

    fn idle(&self) -> bool {
        self.c.session.state() == State::Listen && self.c.tx.is_empty()
    }

    fn link(&self) -> Link {
        let s = &self.c.session;
        Link {
            state: format!("{:?}", s.state()),
            level: s.level().get(),
            peer: s.peer().to_string(),
            bw: s.bandwidth().hz().to_string(),
            passband: s.bandwidth().passband(),
            attempts: s.attempts(),
            afc_hz: Some(self.rx.afc_hz),
        }
    }

    fn reconfigure(&mut self, cfg: Config) -> bool {
        if !self.idle() {
            return false;
        }
        if cfg.bw != self.rx.bw {
            self.rx = Receiver::idle(cfg.bw);
            self.rx_base = self.c.now;
            self.modm = Modulator::new(cfg.bw);
        }
        self.c.cfg = cfg;
        self.reset_session();
        true
    }

    fn process(&mut self, input: &[f64], output: &mut [f64]) {
        let (t0, n) = (self.c.now, input.len());
        let fed = self.key_block(t0, input, output);
        self.c.now = t0 + n as Time;
        self.c.busy.set_bw(self.c.session.bandwidth());
        let forced =
            self.c.tx.ptt() || !self.c.tx.is_empty() || self.c.session.state() != State::Listen;
        for e in self.c.busy.feed(input, forced) {
            self.c.reports.push_back(Report::Busy(e));
        }
        for ev in self.rx.feed(&fed) {
            self.on_event(ev);
        }
        for ev in self.un.rx.feed(&fed) {
            self.on_unproto_event(ev);
        }
        if self.c.session.next_deadline().is_some_and(|d| d <= self.c.now) {
            self.c.session.poll(self.c.now);
        }
        self.drain();
        self.poll_unproto();
    }

    fn connect(&mut self, mycall: &str, remote: &str, vias: &[String]) -> bool {
        if !vias.is_empty() {
            return false;
        }
        if self.c.session.state() == State::Answering && !self.c.tx.ptt() && self.c.tx.is_empty() {
            self.reset_session();
        }
        if self.c.session.state() != State::Listen || !self.un.bc.idle() {
            return false;
        }
        self.c.session.connect(self.c.now, Some(mycall), remote);
        self.drain();
        true
    }

    fn kiss(&mut self, cmd: kiss::Command, data: Vec<u8>) -> bool {
        let ok = self.un.bc.kiss(cmd, data);
        self.poll_unproto();
        ok
    }

    fn cq_frame(&mut self, call: &str, bw: Option<&str>) -> bool {
        let Some(bw) = bw else { return false };
        let ok =
            self.is_free() && self.un.bc.cq_frame(call, bw.parse().unwrap_or(Bandwidth::Hz2750));
        self.poll_unproto();
        ok
    }

    fn set_ignore_dcd(&mut self, on: bool) {
        self.un.bc.ignore_dcd = on;
    }

    fn set_bw(&mut self, bw: Bw) -> bool {
        let mut cfg = self.c.cfg.clone();
        cfg.bw = match bw {
            Bw::Hz500 => Bandwidth::Hz500,
            Bw::Hz2300 => Bandwidth::Hz2300,
            Bw::Hz2750 => Bandwidth::Hz2750,
        };
        self.reconfigure(cfg)
    }

    fn set_p2p(&mut self, on: bool) {
        self.update(|c| c.p2p = on);
        self.c.session.reset_timing();
    }

    fn set_kiss_rx(&mut self, on: bool) {
        self.un.bc.kiss_rx = on;
    }
}

impl Arq for Session {
    type Config = Config;
    type Listen = Listen;

    fn listening(&self) -> bool {
        self.state() == State::Listen
    }

    fn buffered(&self) -> usize {
        Session::buffered(self)
    }

    fn config_mut(&mut self) -> &mut Config {
        Session::config_mut(self)
    }

    fn send(&mut self, now: Time, data: &[u8]) {
        Session::send(self, now, data)
    }

    fn disconnect(&mut self, now: Time) {
        Session::disconnect(self, now)
    }

    fn abort(&mut self, now: Time) {
        Session::abort(self, now)
    }

    fn drop_link(&mut self) {
        Session::drop_link(self)
    }
}

fn unproto_bitrate(level: Record, tx: bool) -> Report {
    let level = if level == cq::RECORD { Record::ONE } else { level };
    Report::Bitrate { level: level.get(), bps: kiss::BW.level(level).net_bps().round() as u32, tx }
}

fn rx_end(ev: &Event) -> Option<Time> {
    match ev {
        Event::Ctrl(c) => Some(c.t_end48()),
        Event::Tx(t) => Some(t.t_end48()),
        Event::Frame(_) => None,
    }
}

fn data_in(t: &Transmission, t_end: Time) -> DataIn {
    let frames = t
        .frames
        .iter()
        .map(|f| FrameIn {
            ok: f.frame.ok && !f.missing,
            bytes: if f.frame.ok { f.frame.bytes.clone() } else { Vec::new() },
            snr_db: f.frame.snr_avg_db.unwrap_or(f.frame.snr_db),
        })
        .collect();
    DataIn { level: t.record, frames, t_end }
}
