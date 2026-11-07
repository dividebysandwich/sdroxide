use crate::fm::adapt::SNR_OFFSET_DB;
use crate::fm::session::NO_LEAD_HEARD;
use crate::fm::timing::watchdog;
use crate::host::{HostEvent, QRT_RECEIVED};
use crate::time::Time;
use std::collections::VecDeque;
use xara_phy::call::call_hash;
use xara_phy::fm::ctrl::code::*;
use xara_phy::fm::ctrl::{LeadIn, Side};
use xara_phy::fm::frame::{Kind, RECORD0_LEN, Record0, Trailer, snr_byte};
use xara_phy::fm::level::{DataLevel, FmBandwidth, Record};
use xara_phy::fm::tx::CONNECT_LEAD_IN;

const POLL_CAP: u32 = 99;

#[derive(Clone, Debug, PartialEq)]
pub enum DigiTx {
    Ctrl { code: u32, lead: usize, side: Side },
    Data { record: Record, frame: Vec<u8>, lead: usize, side: Side },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtrlListen {
    pub key_d: u32,
    pub key_u: u32,
    pub codes: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DigiListen {
    pub bw: FmBandwidth,
    pub data_key: u32,
    pub records: Vec<Record>,
    pub ctrl: [CtrlListen; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub enum DigiOutput {
    Tx(DigiTx),
    Listen(DigiListen),
    Host(HostEvent),
    Done,
}

pub struct Digi {
    call: String,
    own_bw: FmBandwidth,
    active: bool,
    path: String,
    keys: [(u32, u32); 2],
    lead: [LeadIn; 2],
    heard: [u8; 2],
    data_side: Side,
    link_up: bool,
    linked: bool,
    bw: FmBandwidth,
    mask: Vec<Record>,
    last_level: DataLevel,
    expected: DataLevel,
    nack_run: usize,
    offer: Option<usize>,
    polls: u32,
    interval: Time,
    deadline: Option<Time>,
    quitting: bool,
    out: VecDeque<DigiOutput>,
}

fn snr_in(snr_db: f64, chain: u8) -> u8 {
    let s = snr_db + SNR_OFFSET_DB;
    snr_byte(if chain != 0 { s.min(f64::from(chain) / 7.0) } else { s })
}

impl Digi {
    pub fn new(call: &str, bw: FmBandwidth) -> Self {
        Self {
            call: call.to_ascii_uppercase(),
            own_bw: bw,
            active: false,
            path: String::new(),
            keys: [(0, 0); 2],
            lead: [LeadIn::default(), LeadIn::default()],
            heard: [NO_LEAD_HEARD; 2],
            data_side: Side::Target,
            link_up: false,
            linked: false,
            bw,
            mask: Vec::new(),
            last_level: DataLevel::ONE,
            expected: DataLevel::ONE,
            nack_run: 0,
            offer: None,
            polls: 0,
            interval: watchdog(1),
            deadline: None,
            quitting: false,
            out: VecDeque::new(),
        }
    }

    pub fn call(&self) -> &str {
        &self.call
    }

    pub fn set_call(&mut self, call: &str) {
        self.call = call.to_ascii_uppercase();
    }

    pub fn set_bandwidth(&mut self, bw: FmBandwidth) {
        self.own_bw = bw;
        if !self.active {
            self.bw = bw;
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn bandwidth(&self) -> FmBandwidth {
        self.bw
    }

    pub fn data_key(&self) -> u32 {
        self.key(self.data_side)
    }

    pub fn key(&self, side: Side) -> u32 {
        self.keys[side.idx()].0
    }

    pub fn next_deadline(&self) -> Option<Time> {
        self.deadline
    }

    pub fn pop(&mut self) -> Option<DigiOutput> {
        self.out.pop_front()
    }

    fn log(&mut self, s: String) {
        self.out.push_back(DigiOutput::Host(HostEvent::Log(s)));
    }

    fn restart(&mut self, now: Time) {
        self.deadline = Some(now + self.interval);
    }

    pub fn on_record0(&mut self, now: Time, bytes: &[u8], snr_db: f64) -> bool {
        if self.call.is_empty() || bytes.len() != RECORD0_LEN {
            return false;
        }
        let Some(r) = Record0::parse(bytes) else { return false };
        if r.kind != Kind::Connect {
            return false;
        }
        let eq = |a: &str, b: &str| !a.is_empty() && a.eq_ignore_ascii_case(b);
        if eq(&r.via1, &r.source)
            || eq(&r.via2, &r.source)
            || eq(&r.via1, &r.target)
            || eq(&r.via2, &r.target)
            || eq(&r.via1, &r.via2)
        {
            return false;
        }
        let hop = if eq(&r.via1, &self.call) && r.hop == 0 {
            1
        } else if eq(&r.via2, &self.call) && r.hop == 1 {
            2
        } else {
            return false;
        };
        let h = |c: &str| call_hash(c.as_bytes());
        self.keys = if hop == 1 {
            let next = if r.via2.is_empty() { &r.target } else { &r.via2 };
            [(h(&self.call), h(&r.source)), (h(next), h(&r.via1))]
        } else {
            [(h(&self.call), h(&r.via1)), (h(&r.target), h(&r.via2))]
        };
        self.interval = watchdog(if hop == 1 && r.via2.is_empty() { 1 } else { 2 });
        let fresh = !self.active;
        self.active = true;
        self.restart(now);
        if fresh {
            self.lead = [LeadIn::default(), LeadIn::default()];
            self.heard = [NO_LEAD_HEARD; 2];
            (self.link_up, self.linked, self.quitting, self.polls) = (false, false, false, 0);
            (self.last_level, self.expected, self.nack_run, self.offer) = (DataLevel::ONE, DataLevel::ONE, 0, None);
            self.bw = self.own_bw;
            let vias: Vec<&str> =
                [&r.via1, &r.via2].into_iter().filter(|v| !v.is_empty()).map(String::as_str).collect();
            self.path = format!("{} <-> {} <-> {}", r.source, vias.join(" <-> "), r.target);
            self.out.push_back(DigiOutput::Host(HostEvent::Pending));
            self.log(format!("Starting Digipeater: {}", self.path));
        }
        let fwd = Record0 {
            hop,
            wide: self.own_bw == FmBandwidth::Wide && r.wide,
            chain_snr: snr_in(snr_db, r.chain_snr),
            ..r
        };
        let f = fwd.restamp(bytes);
        self.data_side = Side::Target;
        self.set_mask(Vec::new());
        self.out.push_back(DigiOutput::Tx(DigiTx::Data {
            record: Record::Connect,
            frame: f,
            lead: CONNECT_LEAD_IN,
            side: Side::Target,
        }));
        self.announce();
        true
    }

    pub fn on_sync(&mut self, now: Time) {
        if self.active {
            self.restart(now);
        }
    }

    pub fn on_data(&mut self, now: Time, record: Record, ok: bool, bytes: &[u8], snr_db: f64) {
        if !self.active || self.quitting || record == Record::Connect {
            return;
        }
        self.restart(now);
        let (from, to) = (self.data_side, self.data_side.other());
        if !ok {
            self.log(format!("rec{record} from side {from} failed: relay failed"));
            self.send_ctrl(RELAY_FAILED, to);
            return;
        }
        let Some(mut t) = Trailer::read(record, bytes, true) else { return };
        let Some(chain) = t.chain_snr else { return };
        self.lead[from.idx()].report(t.lead_report);
        t.lead_report = self.heard[to.idx()];
        if record == Record::Start {
            self.offer = Some(usize::from(t.level)).filter(|&l| l != 0);
        }
        t.chain_snr = Some(snr_in(snr_db, chain));
        let Some(f) = t.restamp(record, bytes) else { return };
        self.mask_after_frame(record);
        let lead = self.lead[to.idx()].data_symbols();
        self.out.push_back(DigiOutput::Tx(DigiTx::Data { record, frame: f, lead, side: to }));
        self.announce();
    }

    pub fn on_ctrl(&mut self, now: Time, code: u32, lead_row: u8, side: Side) {
        if !self.active || self.quitting || !self.codes(side).contains(&code) {
            return;
        }
        self.restart(now);
        self.heard[side.idx()] = lead_row;
        let to = side.other();
        if code == QRT {
            self.log(QRT_RECEIVED.into());
        }
        if ACK_REQ.contains(&code) {
            self.polls += 1;
            if self.polls > POLL_CAP {
                self.end(true);
                return;
            }
        } else {
            self.polls = 0;
        }
        if side == Side::Source && code == GO_AHEAD {
            self.link_up = true;
        }
        self.mask_after_code(code);
        if side == Side::Target && matches!(code, CONNECT_REPLY_NARROW | CONNECT_REPLY_WIDE) {
            if !self.linked {
                self.log(format!("Digipeating: {}", self.path));
            }
            self.linked = true;
            self.bw = if code == CONNECT_REPLY_WIDE { FmBandwidth::Wide } else { FmBandwidth::Narrow };
        }
        self.send_ctrl(code, to);
        if code == BREAK {
            self.data_side = side;
        }
        if code == QRT {
            self.quitting = true;
        }
        self.announce();
    }

    pub fn on_tx_done(&mut self) {
        if self.quitting {
            self.end(false);
        }
    }

    pub fn poll(&mut self, now: Time) {
        if self.deadline.is_some_and(|d| d <= now) {
            if self.link_up {
                self.log("Connection timed out".into());
            }
            self.end(true);
        }
    }

    pub fn abort(&mut self) {
        if self.active {
            self.end(false);
        }
    }

    fn end(&mut self, timeout: bool) {
        if self.linked {
            self.log(format!("Digipeater is disconnected{}", if timeout { " (Timeout)" } else { "" }));
        }
        self.active = false;
        self.deadline = None;
        self.quitting = false;
        self.bw = self.own_bw;
        self.keys = [(0, 0); 2];
        self.out.push_back(DigiOutput::Host(HostEvent::CancelPending));
        self.out.push_back(DigiOutput::Done);
    }

    fn send_ctrl(&mut self, code: u32, side: Side) {
        let lead = usize::from(self.lead[side.idx()].len);
        self.out.push_back(DigiOutput::Tx(DigiTx::Ctrl { code, lead, side }));
    }

    fn codes(&self, side: Side) -> &'static [u32] {
        const DATA: [u32; 9] = [
            QRT,
            BREAK,
            ACK_REQ_0,
            ACK_REQ_1,
            IDLE,
            NO_TRAFFIC,
            RELAY_FAILED,
            CONNECT_REPLY_NARROW,
            CONNECT_REPLY_WIDE,
        ];
        const ACKS: [u32; 17] = [
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
            ACK_REQ_0,
            ACK_REQ_1,
            GO_AHEAD,
            IDLE,
            NO_TRAFFIC,
            RELAY_FAILED,
        ];
        if side != self.data_side {
            &ACKS
        } else if self.data_side == Side::Target {
            &DATA
        } else {
            &DATA[..7]
        }
    }

    fn top(&self) -> DataLevel {
        self.bw.top_level()
    }

    fn at(&self, l: usize) -> Vec<Record> {
        let l = DataLevel::clamped(l, self.top());
        if l == DataLevel::ONE { vec![l.into(), Record::Short] } else { vec![l.into()] }
    }

    fn mask_after_frame(&mut self, record: Record) {
        let l = record.data_level().filter(|&l| l <= self.top()).unwrap_or(DataLevel::ONE);
        self.last_level = l;
        self.nack_run = 0;
        if record != Record::Start {
            self.offer = None;
        }
        self.set_mask(self.at(l.get()));
    }

    fn mask_after_code(&mut self, code: u32) {
        let l = self.last_level.get();
        let m = match code {
            ACK_STAY => {
                self.expected = self.last_level;
                self.at(l)
            }
            ACK_UP1 => self.at(l + 1),
            ACK_UP2 => self.at(l + 2),
            ACK_UP3 => self.at(l + 3),
            ACK_UP6 => self.at(l + 6),
            ACK_DOWN => self.at(self.last_level.down().get()),
            GO_AHEAD => {
                self.expected = DataLevel::ONE;
                self.offer.map_or_else(|| vec![Record::Start], |l| self.at(l))
            }
            NACK => {
                let e = self.expected.get();
                let m = if e >= 2 {
                    let lo = e.saturating_sub(2 + self.nack_run).max(1);
                    (lo..e).flat_map(|l| self.at(l)).collect()
                } else {
                    self.at(1)
                };
                self.nack_run += 1;
                m
            }
            _ => Vec::new(),
        };
        self.set_mask(m);
    }

    fn set_mask(&mut self, mut m: Vec<Record>) {
        if !self.link_up {
            m.push(Record::Connect);
        }
        m.sort_unstable();
        m.dedup();
        self.mask = m;
    }

    fn announce(&mut self) {
        let ctrl = Side::BOTH.map(|s| {
            let (key_d, key_u) = self.keys[s.idx()];
            CtrlListen { key_d, key_u, codes: self.codes(s).to_vec() }
        });
        self.out.push_back(DigiOutput::Listen(DigiListen {
            bw: self.bw,
            data_key: self.data_key(),
            records: self.mask.clone(),
            ctrl,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SECOND;
    use xara_phy::fm::frame::{Trailer, build_data, parse_data};

    fn rec(i: usize) -> Record {
        Record::from_index(i).unwrap()
    }

    fn connect(src: &str, via1: &str, via2: &str, dst: &str, hop: u8) -> Vec<u8> {
        let mut r = Record0::connect(src, dst, via1, via2, true, false);
        r.hop = hop;
        r.to_bytes().unwrap()
    }

    fn drain(d: &mut Digi) -> Vec<DigiOutput> {
        std::iter::from_fn(|| d.pop()).collect()
    }

    fn txs(o: &[DigiOutput]) -> Vec<DigiTx> {
        o.iter().filter_map(|x| if let DigiOutput::Tx(t) = x { Some(t.clone()) } else { None }).collect()
    }

    #[test]
    fn via1_forwards_record0_with_hop_and_keys() {
        let mut d = Digi::new("N2CALL", FmBandwidth::Narrow);
        assert!(d.on_record0(0, &connect("N0CALL", "N2CALL", "", "N1CALL", 0), 20.0));
        let o = drain(&mut d);
        assert!(o.contains(&DigiOutput::Host(HostEvent::Pending)));
        let t = txs(&o);
        let DigiTx::Data { record: Record::Connect, frame, side: Side::Target, .. } = &t[0] else { panic!("{t:?}") };
        let r = Record0::parse(frame).unwrap();
        assert_eq!((r.hop, r.registered, r.source.as_str(), r.target.as_str()), (1, true, "N0CALL", "N1CALL"));
        assert_eq!(r.chain_snr, snr_byte(20.0 + SNR_OFFSET_DB));
        assert_eq!(d.key(Side::Source), call_hash(b"N2CALL"));
        assert_eq!(d.key(Side::Target), call_hash(b"N1CALL"));
        assert!(!Digi::new("N2CALL", FmBandwidth::Narrow).on_record0(0, frame, 20.0));
        let mut v2 = Digi::new("N3CALL", FmBandwidth::Narrow);
        assert!(!v2.on_record0(0, &connect("N0CALL", "N2CALL", "N3CALL", "N1CALL", 0), 20.0));
        assert!(v2.on_record0(0, &connect("N0CALL", "N2CALL", "N3CALL", "N1CALL", 1), 20.0));
        assert_eq!((v2.key(Side::Source), v2.key(Side::Target)), (call_hash(b"N3CALL"), call_hash(b"N1CALL")));
    }

    #[test]
    fn relays_codes_and_forwards_data() {
        let mut d = Digi::new("N2CALL", FmBandwidth::Narrow);
        d.on_record0(0, &connect("N0CALL", "N2CALL", "", "N1CALL", 0), 20.0);
        drain(&mut d);
        d.on_ctrl(1, 23, 1, Side::Target);
        assert_eq!(txs(&drain(&mut d)), vec![DigiTx::Ctrl { code: 23, lead: 28, side: Side::Source }]);
        d.on_ctrl(2, 14, 28, Side::Source);
        let o = drain(&mut d);
        assert_eq!(txs(&o)[0], DigiTx::Ctrl { code: 14, lead: 28, side: Side::Target });
        let Some(DigiOutput::Listen(l)) = o.last() else { panic!() };
        assert_eq!((l.records.clone(), l.data_key), (vec![Record::Start], call_hash(b"N1CALL")));
        let len = FmBandwidth::Narrow.level(Record::Start).frame_bytes();
        let f = build_data(
            Record::Start,
            len,
            b"hello",
            false,
            3,
            false,
            Trailer { lead_report: 5, level: 4, chain_snr: Some(0) },
        );
        d.on_data(3, Record::Start, true, &f, 15.0);
        let t = txs(&drain(&mut d));
        let DigiTx::Data { record: Record::Start, frame, side: Side::Source, .. } = &t[0] else { panic!("{t:?}") };
        let p = parse_data(Record::Start, frame, true).unwrap();
        assert_eq!(p.data, b"hello");
        assert_eq!(p.trailer, Trailer { lead_report: 28, level: 4, chain_snr: Some(snr_byte(15.0 + SNR_OFFSET_DB)) });
        d.on_data(4, Record::Start, false, &[], 0.0);
        assert_eq!(txs(&drain(&mut d)), vec![DigiTx::Ctrl { code: 22, lead: 28, side: Side::Source }]);
        d.on_ctrl(5, 7, 28, Side::Source);
        let o = drain(&mut d);
        let Some(DigiOutput::Listen(l)) = o.last() else { panic!() };
        assert_eq!(l.records, vec![rec(2)]);
        d.on_ctrl(6, 4, 28, Side::Source);
        drain(&mut d);
        d.on_ctrl(7, 14, 28, Side::Target);
        let o = drain(&mut d);
        let Some(DigiOutput::Listen(l)) = o.last() else { panic!() };
        assert_eq!((l.data_key, l.records.clone()), (call_hash(b"N2CALL"), vec![rec(4)]));
        d.on_ctrl(8, 3, 28, Side::Target);
        assert_eq!(txs(&drain(&mut d)), vec![DigiTx::Ctrl { code: 3, lead: 28, side: Side::Source }]);
        d.on_tx_done();
        assert!(!d.active());
        assert!(drain(&mut d).contains(&DigiOutput::Host(HostEvent::CancelPending)));
    }

    #[test]
    fn times_out() {
        let mut d = Digi::new("N2CALL", FmBandwidth::Narrow);
        d.on_record0(0, &connect("N0CALL", "N2CALL", "", "N1CALL", 0), 20.0);
        d.poll(22 * SECOND);
        assert!(d.active());
        d.poll(23 * SECOND);
        assert!(!d.active());
    }
}
