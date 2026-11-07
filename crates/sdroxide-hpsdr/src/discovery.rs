//! OpenHPSDR device discovery over UDP port 1024.
//!
//! Broadcasts both a Protocol 1 and a Protocol 2 discovery request so we can
//! enumerate every board on the LAN and report which protocol each speaks. Both
//! protocols are drivable, so every responder is selectable.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig};
use sdroxide_types::HpsdrDevice;

use crate::protocol2::port;

/// Protocol 2 discovery request: 60 bytes, command `0x02` at offset 4.
fn p2_request() -> [u8; 60] {
    let mut b = [0u8; 60];
    b[4] = 0x02;
    b
}

/// Protocol 1 (Metis) discovery request: `0xEF 0xFE 0x02` then padding.
fn p1_request() -> [u8; 63] {
    let mut b = [0u8; 63];
    b[0] = 0xEF;
    b[1] = 0xFE;
    b[2] = 0x02;
    b
}

/// Map a board-type id to a human name for the given protocol.
///
/// The commercial name is carried alongside the board's own, because the board
/// is what the gateware reports and the radio is what the operator bought: an
/// Apache ANAN-100D says "Angelia" and nothing on screen said the two were the
/// same radio, which is a poor start to working out why one will not stream
/// (issue #365). `board_is_hermes_lite` matches on the prefix, so only the
/// tail of these ever changes.
fn board_name(protocol: u8, id: u8) -> String {
    let name = match (protocol, id) {
        (1, 0) => "Metis",
        (1, 1) => "Hermes (ANAN-10/10E/100/100B)",
        (1, 2) => "Griffin",
        (1, 4) => "Angelia (ANAN-100D)",
        (1, 5) => "Orion (ANAN-200D)",
        (1, 6) => "Hermes-Lite 2",
        (1, 10) => "Orion 2 (ANAN-7000/8000)",
        (2, 0) => "Atlas/Metis",
        (2, 1) => "Hermes (ANAN-10/10E/100/100B)",
        (2, 2) => "Hermes2",
        (2, 3) => "Angelia (ANAN-100D)",
        (2, 4) => "Orion (ANAN-200D)",
        (2, 5) => "Orion 2 (ANAN-7000/8000)",
        (2, 6) => "Hermes-Lite 2",
        (2, 10) => "Saturn (ANAN-G2)",
        _ => return format!("HPSDR board {id}"),
    };
    name.to_string()
}

