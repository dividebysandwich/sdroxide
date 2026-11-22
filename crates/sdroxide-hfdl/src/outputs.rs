//! Where decoded HFDL goes besides the panel: the feeds the operator has
//! switched on. Each decoded event is serialized once (dumphfdl's
//! `decoded:json`, see [`crate::dumphfdl`]) and sent to every target that
//! wants it.
//!
//! Sending must never hold up decoding. The local JSON targets get UDP
//! datagrams on a non-blocking socket from the worker thread, and the one slow
//! thing — looking a host name up — happens on a resolver thread of its own,
//! which refreshes the address every ten minutes so a target whose address
//! moves keeps working. A target whose name has not resolved yet simply drops
//! what would have gone to it, and says so in the status.
//!
//! Airframes is fed over TCP: its ingest wants a connection, and a datagram
//! sent at it is accepted by the network and never seen by anyone. The
//! connection, its name lookup, its reconnecting and its writes all belong to
//! a thread of its own ([`TcpFeed`]); the worker only queues lines for it, and
//! a line that cannot be queued or sent is dropped, never waited for.

use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Sender, TrySendError, bounded};
use sdroxide_types::{
    HFDL_AIRFRAMES_HOST, HFDL_AIRFRAMES_PORT, HfdlOutputStatus, HfdlOutputs, HfdlStation,
};
use xng_mode_hfdl::pdu::HfdlEvent;

use crate::dumphfdl::{self, FrameMeta};
use crate::sbs::{SbsServer, sbs_line};

/// How often a target's name is looked up again.
const RESOLVE_EVERY: Duration = Duration::from_secs(600);
/// How soon a name that failed to resolve is tried again.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// A host:port looked up on a thread of its own.
struct Target {
    addr: Arc<Mutex<Result<SocketAddr, String>>>,
    /// Dropped with the target, which ends the resolver thread.
    _stop: Sender<()>,
}

impl Target {
    fn new(host_port: String) -> Self {
        let addr = Arc::new(Mutex::new(Err(format!("looking up {host_port}…"))));
        let (stop_tx, stop_rx) = bounded::<()>(0);
        let shared = addr.clone();
        let _ = std::thread::Builder::new().name("sdroxide-hfdl-resolve".into()).spawn(move || {
            loop {
                let found = host_port
                    .to_socket_addrs()
                    .map_err(|e| format!("{host_port}: {e}"))
                    .and_then(|mut it| {
                        it.find(SocketAddr::is_ipv4)
                            .ok_or_else(|| format!("{host_port}: no IPv4 address"))
                    });
                let wait = if found.is_ok() { RESOLVE_EVERY } else { RETRY_AFTER };
                if let Ok(mut a) = shared.lock() {
                    *a = found;
                }
                // Ends when the target is dropped (the sender goes away).
                match stop_rx.recv_timeout(wait) {
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                    _ => break,
                }
            }
        });
        Target { addr, _stop: stop_tx }
    }

    fn addr(&self) -> Result<SocketAddr, String> {
        self.addr.lock().map_or_else(|_| Err("resolver failed".into()), |a| a.clone())
    }
}

/// How long a connection attempt may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// How long a write may take before the connection is given up on.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait after a failed connection before the next try.
#[cfg(not(test))]
const RECONNECT_AFTER: Duration = Duration::from_secs(15);
#[cfg(test)]
const RECONNECT_AFTER: Duration = Duration::from_millis(300);
/// Lines waiting for the connection; more than this are dropped (a feed that
/// cannot keep up is not worth holding the decoder for).
const FEED_QUEUE: usize = 512;

#[derive(Default)]
struct FeedState {
    /// Lines written to an open connection.
    sent: u64,
    /// Why there is no connection (or why the last write failed), while so.
    error: Option<String>,
    connected: bool,
}

/// A line-oriented feed over one TCP connection, kept open and re-opened by a
/// thread of its own.
struct TcpFeed {
    tx: Sender<Vec<u8>>,
    state: Arc<Mutex<FeedState>>,
}

