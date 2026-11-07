//! **Decoded CW over UDP**: the CW panel's settled copy, as plain UTF-8.
//!
//! Each datagram is the text settled since the one before: characters and the
//! spaces between words, nothing else. No header and no framing, so a listener
//! appends what arrives, and `nc -ul 9999` shows it as it comes. SDRangel's
//! Morse decoder sends its copy the same way to the same default port, so a
//! listener written for one serves the other.
//!
//! NATIVE ONLY — it binds a UDP socket.

use std::net::{ToSocketAddrs, UdpSocket};

use sdroxide_types::CwTextConfig;
use tracing::{debug, info};

/// A configured sender. Fire-and-forget, like [`crate::WsjtxUdp`]: a listener
/// that is not running must never stall the radio.
pub struct CwTextUdp {
    sock: UdpSocket,
    dest: std::net::SocketAddr,
    addr: String,
}

impl CwTextUdp {
    /// Bind a socket and resolve the destination. Fails only on a bad address
    /// or an unusable local port.
    pub fn start(cfg: &CwTextConfig) -> Result<Self, String> {
        let host = if cfg.host.trim().is_empty() { "127.0.0.1" } else { cfg.host.trim() };
        let dest = (host, cfg.port)
            .to_socket_addrs()
            .map_err(|e| format!("{host}:{}: {e}", cfg.port))?
            .next()
            .ok_or_else(|| format!("{host}:{}: no address", cfg.port))?;
        let sock = UdpSocket::bind(if dest.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })
            .map_err(|e| e.to_string())?;
        // A subnet broadcast address reaches every machine on the network, and
        // a socket has to be asked before the kernel will carry one.
        if dest.is_ipv4() {
            let _ = sock.set_broadcast(true);
        }
        if dest.is_ipv4() && dest.ip().is_multicast() {
            let _ = sock.set_multicast_ttl_v4(2);
        }
        let addr = format!("{host}:{}", cfg.port);
        info!(dest = %addr, "decoded CW UDP output started");
        Ok(CwTextUdp { sock, dest, addr })
    }

    /// The destination this sender was built for, so the engine can tell a
    /// config change that needs a rebuild from one that doesn't.
    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// Send newly settled text. Empty text sends nothing.
    pub fn send(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Err(e) = self.sock.send_to(text.as_bytes(), self.dest) {
            debug!(dest = %self.addr, error = %e, "decoded CW UDP send failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// What is sent arrives as it was sent: the bytes of the text, nothing
    /// around them.
    #[test]
    fn text_arrives_as_plain_utf8() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let cfg = CwTextConfig { enabled: true, host: "127.0.0.1".into(), port };
        let tx = CwTextUdp::start(&cfg).unwrap();
        assert_eq!(tx.addr(), format!("127.0.0.1:{port}"));

        tx.send("CQ CQ DE W1AW ");
        let mut buf = [0u8; 256];
        let n = listener.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], "CQ CQ DE W1AW ".as_bytes());
    }

    /// Nothing settled, nothing sent: an empty datagram would be an empty
    /// line to a listener that prints each one.
    #[test]
    fn empty_text_sends_nothing() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let tx = CwTextUdp::start(&CwTextConfig { enabled: true, host: "127.0.0.1".into(), port })
            .unwrap();
        tx.send("");
        let mut buf = [0u8; 16];
        assert!(listener.recv(&mut buf).is_err(), "nothing should arrive");
    }
}
