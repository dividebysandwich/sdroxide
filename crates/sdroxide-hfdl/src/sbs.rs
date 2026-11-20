//! BaseStation (SBS-1) position output, for plotting programs.
//!
//! Virtual Radar Server, PlanePlotter and the like read aircraft positions as
//! BaseStation CSV over a TCP connection. dumphfdl offers HFDL positions this
//! way (its `basestation` formatter), and the line here is that one, field for
//! field (`fmtr-basestation.c`, read for the format only):
//!
//! ```text
//! MSG,3,1,1,<ICAO hex>,1,<date>,<time>,<date>,<time>,<flight>,,,,<lat>,<lon>,,0,,,,0
//! ```
//!
//! — an airborne-position message with the report's own time, the flight, and
//! the position; no altitude, speed or track, because an HFDL position report
//! carries none (the table's calculated course and speed are estimates, and a
//! plotter would show them as reported, so they are not sent). Like dumphfdl,
//! only a report from an aircraft whose ICAO address is known is sent (the
//! format has no other way to name it), and only while the report is under
//! five minutes old and not in the future.
//!
//! The server listens on the operator's port — on this computer only unless
//! they open it to the network — and writes each line to every connected
//! client without ever waiting: a client that cannot take a line at once is
//! too slow for a live feed and is dropped.

use std::io::{ErrorKind, Write};
use std::net::{TcpListener, TcpStream};

use xng_mode_hfdl::pdu::HfdlEvent;

/// Reports older than this are not sent (dumphfdl's `POSITION_MAX_AGE`).
pub const SBS_MAX_AGE_S: i64 = 300;

/// How far ahead of this computer's clock a report's time may be and still
/// count as now: the aircraft's clock and ours are not the same clock.
const FUTURE_SLACK_S: i64 = 30;

/// The Unix time of a report stamped `h:m:s` UTC, received at `now`: today's
/// date, or yesterday's when that puts it in the future (a report from just
/// before midnight, received just after).
pub fn report_unix(now: i64, h: i64, m: i64, s: i64) -> i64 {
    let day = now - now.rem_euclid(86_400);
    let t = day + h * 3600 + m * 60 + s;
    if t > now + FUTURE_SLACK_S { t - 86_400 } else { t }
}

/// The BaseStation line for an event, or `None` for an event that is not a
/// position report from an aircraft with a known ICAO address, or a report too
/// old or from the future.
pub fn sbs_line(e: &HfdlEvent, now: i64) -> Option<String> {
    if !matches!(e.kind.as_str(), "performance-data" | "frequency-data") {
        return None;
    }
    let d = &e.details;
    // xng leaves the position object out for the all-zero "no fix yet" value.
    let p = d.get("position")?;
    let icao = p
        .get("icao")
        .or_else(|| d.get("who").and_then(|w| w.get("icao")))
        .and_then(|v| v.as_str())
        .and_then(|s| u32::from_str_radix(s, 16).ok())?;
    let lat = p.get("lat")?.as_f64()?;
    let lon = p.get("lon")?.as_f64()?;
    let utc = d.get("utc")?;
    let hms = |k: &str| utc.get(k).and_then(|v| v.as_i64());
    let t = report_unix(now, hms("hour")?, hms("min")?, hms("sec")?);
    if t > now + FUTURE_SLACK_S || t + SBS_MAX_AGE_S < now {
        return None;
    }
    let (y, mo, da, h, mi, s) = sdroxide_types::utc_ymd_hms(t);
    let stamp = format!("{y:04}/{mo:02}/{da:02},{h:02}:{mi:02}:{s:02}.000");
    let flight = p.get("flight").and_then(|v| v.as_str()).map(str::trim).unwrap_or("");
    Some(format!("MSG,3,1,1,{icao:06X},1,{stamp},{stamp},{flight},,,,{lat:.6},{lon:.6},,0,,,,0\n"))
}

/// The listening socket and its clients.
pub struct SbsServer {
    listener: TcpListener,
    clients: Vec<TcpStream>,
}

impl SbsServer {
    /// Listen on `port`: on this computer only, or on every interface when
    /// `lan` is set. Port 0 picks one (for tests).
    pub fn bind(port: u16, lan: bool) -> std::io::Result<Self> {
        let host = if lan { "0.0.0.0" } else { "127.0.0.1" };
        let listener = TcpListener::bind((host, port))?;
        listener.set_nonblocking(true)?;
        Ok(SbsServer { listener, clients: Vec::new() })
    }

