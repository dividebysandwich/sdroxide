//! Where decoded HFDL goes besides the panel: the feeds the operator has
//! switched on. Each decoded event is serialized once (dumphfdl's
//! `decoded:json`, see [`crate::dumphfdl`]) and sent to every target that
//! wants it.
//!
//! Sending must never hold up decoding: datagrams go out on a non-blocking
//! socket from the worker thread, and the one slow thing — looking a host name
//! up — happens on a resolver thread of its own, which refreshes the address
//! every ten minutes so a feed whose address moves keeps working. A target
//! whose name has not resolved yet simply drops what would have gone to it,
//! and says so in the status.

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Sender, bounded};
use sdroxide_types::{
    HFDL_AIRFRAMES_HOST, HFDL_AIRFRAMES_PORT, HfdlOutputStatus, HfdlOutputs, HfdlStation,
};
use xng_mode_hfdl::pdu::HfdlEvent;

use crate::dumphfdl::{self, FrameMeta};

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

pub struct Outputs {
    cfg: HfdlOutputs,
    /// Airframes' HFDL ingest; a local address in tests, which must never
    /// reach the real feed.
    airframes_endpoint: String,
    sock: Option<UdpSocket>,
    airframes: Option<Target>,
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
            (true, None) => Some(Target::new(self.airframes_endpoint.clone())),
            (false, _) => None,
        };
        if !airframes_on {
            self.status.airframes_error = None;
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

    fn wants_udp(&self) -> bool {
        self.airframes.is_some()
    }

    /// Whether anything is switched on — when nothing is, the worker skips
    /// serializing altogether.
    pub fn active(&self) -> bool {
        self.wants_udp()
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
        let meta = FrameMeta { station: Some(self.cfg.station_id.clone()), ..meta.clone() };
        let Some(line) = dumphfdl::format(e, &meta, gs) else { return };
        let mut bytes = line.to_string().into_bytes();
        bytes.push(b'\n');
        if let (Some(sock), Some(t)) = (self.sock.as_ref(), self.airframes.as_ref()) {
            match t.addr() {
                Ok(addr) => match sock.send_to(&bytes, addr) {
                    Ok(_) => {
                        self.status.airframes_sent += 1;
                        self.status.airframes_error = None;
                    }
                    Err(e) => self.status.airframes_error = Some(e.to_string()),
                },
                Err(why) => self.status.airframes_error = Some(why),
            }
        }
    }

    /// What the outputs have done, for the panel.
    pub fn status(&self) -> HfdlOutputStatus {
        self.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Outputs aimed at a local socket instead of Airframes.
    fn local() -> (Outputs, UdpSocket) {
        let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
        rx.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut o = Outputs::new();
        o.airframes_endpoint = rx.local_addr().unwrap().to_string();
        (o, rx)
    }

    fn feeding() -> HfdlOutputs {
        HfdlOutputs { airframes: true, station_id: "XX-TEST-HFDL".into(), ..Default::default() }
    }

    #[test]
    fn nothing_is_open_until_asked_for() {
        let (mut o, _rx) = local();
        assert!(!o.active());
        // Asked for, but without a station id Airframes would not accept: off.
        o.configure(HfdlOutputs {
            airframes: true,
            station_id: String::new(),
            ..Default::default()
        });
        assert!(!o.active());
        o.configure(feeding());
        assert!(o.active());
        assert!(o.sock.is_some());
        o.configure(HfdlOutputs::default());
        assert!(!o.active());
        assert!(o.sock.is_none(), "the socket closes with the last output");
    }

    /// A squitter goes out as one dumphfdl JSON line, stamped with the station
    /// id, and is counted.
    #[test]
    fn a_decode_reaches_the_feed_as_a_json_line() {
        let (mut o, rx) = local();
        o.configure(feeding());
        let e = xng_mode_hfdl::pdu::PduParser::new()
            .parse(&xng_mode_hfdl::pdu::build_spdu(4, 9, 52), 300)
            .remove(0);
        let meta =
            FrameMeta { sec: 1, usec: 2, freq_hz: 8_912_000, sig_level: -30.0, station: None };
        let table = sdroxide_types::hfdl_builtin_stations();
        let lookup = |id: u8| table.iter().find(|s| s.gs_id == id);
        // The name resolves on its own thread: keep emitting until it has.
        let mut buf = [0u8; 4096];
        let mut got = None;
        for _ in 0..200 {
            o.emit(&e, &meta, &lookup);
            if o.status().airframes_sent > 0 {
                let (n, _) = rx.recv_from(&mut buf).expect("a datagram");
                got = Some(String::from_utf8_lossy(&buf[..n]).to_string());
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let line = got.expect("sent once the address resolved");
        assert!(line.ends_with('\n'), "one line per frame");
        let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["hfdl"]["station"], "XX-TEST-HFDL");
        assert_eq!(v["hfdl"]["spdu"]["src"]["id"], 4);
        assert_eq!(o.status().airframes_error, None);
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