impl TcpFeed {
    fn new(host_port: String) -> Self {
        let state = Arc::new(Mutex::new(FeedState {
            error: Some(format!("connecting to {host_port}…")),
            ..Default::default()
        }));
        let (tx, rx) = bounded::<Vec<u8>>(FEED_QUEUE);
        let shared = state.clone();
        let set = move |f: &dyn Fn(&mut FeedState)| {
            if let Ok(mut s) = shared.lock() {
                f(&mut s);
            }
        };
        let _ = std::thread::Builder::new().name("sdroxide-hfdl-feed".into()).spawn(move || {
            // Ends when the feed is dropped (the sender goes away).
            'life: loop {
                let conn = connect(&host_port);
                let mut stream = match conn {
                    Ok(c) => c,
                    Err(why) => {
                        set(&|s| {
                            s.connected = false;
                            s.error = Some(why.clone());
                        });
                        // Lines meant for a connection that is not there are
                        // dropped, not saved up to arrive late.
                        let until = std::time::Instant::now() + RECONNECT_AFTER;
                        while std::time::Instant::now() < until {
                            match rx.recv_timeout(Duration::from_millis(200)) {
                                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                                    break 'life;
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }
                };
                set(&|s| {
                    s.connected = true;
                    s.error = None;
                });
                loop {
                    match rx.recv() {
                        Err(_) => break 'life,
                        Ok(line) => match stream.write_all(&line) {
                            Ok(()) => set(&|s| s.sent += 1),
                            Err(e) => {
                                set(&|s| {
                                    s.connected = false;
                                    s.error = Some(format!("{host_port}: connection lost: {e}"));
                                });
                                break;
                            }
                        },
                    }
                }
            }
        });
        TcpFeed { tx, state }
    }

    /// Queue a line; dropped when the queue is full.
    fn send(&self, line: Vec<u8>) {
        if let Err(TrySendError::Full(_)) = self.tx.try_send(line)
            && let Ok(mut s) = self.state.lock()
            && s.connected
        {
            s.error = Some("the connection cannot keep up; dropping frames".into());
        }
    }

    fn snapshot(&self) -> (u64, Option<String>) {
        self.state.lock().map_or((0, Some("feed failed".into())), |s| (s.sent, s.error.clone()))
    }
}

/// Resolve and connect, trying each address the name gives.
fn connect(host_port: &str) -> Result<TcpStream, String> {
    let addrs: Vec<SocketAddr> =
        host_port.to_socket_addrs().map_err(|e| format!("{host_port}: {e}"))?.collect();
    let mut last = format!("{host_port}: no address");
    for a in addrs {
        match TcpStream::connect_timeout(&a, CONNECT_TIMEOUT) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                let _ = s.set_write_timeout(Some(WRITE_TIMEOUT));
                return Ok(s);
            }
            Err(e) => last = format!("{host_port}: {e}"),
        }
    }
    Err(last)
}

pub struct Outputs {
    cfg: HfdlOutputs,
    /// Airframes' HFDL ingest; a local address in tests, which must never
    /// reach the real feed.
    airframes_endpoint: String,
    sock: Option<UdpSocket>,
    airframes: Option<TcpFeed>,
    /// The local JSON targets, by the `host:port` they were given as.
    json: Vec<(String, Target)>,
    /// The BaseStation server, while one is asked for and could listen.
    sbs: Option<SbsServer>,
    status: HfdlOutputStatus,
}

impl Default for Outputs {
    fn default() -> Self {
        Self::new()
    }
}

impl Outputs {
    pub fn new() -> Self {
        Outputs {
            cfg: HfdlOutputs::default(),
            airframes_endpoint: format!("{HFDL_AIRFRAMES_HOST}:{HFDL_AIRFRAMES_PORT}"),
            sock: None,
            airframes: None,
            json: Vec::new(),
            sbs: None,
            status: HfdlOutputStatus::default(),
        }
    }

