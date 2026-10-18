//! A stratum 1 NTP server whose reference clock is the radio.
//!
//! RFC 5905 (NTPv4), section 7.3 for the packet and 9.1 for what a server puts
//! in it; RFC 4330 (SNTPv4) for the subset a server with one reference and no
//! peers needs. A client's mode 3 request is answered with a mode 4 reply,
//! version echoed, carrying three timestamps: the client's own transmit time
//! handed back (origin), and ours on receipt and on reply. The client works out
//! the rest.
//!
//! The time served is the computer's clock *corrected by the latest
//! measurement*, not the computer's clock as it stands — so a machine on the
//! network can be kept on radio time without this one ever being set, and
//! setting it changes nothing for the clients.
//!
//! While a measurement is fresh the server says so: stratum 1, the station's
//! reference identifier, and a root dispersion that grows with the
//! measurement's age at the rate RFC 5905 assumes of an undisciplined clock.
//! Without one it answers anyway, as unsynchronised — leap indicator 3,
//! stratum 16 — which every client knows not to set its clock from.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

/// Seconds from the NTP era's start (1900) to the Unix epoch (1970).
const NTP_UNIX_S: i128 = 2_208_988_800;
/// log₂ of the precision claimed: 2⁻¹⁰ s, about a millisecond — what a radio
/// path and a desktop scheduler deliver, not what the clock register holds.
const PRECISION: i8 = -10;

/// What the server answers with at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reference {
    /// A fresh enough measurement is in hand.
    pub synced: bool,
    /// RFC 5905's identifier for the radio clock, e.g. `DCF\0`.
    pub refid: [u8; 4],
    /// How far to move the computer's clock to get the served time, ns.
    pub offset_ns: i64,
    /// The served time the last measurement was made at, Unix ns.
    pub ref_ns: i128,
    /// The root dispersion to announce, seconds.
    pub dispersion_s: f64,
}

impl Reference {
    pub const UNSYNCED: Reference =
        Reference { synced: false, refid: *b"INIT", offset_ns: 0, ref_ns: 0, dispersion_s: 16.0 };
}

/// Unix nanoseconds to an NTP 32.32 timestamp.
pub fn to_ntp(unix_ns: i128) -> u64 {
    let ns = unix_ns + NTP_UNIX_S * 1_000_000_000;
    let secs = ns.div_euclid(1_000_000_000);
    let frac = ns.rem_euclid(1_000_000_000);
    ((secs as u64) << 32) | ((frac << 32) / 1_000_000_000) as u64
}

/// An NTP 32.32 timestamp to Unix nanoseconds, in the era of 1900–2036 or the
/// one after it, whichever puts it nearer the present.
pub fn from_ntp(ts: u64) -> i128 {
    let secs = i128::from(ts >> 32);
    let frac = (i128::from(ts & 0xffff_ffff) * 1_000_000_000) >> 32;
    // Era 1 starts in 2036; a timestamp below 1968 in era 0 is in era 1.
    let era = if secs < 0x8000_0000 { 1i128 << 32 } else { 0 };
    (secs + era - NTP_UNIX_S) * 1_000_000_000 + frac
}

/// Seconds in NTP's 16.16 short format.
fn short(s: f64) -> u32 {
    (s.clamp(0.0, 65_535.0) * 65_536.0) as u32
}

/// The reply to `req`, received at served time `recv_ns` and sent at
/// `xmit_ns` — or `None` if it is not a client request.
pub fn reply(req: &[u8], recv_ns: i128, xmit_ns: i128, r: &Reference) -> Option<[u8; 48]> {
    if req.len() < 48 {
        return None;
    }
    let version = (req[0] >> 3) & 7;
    let mode = req[0] & 7;
    if mode != 3 || !(1..=4).contains(&version) {
        return None;
    }
    let mut out = [0u8; 48];
    let leap = if r.synced { 0 } else { 3 };
    out[0] = (leap << 6) | (version << 3) | 4;
    out[1] = if r.synced { 1 } else { 16 };
    out[2] = req[2];
    out[3] = PRECISION as u8;
    // Root delay is zero: the reference is attached to this server.
    out[8..12].copy_from_slice(&short(r.dispersion_s).to_be_bytes());
    out[12..16].copy_from_slice(if r.synced { &r.refid } else { b"INIT" });
    if r.synced {
        out[16..24].copy_from_slice(&to_ntp(r.ref_ns).to_be_bytes());
    }
    out[24..32].copy_from_slice(&req[40..48]);
    out[32..40].copy_from_slice(&to_ntp(recv_ns).to_be_bytes());
    out[40..48].copy_from_slice(&to_ntp(xmit_ns).to_be_bytes());
    Some(out)
}

/// The served-time source the server asks once per request.
pub type ReferenceFn = Arc<dyn Fn() -> Reference + Send + Sync>;

