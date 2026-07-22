//! Native TCI (Transceiver Control Interface) client over WebSocket.
//!
//! NATIVE ONLY. Pure-Rust WebSocket (tungstenite); this crate must never be a
//! dependency of any wasm-targeted crate. It is reached only from the root
//! binary and `local_controller.rs`; the settings UI talks to it exclusively
//! through the `RadioController` trait.
//!
//! Receive is wideband IQ (sdroxide demodulates); transmit is audio (the rig
//! modulates the audio we send).

mod net;
mod protocol;

use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use tungstenite::Message;

pub use net::{TciError, TciHandle, TciUpdate, TX_RATE_HZ};

/// Default TCI port (ExpertSDR3).
pub const DEFAULT_PORT: u16 = 50001;

/// Split a `host[:port]` (or `ws://host:port`) address into `(host, port)`,
/// defaulting the port to [`DEFAULT_PORT`].
pub(crate) fn split_addr(address: &str) -> Result<(String, u16), String> {
    let a = address.trim();
    let a = a.strip_prefix("ws://").or_else(|| a.strip_prefix("wss://")).unwrap_or(a);
    let a = a.trim_end_matches('/');
    match a.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port: u16 = port.parse().map_err(|_| format!("invalid port in {address:?}"))?;
            Ok((host.to_string(), port))
        }
        _ => Ok((a.to_string(), DEFAULT_PORT)),
    }
}

/// Test a TCI server: connect, read the status burst until `ready;` (or
/// `timeout`), and return a one-line summary (device / protocol / IQ rates) or
/// an error message.
pub fn test_connection(address: &str, timeout: Duration) -> Result<String, String> {
    let (host, port) = split_addr(address)?;
    let sockaddr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host}:{port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))?;
    let stream = TcpStream::connect_timeout(&sockaddr, timeout.min(Duration::from_secs(3)))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_millis(250))).ok();

    let url = format!("ws://{host}:{port}/");
    let (mut ws, _resp) = tungstenite::client(url.as_str(), stream)
        .map_err(|e| format!("WebSocket handshake failed: {e}"))?;

    let deadline = Instant::now() + timeout;
    let mut device = String::new();
    let mut protocol = String::new();
    let mut iq_rate = String::new();
    loop {
        if Instant::now() > deadline {
            let _ = ws.close(None);
            return Err("timed out waiting for 'ready;'".into());
        }
        match ws.read() {
            Ok(Message::Text(t)) => {
                for (cmd, args) in protocol::parse_status(t.as_str()) {
                    match cmd.as_str() {
                        "device" => device = args,
                        "protocol" => protocol = args,
                        "iq_samplerate" if iq_rate.is_empty() => iq_rate = args,
                        "ready" => {
                            let _ = ws.close(None);
                            let mut s =
                                if device.is_empty() { "connected".to_string() } else { device };
                            if !protocol.is_empty() {
                                s = format!("{s}  [{protocol}]");
                            }
                            if !iq_rate.is_empty() {
                                s = format!("{s}, IQ {iq_rate} Hz");
                            }
                            return Ok(s);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => return Err("server closed the connection".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => return Err(format!("read error: {e}")),
        }
    }
}