    pub fn port(&self) -> Option<u16> {
        self.listener.local_addr().ok().map(|a| a.port())
    }

    /// Take in whoever has connected since last time. Never blocks.
    pub fn accept(&mut self) {
        const MAX_CLIENTS: usize = 16;
        loop {
            match self.listener.accept() {
                // Capped, as with LAN on anyone on the network can connect.
                Ok((s, _)) if self.clients.len() >= MAX_CLIENTS => drop(s),
                Ok((s, _)) => {
                    if s.set_nonblocking(true).is_ok() {
                        let _ = s.set_nodelay(true);
                        self.clients.push(s);
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    /// Write `line` to every client; drop the ones that cannot take it whole,
    /// now. Returns whether anyone got it.
    pub fn send(&mut self, line: &str) -> bool {
        let bytes = line.as_bytes();
        self.clients.retain_mut(|c| matches!(c.write(bytes), Ok(n) if n == bytes.len()));
        !self.clients.is_empty()
    }

    pub fn clients(&self) -> usize {
        self.clients.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{BufRead, BufReader};
    use std::time::Duration;

    fn perf(icao: Option<&str>, h: i64, m: i64, s: i64) -> HfdlEvent {
        let mut pos = json!({ "lat": 52.5, "lon": -30.25, "flight": "UAL12 " });
        if let Some(i) = icao {
            pos["icao"] = json!(i);
        }
        HfdlEvent {
            kind: "performance-data".into(),
            details: json!({
                "utc": { "hour": h, "min": m, "sec": s },
                "position": pos,
                "who": { "dir": "downlink", "gs_id": 7, "aircraft_id": 12 },
            }),
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: None,
            raw: Vec::new(),
        }
    }

    /// 2023-11-14 22:13:20 UTC.
    const NOW: i64 = 1_700_000_000;

    #[test]
    fn a_position_is_dumphfdls_basestation_line() {
        let line = sbs_line(&perf(Some("a1b2c3"), 22, 12, 0), NOW).expect("a line");
        assert_eq!(
            line,
            "MSG,3,1,1,A1B2C3,1,2023/11/14,22:12:00.000,2023/11/14,22:12:00.000,UAL12,,,,52.500000,-30.250000,,0,,,,0\n"
        );
        assert_eq!(line.trim_end().split(',').count(), 22, "BaseStation's 22 fields");
    }

    #[test]
    fn only_known_aircraft_and_fresh_reports_are_sent() {
        assert!(sbs_line(&perf(None, 22, 12, 0), NOW).is_none(), "no ICAO: nothing to name it by");
        assert!(sbs_line(&perf(Some("A1B2C3"), 22, 5, 0), NOW).is_none(), "over five minutes old");
        assert!(sbs_line(&perf(Some("A1B2C3"), 22, 15, 0), NOW).is_none(), "in the future");
        let mut ack = perf(Some("A1B2C3"), 22, 12, 0);
        ack.kind = "acars".into();
        assert!(sbs_line(&ack, NOW).is_none(), "not a position report");
    }

    #[test]
    fn a_report_from_before_midnight_is_dated_yesterday() {
        // Received at 00:00:10 for a report stamped 23:59:50.
        let now = 86_400 * 19_675 + 10;
        let t = report_unix(now, 23, 59, 50);
        assert_eq!(now - t, 20);
    }

    #[test]
    fn clients_get_each_line_and_a_gone_one_is_dropped() {
        let mut srv = SbsServer::bind(0, false).unwrap();
        let port = srv.port().unwrap();
        let client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        for _ in 0..200 {
            srv.accept();
            if srv.clients() == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(srv.clients(), 1);
        assert!(srv.send("MSG,3,test\n"));
        let mut got = String::new();
        BufReader::new(&client).read_line(&mut got).unwrap();
        assert_eq!(got, "MSG,3,test\n");
        drop(client);
        // The next writes find it gone (the first may still be buffered).
        for _ in 0..50 {
            srv.send("MSG,3,again\n");
            if srv.clients() == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(srv.clients(), 0);
    }

    #[test]
    fn it_listens_on_this_computer_only_unless_asked() {
        let srv = SbsServer::bind(0, false).unwrap();
        assert!(srv.listener.local_addr().unwrap().ip().is_loopback());
    }
}