/// A running server. Dropping it stops it.
pub struct NtpServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    requests: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl NtpServer {
    pub fn start(bind: &str, reference: ReferenceFn) -> io::Result<NtpServer> {
        let socket = UdpSocket::bind(bind)?;
        socket.set_read_timeout(Some(Duration::from_millis(250)))?;
        let addr = socket.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicU64::new(0));
        let thread = std::thread::Builder::new().name("sdroxide-ntp".into()).spawn({
            let (stop, requests) = (stop.clone(), requests.clone());
            move || {
                let mut buf = [0u8; 512];
                while !stop.load(Ordering::Relaxed) {
                    let Ok((n, from)) = socket.recv_from(&mut buf) else { continue };
                    // Both timestamps as close to the socket as they can be.
                    let recv_host = crate::os::now_ns();
                    let r = reference();
                    let off = i128::from(r.offset_ns);
                    let xmit = crate::os::now_ns() + off;
                    if let Some(out) = reply(&buf[..n], recv_host + off, xmit, &r)
                        && socket.send_to(&out, from).is_ok()
                    {
                        requests.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        })?;
        Ok(NtpServer { addr, stop, requests, thread: Some(thread) })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn requests(&self) -> u64 {
        self.requests.load(Ordering::Relaxed)
    }
}

impl Drop for NtpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(transmit: u64) -> [u8; 48] {
        let mut req = [0u8; 48];
        req[0] = (4 << 3) | 3;
        req[2] = 6;
        req[40..48].copy_from_slice(&transmit.to_be_bytes());
        req
    }

    #[test]
    fn timestamps_round_trip() {
        let t: i128 = 1_792_327_020_123_456_789;
        let back = from_ntp(to_ntp(t));
        assert!((back - t).abs() < 2, "{back} vs {t}");
        // Past the 2036 rollover.
        let late: i128 = 2_200_000_000_000_000_000;
        assert!((from_ntp(to_ntp(late)) - late).abs() < 2);
    }

    #[test]
    fn a_synced_reply_is_stratum_one_with_the_station_named() {
        let r = Reference {
            synced: true,
            refid: *b"DCF\0",
            offset_ns: 0,
            ref_ns: 1_792_327_000_000_000_000,
            dispersion_s: 0.002,
        };
        let req = request(0x1234_5678_9abc_def0);
        let out = reply(&req, 1_792_327_020_000_000_000, 1_792_327_020_000_100_000, &r).unwrap();
        assert_eq!(out[0] >> 6, 0, "leap indicator");
        assert_eq!((out[0] >> 3) & 7, 4, "version echoed");
        assert_eq!(out[0] & 7, 4, "server mode");
        assert_eq!(out[1], 1, "stratum");
        assert_eq!(&out[12..16], b"DCF\0");
        assert_eq!(&out[24..32], &req[40..48], "origin is the client's transmit time");
        let recv = u64::from_be_bytes(out[32..40].try_into().unwrap());
        assert!((from_ntp(recv) - 1_792_327_020_000_000_000).abs() < 2);
        assert_eq!(u32::from_be_bytes(out[8..12].try_into().unwrap()), short(0.002));
    }

    #[test]
    fn unsynced_says_so() {
        let out = reply(&request(1), 0, 0, &Reference::UNSYNCED).unwrap();
        assert_eq!(out[0] >> 6, 3, "alarm");
        assert_eq!(out[1], 16);
    }

    #[test]
    fn only_client_requests_are_answered() {
        let mut req = request(1);
        req[0] = (4 << 3) | 4; // a server reply, reflected back at us
        assert!(reply(&req, 0, 0, &Reference::UNSYNCED).is_none());
        assert!(reply(&req[..40], 0, 0, &Reference::UNSYNCED).is_none());
    }

    #[test]
    fn serves_the_corrected_time_over_udp() {
        let offset_ns: i64 = 3_600_000_000_000; // an hour fast
        let server = NtpServer::start(
            "127.0.0.1:0",
            Arc::new(move || Reference {
                synced: true,
                refid: *b"MSF\0",
                offset_ns,
                ref_ns: crate::os::now_ns(),
                dispersion_s: 0.001,
            }),
        )
        .expect("bind");
        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let sent = crate::os::now_ns();
        client.send_to(&request(to_ntp(sent)), server.addr()).unwrap();
        let mut buf = [0u8; 64];
        let (n, _) = client.recv_from(&mut buf).expect("reply");
        assert_eq!(n, 48);
        let xmit = from_ntp(u64::from_be_bytes(buf[40..48].try_into().unwrap()));
        let ahead = xmit - crate::os::now_ns();
        assert!((ahead - i128::from(offset_ns)).abs() < 100_000_000, "served {ahead} ns ahead");
        assert_eq!(&buf[12..16], b"MSF\0");
        assert_eq!(server.requests(), 1);
    }
}