    /// Apply new settings: open what is newly wanted, close what is not.
    pub fn configure(&mut self, cfg: HfdlOutputs) {
        if cfg == self.cfg {
            return;
        }
        let airframes_on = cfg.airframes_ready();
        self.airframes = match (airframes_on, self.airframes.take()) {
            (true, Some(t)) => Some(t),
            (true, None) => Some(TcpFeed::new(self.airframes_endpoint.clone())),
            (false, _) => None,
        };
        if !airframes_on {
            self.status.airframes_error = None;
        }
        // The JSON targets: keep the ones still asked for (and their looked-up
        // addresses), start the new ones.
        let mut old = std::mem::take(&mut self.json);
        for t in cfg.json_targets() {
            let target = match old.iter().position(|(k, _)| *k == t) {
                Some(i) => old.remove(i).1,
                None => Target::new(t.clone()),
            };
            self.json.push((t, target));
        }
        if self.json.is_empty() {
            self.status.json_error = None;
        }
        // The BaseStation server: (re)opened when its port or reach changes,
        // and tried again on a re-apply after a port that was taken.
        if cfg.sbs_port != self.cfg.sbs_port
            || cfg.sbs_lan != self.cfg.sbs_lan
            || (self.sbs.is_none() && cfg.sbs_port != 0)
        {
            self.sbs = None;
            self.status.sbs_error = None;
            if cfg.sbs_port != 0 {
                match SbsServer::bind(cfg.sbs_port, cfg.sbs_lan) {
                    Ok(s) => self.sbs = Some(s),
                    Err(e) => self.status.sbs_error = Some(format!("port {}: {e}", cfg.sbs_port)),
                }
            }
        }
        self.cfg = cfg;
        if self.wants_udp() && self.sock.is_none() {
            match UdpSocket::bind("0.0.0.0:0").and_then(|s| s.set_nonblocking(true).map(|()| s)) {
                Ok(s) => self.sock = Some(s),
                Err(e) => tracing::warn!(%e, "HFDL outputs: no UDP socket"),
            }
        }
        if !self.wants_udp() {
            self.sock = None;
        }
    }

    /// Whether the local JSON targets need their socket.
    fn wants_udp(&self) -> bool {
        !self.json.is_empty()
    }

    /// Whether any JSON line is wanted at all.
    fn wants_lines(&self) -> bool {
        self.airframes.is_some() || self.wants_udp()
    }

    /// Whether anything is switched on — when nothing is, the worker skips
    /// serializing altogether.
    pub fn active(&self) -> bool {
        self.wants_lines() || self.sbs.is_some()
    }

    /// Housekeeping between frames: take in new BaseStation clients. Cheap,
    /// and never blocks.
    pub fn poll(&mut self) {
        if let Some(s) = self.sbs.as_mut() {
            s.accept();
            self.status.sbs_clients = s.clients() as u32;
        }
    }