fn fmt_mac(m: &[u8]) -> String {
    m.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// Parse a discovery response. The caller fills in `ip` from the datagram
/// source address.
fn parse_response(pkt: &[u8]) -> Option<HpsdrDevice> {
    // Protocol 1 responses begin with the Metis sync 0xEF 0xFE.
    if pkt.len() >= 11 && pkt[0] == 0xEF && pkt[1] == 0xFE {
        let status = pkt[2];
        if status != 0x02 && status != 0x03 {
            return None;
        }
        // An HL2 can run gateware that reports board id 1, "Hermes
        // emulation". Named a Hermes, it never had its onboard PA switched on
        // (#578). Its HL2-only fields still give it away: 0x13 is the receiver
        // count and 0x14[5:0] the build id (5, 3 or 2); a Metis or Hermes
        // leaves both zero.
        let board = if pkt[10] == 1 && looks_like_hl2(pkt) {
            "Hermes-Lite 2 (Hermes emulation)".to_string()
        } else {
            board_name(1, pkt[10])
        };
        return Some(HpsdrDevice {
            ip: String::new(),
            mac: fmt_mac(&pkt[3..9]),
            board,
            protocol: 1,
            in_use: status == 0x03,
        });
    }
    // Protocol 2 responses: 4-byte sequence prefix, status at offset 4.
    if pkt.len() >= 12 {
        let status = pkt[4];
        if status == 0x02 || status == 0x03 {
            return Some(HpsdrDevice {
                ip: String::new(),
                mac: fmt_mac(&pkt[5..11]),
                board: board_name(2, pkt[11]),
                protocol: 2,
                in_use: status == 0x03,
            });
        }
    }
    None
}

/// The HL2-only fields of a Protocol 1 discovery reply. See [`parse_response`].
fn looks_like_hl2(pkt: &[u8]) -> bool {
    pkt.len() > 0x14 && pkt[0x13] != 0 && matches!(pkt[0x14] & 0x3F, 2 | 3 | 5)
}

/// Destination addresses to send discovery requests to: the global broadcast
/// plus each local IPv4 interface's directed broadcast.
fn broadcast_targets() -> Vec<SocketAddr> {
    let mut dests: Vec<SocketAddr> = vec![(Ipv4Addr::BROADCAST, port::GENERAL).into()];
    for iface in NetworkInterface::show().unwrap_or_default() {
        for addr in iface.addr {
            if let Addr::V4(v4) = addr {
                if let Some(bcast) = v4.broadcast {
                    let d: SocketAddr = (bcast, port::GENERAL).into();
                    if !dests.contains(&d) {
                        dests.push(d);
                    }
                }
            }
        }
    }
    dests
}

/// Broadcast-discover HPSDR devices on the LAN, collecting responders until
/// `timeout` elapses.
pub fn discover(timeout: Duration) -> Vec<HpsdrDevice> {
    discover_impl(&broadcast_targets(), timeout)
}

/// Directed-unicast probe of a single known IP (for a manually configured
/// target). Returns the device if it answers within `timeout`.
pub fn probe(ip: Ipv4Addr, timeout: Duration) -> Option<HpsdrDevice> {
    discover_impl(&[(ip, port::GENERAL).into()], timeout).into_iter().next()
}

fn discover_impl(dests: &[SocketAddr], timeout: Duration) -> Vec<HpsdrDevice> {
    let socket = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("HPSDR discovery: bind failed: {e}");
            return Vec::new();
        }
    };
    let _ = socket.set_broadcast(true);
    let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));

    let p1 = p1_request();
    let p2 = p2_request();
    tracing::debug!(
        "HPSDR discovery: sending P1 + P2 requests to {} target(s): {:?}",
        dests.len(),
        dests
    );
    for d in dests {
        let _ = socket.send_to(&p1, d);
        let _ = socket.send_to(&p2, d);
    }

    let deadline = Instant::now() + timeout;
    let mut found: Vec<HpsdrDevice> = Vec::new();
    let mut buf = [0u8; 256];
    while Instant::now() < deadline {
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                match parse_response(&buf[..n]) {
                    Some(mut dev) => {
                        dev.ip = src.ip().to_string();
                        tracing::debug!(
                            "HPSDR discovery: reply from {} ({n} bytes): board \"{}\", \
                             Protocol {}, MAC {}, {} [{}]",
                            src,
                            dev.board,
                            dev.protocol,
                            dev.mac,
                            if dev.in_use { "in use" } else { "idle" },
                            crate::net::hex_head(&buf[..n], 16),
                        );
                        // De-dup by IP; prefer the Protocol 2 answer if a board
                        // replies to both requests.
                        if let Some(existing) = found.iter_mut().find(|d| d.ip == dev.ip) {
                            if dev.protocol > existing.protocol {
                                *existing = dev;
                            }
                        } else {
                            found.push(dev);
                        }
                    }
                    None => {
                        tracing::trace!(
                            "HPSDR discovery: ignored {n}-byte datagram from {src} [{}]",
                            crate::net::hex_head(&buf[..n], 16),
                        );
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => {
                tracing::debug!("HPSDR discovery recv error: {e}");
                break;
            }
        }
    }
    found.sort_by(|a, b| a.ip.cmp(&b.ip));
    tracing::info!(
        "HPSDR discovery: {} device(s) found{}",
        found.len(),
        if found.is_empty() {
            String::new()
        } else {
            format!(
                ": {}",
                found
                    .iter()
                    .map(|d| format!("{} (P{}, {})", d.ip, d.protocol, d.board))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    );
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_p2_response() {
        let mut pkt = [0u8; 60];
        pkt[4] = 0x02; // idle
        pkt[5..11].copy_from_slice(&[0x00, 0x1C, 0xC0, 0xA2, 0x33, 0x44]);
        pkt[11] = 10; // Saturn
        let d = parse_response(&pkt).expect("parsed");
        assert_eq!(d.protocol, 2);
        assert_eq!(d.board, "Saturn (ANAN-G2)");
        assert!(!d.in_use);
        assert_eq!(d.mac, "00:1C:C0:A2:33:44");
    }

    #[test]
    fn parse_p1_response_hl2() {
        let mut pkt = [0u8; 60];
        pkt[0] = 0xEF;
        pkt[1] = 0xFE;
        pkt[2] = 0x03; // in use
        pkt[3..9].copy_from_slice(&[0x00, 0x1C, 0xC0, 0x11, 0x22, 0x33]);
        pkt[10] = 6; // Hermes-Lite 2
        let d = parse_response(&pkt).expect("parsed");
        assert_eq!(d.protocol, 1);
        assert_eq!(d.board, "Hermes-Lite 2");
        assert!(d.in_use);
        assert!(d.supported()); // Protocol 1 is now driven
    }

    /// The Hermes-Lite's name is matched on by prefix, and the commercial
    /// names hung on the others must not reach into it.
    /// #578: an HL2 on Hermes-emulation gateware reports board id 1, and has
    /// to be recognised by its HL2-only fields or its PA is never enabled.
    #[test]
    fn parse_p1_response_hl2_in_hermes_emulation() {
        let mut pkt = [0u8; 60];
        pkt[0] = 0xEF;
        pkt[1] = 0xFE;
        pkt[2] = 0x02;
        pkt[10] = 1; // Hermes emulation
        pkt[0x13] = 4; // receivers
        pkt[0x14] = 0x05; // build id
        let d = parse_response(&pkt).expect("parsed");
        assert!(crate::net::board_is_hermes_lite(&d.board), "{}", d.board);

        // A real Hermes leaves those fields zero.
        pkt[0x13] = 0;
        pkt[0x14] = 0;
        let d = parse_response(&pkt).expect("parsed");
        assert_eq!(d.board, "Hermes (ANAN-10/10E/100/100B)");
    }

    #[test]
    fn only_the_hermes_lite_answers_to_the_hermes_lite_prefix() {
        for proto in [1u8, 2] {
            for id in 0..=10u8 {
                let name = board_name(proto, id);
                assert_eq!(
                    crate::net::board_is_hermes_lite(&name),
                    id == 6,
                    "P{proto} board {id} reads {name:?}"
                );
            }
        }
    }

    #[test]
    fn junk_is_ignored() {
        assert!(parse_response(&[0u8; 4]).is_none());
        assert!(parse_response(&[0xAA; 60]).is_none()); // status byte not 0x02/0x03
    }
}