    /// Send one decoded event to every output that wants it.
    pub fn emit<'a>(
        &mut self,
        e: &HfdlEvent,
        meta: &FrameMeta,
        gs: &'a dyn Fn(u8) -> Option<&'a HfdlStation>,
    ) {
        if !self.active() {
            return;
        }
        if let Some(srv) = self.sbs.as_mut()
            && let Some(line) = sbs_line(e, meta.sec)
        {
            if srv.send(&line) {
                self.status.sbs_sent += 1;
            }
            self.status.sbs_clients = srv.clients() as u32;
        }
        if !self.wants_lines() {
            return;
        }
        let meta = FrameMeta { station: Some(self.cfg.station_id.clone()), ..meta.clone() };
        let Some(line) = dumphfdl::format(e, &meta, gs) else { return };
        let mut bytes = line.to_string().into_bytes();
        bytes.push(b'\n');
        if let Some(feed) = self.airframes.as_ref() {
            feed.send(bytes.clone());
        }
        if let Some(sock) = self.sock.as_ref() {
            for (name, t) in &self.json {
                match t
                    .addr()
                    .and_then(|a| sock.send_to(&bytes, a).map_err(|e| format!("{name}: {e}")))
                {
                    Ok(_) => self.status.json_sent += 1,
                    Err(why) => self.status.json_error = Some(why),
                }
            }
        }
    }

    /// What the outputs have done, for the panel.
    pub fn status(&self) -> HfdlOutputStatus {
        let mut st = self.status.clone();
        if let Some(feed) = self.airframes.as_ref() {
            (st.airframes_sent, st.airframes_error) = feed.snapshot();
        }
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Outputs aimed at a local listener instead of Airframes.
    fn local() -> (Outputs, std::net::TcpListener) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut o = Outputs::new();
        o.airframes_endpoint = l.local_addr().unwrap().to_string();
        (o, l)
    }

    fn feeding() -> HfdlOutputs {
        HfdlOutputs { airframes: true, station_id: "XX-TEST-HFDL".into(), ..Default::default() }
    }

    fn squitter() -> (HfdlEvent, FrameMeta) {
        let e = xng_mode_hfdl::pdu::PduParser::new()
            .parse(&xng_mode_hfdl::pdu::build_spdu(4, 9, 52), 300)
            .remove(0);
        let meta =
            FrameMeta { sec: 1, usec: 2, freq_hz: 8_912_000, sig_level: -30.0, station: None };
        (e, meta)
    }

    #[test]
    fn nothing_is_open_until_asked_for() {
        let (mut o, _l) = local();
        assert!(!o.active());
        // Asked for, but without a station id Airframes would not accept: off.
        o.configure(HfdlOutputs {
            airframes: true,
            station_id: String::new(),
            ..Default::default()
        });
        assert!(!o.active());
        assert!(o.airframes.is_none());
        o.configure(feeding());
        assert!(o.active());
        assert!(o.airframes.is_some());
        assert!(o.sock.is_none(), "Airframes is TCP: no datagram socket for it");
        o.configure(HfdlOutputs::default());
        assert!(!o.active());
        assert!(o.airframes.is_none(), "the connection closes with the feed");
    }

    /// A squitter goes out over TCP as one dumphfdl JSON line, stamped with
    /// the station id, and is counted once it is written.
    #[test]
    fn a_decode_reaches_the_feed_as_a_json_line() {
        use std::io::{BufRead, BufReader};
        let (mut o, listener) = local();
        o.configure(feeding());
        let (e, meta) = squitter();
        let table = sdroxide_types::hfdl_builtin_stations();
        let lookup = |id: u8| table.iter().find(|s| s.gs_id == id);
        let (conn, _) = listener.accept().expect("the feed connects");
        conn.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        o.emit(&e, &meta, &lookup);
        let mut line = String::new();
        BufReader::new(&conn).read_line(&mut line).expect("a line");
        assert!(line.ends_with('\n'), "one line per frame");
        let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["hfdl"]["station"], "XX-TEST-HFDL");
        assert_eq!(v["hfdl"]["spdu"]["src"]["id"], 4);
        for _ in 0..200 {
            if o.status().airframes_sent > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(o.status().airframes_sent, 1);
        assert_eq!(o.status().airframes_error, None);
    }

    /// Until the connection is there the panel is told so, and nothing is
    /// counted as sent; a refused connection is retried and the reason shown.
    #[test]
    fn a_feed_with_no_connection_says_why_and_connects_when_it_can() {
        // A port nobody listens on.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut o = Outputs::new();
        o.airframes_endpoint = format!("127.0.0.1:{port}");
        o.configure(feeding());
        let (e, meta) = squitter();
        let mut err = None;
        for _ in 0..200 {
            o.emit(&e, &meta, &|_| None);
            err = o.status().airframes_error;
            if err.as_deref().is_some_and(|m| !m.starts_with("connecting")) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(err.unwrap().contains(&port.to_string()));
        assert_eq!(o.status().airframes_sent, 0, "nothing counted as sent");
        // The server comes up: the feed connects by itself and clears the error.
        let l = std::net::TcpListener::bind(("127.0.0.1", port)).expect("port free again");
        l.set_nonblocking(true).unwrap();
        let mut conn = None;
        for _ in 0..400 {
            if let Ok((c, _)) = l.accept() {
                conn = Some(c);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(conn.is_some(), "retried and connected");
    }

    /// A connection the other end closes is opened again, and later frames
    /// arrive on the new one.
    #[test]
    fn a_lost_connection_is_opened_again() {
        use std::io::{BufRead, BufReader};
        let (mut o, listener) = local();
        o.configure(feeding());
        let (e, meta) = squitter();
        let (first, _) = listener.accept().unwrap();
        drop(first);
        listener.set_nonblocking(true).unwrap();
        let mut second = None;
        for _ in 0..600 {
            // Writes to the closed connection fail, which is what makes the
            // feed notice; keep frames coming until it has reconnected.
            o.emit(&e, &meta, &|_| None);
            if let Ok((c, _)) = listener.accept() {
                second = Some(c);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let conn = second.expect("the feed came back");
        conn.set_nonblocking(false).unwrap();
        conn.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        for _ in 0..50 {
            o.emit(&e, &meta, &|_| None);
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut line = String::new();
        BufReader::new(&conn).read_line(&mut line).expect("frames flow again");
        assert!(line.contains("\"hfdl\""));
    }

    /// Local JSON targets get the same lines, and a station id is not needed
    /// for them — the id is Airframes' requirement, not the format's.
    #[test]
    fn local_targets_get_the_lines_without_a_station_id() {
        let rx_a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let rx_b = UdpSocket::bind("127.0.0.1:0").unwrap();
        for r in [&rx_a, &rx_b] {
            r.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        }
        let mut o = Outputs::new();
        o.configure(HfdlOutputs {
            json_udp: vec![
                rx_a.local_addr().unwrap().to_string(),
                rx_b.local_addr().unwrap().to_string(),
            ],
            ..Default::default()
        });
        assert!(o.active());
        let e = xng_mode_hfdl::pdu::PduParser::new()
            .parse(&xng_mode_hfdl::pdu::build_spdu(7, 9, 52), 300)
            .remove(0);
        let meta =
            FrameMeta { sec: 1, usec: 2, freq_hz: 8_942_000, sig_level: -30.0, station: None };
        let none = |_: u8| None;
        for _ in 0..200 {
            o.emit(&e, &meta, &none);
            if o.status().json_sent >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut buf = [0u8; 4096];
        for r in [&rx_a, &rx_b] {
            let (n, _) = r.recv_from(&mut buf).expect("each target gets the line");
            let v: serde_json::Value =
                serde_json::from_str(String::from_utf8_lossy(&buf[..n]).trim()).unwrap();
            assert_eq!(v["hfdl"]["spdu"]["src"]["id"], 7);
            assert!(v["hfdl"].get("station").is_none(), "no id was given");
        }
        // Dropping a target keeps the other (and its looked-up address).
        let keep = rx_b.local_addr().unwrap().to_string();
        o.configure(HfdlOutputs { json_udp: vec![keep.clone()], ..Default::default() });
        assert_eq!(o.json.len(), 1);
        assert_eq!(o.json[0].0, keep);
        assert!(o.json[0].1.addr().is_ok(), "kept, not looked up again");
    }

    /// A position report with a known ICAO reaches a connected BaseStation
    /// client; turning the port off closes the server.
    #[test]
    fn positions_reach_a_basestation_client() {
        use std::io::{BufRead, BufReader};
        // An ephemeral port: find one free, then hand it to the outputs.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut o = Outputs::new();
        o.configure(HfdlOutputs { sbs_port: port, ..Default::default() });
        assert!(o.active(), "{:?}", o.status().sbs_error);
        let client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        for _ in 0..200 {
            o.poll();
            if o.status().sbs_clients == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(o.status().sbs_clients, 1);
        let now = 1_700_000_000; // 22:13:20 UTC
        let e = HfdlEvent {
            kind: "frequency-data".into(),
            details: serde_json::json!({
                "utc": { "hour": 22, "min": 13, "sec": 0 },
                "position": { "lat": 10.0, "lon": 20.0, "flight": "QFA7", "icao": "7C1234" },
            }),
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: None,
            raw: Vec::new(),
        };
        let meta =
            FrameMeta { sec: now, usec: 0, freq_hz: 8_912_000, sig_level: -30.0, station: None };
        o.emit(&e, &meta, &|_| None);
        let mut line = String::new();
        BufReader::new(&client).read_line(&mut line).unwrap();
        assert!(line.starts_with("MSG,3,1,1,7C1234,1,2023/11/14,22:13:00.000,"), "{line}");
        assert_eq!(o.status().sbs_sent, 1);
        o.configure(HfdlOutputs::default());
        assert!(!o.active());
    }

    #[test]
    fn a_port_already_taken_is_reported_not_fatal() {
        let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = held.local_addr().unwrap().port();
        let mut o = Outputs::new();
        o.configure(HfdlOutputs { sbs_port: port, ..Default::default() });
        assert!(!o.active());
        assert!(o.status().sbs_error.unwrap().contains(&port.to_string()));
    }

    #[test]
    fn a_target_resolves_on_its_own_thread() {
        let t = Target::new("127.0.0.1:5556".into());
        let mut got = t.addr();
        for _ in 0..200 {
            if got.is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            got = t.addr();
        }
        assert_eq!(got.unwrap(), "127.0.0.1:5556".parse().unwrap());
        let bad = Target::new("no-port-here".into());
        std::thread::sleep(Duration::from_millis(50));
        assert!(bad.addr().is_err());
    }
}
