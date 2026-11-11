//! How a B2F session reaches the other end.
//!
//! Today there is one implementation — the CMS telnet gateway over the
//! internet. The trait exists now, before it has a second implementer, because
//! the whole point of the phased plan is that a packet or ARDOP link drops in
//! here without [`crate::session`] learning anything about radios.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// The Winlink CMS telnet gateway.
pub const CMS_ADDRESS: &str = "server.winlink.org:8772";
/// The gateway's own shared password — not the operator's account password.
/// This one gets the socket open; the account password answers the `;PQ:`
/// challenge later, inside the B2F handshake.
pub const CMS_PASSWORD: &str = "CMSTelnet";
/// Who we are forwarding with, once connected.
pub const CMS_TARGET_CALL: &str = "wl2k";

/// A bidirectional byte stream to a forwarding partner, plus the callsign that
/// partner answers to.
pub trait Transport: Read + Write + Send {
    /// The station we are forwarding with — `wl2k` for the CMS.
    fn target_call(&self) -> &str;
    /// Human-readable, for the session log.
    fn describe(&self) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("connecting to {0}: {1}")]
    Connect(String, std::io::Error),
    #[error("telnet login: {0}")]
    Io(#[from] std::io::Error),
    #[error("telnet login: connection closed before a password prompt arrived")]
    NoPrompt,
}

/// A CMS connection over plain TCP.
pub struct TelnetTransport {
    stream: TcpStream,
    address: String,
    /// The gateway login exchange, for the front of the session transcript.
    /// The gateway password is a published constant, not a secret, but the
    /// operator's callsign line is echoed here and nothing else is.
    login_log: Vec<String>,
}

impl TelnetTransport {
    /// What the gateway said while logging in.
    pub fn login_log(&self) -> &[String] {
        &self.login_log
    }

    /// Dial the CMS and get through its callsign/password prompts, leaving the
    /// stream positioned at the start of the B2F handshake.
    ///
    /// `callsign` is the operator's; the password here is the gateway's fixed
    /// [`CMS_PASSWORD`], which is why this function does not take one.
    pub fn dial(address: &str, callsign: &str, timeout: Duration) -> Result<Self, TransportError> {
        let addr = address
            .to_socket_addrs()
            .map_err(|e| TransportError::Connect(address.into(), e))?
            .next()
            .ok_or_else(|| {
                TransportError::Connect(
                    address.into(),
                    std::io::Error::new(std::io::ErrorKind::NotFound, "no address"),
                )
            })?;

        let stream = TcpStream::connect_timeout(&addr, timeout)
            .map_err(|e| TransportError::Connect(address.into(), e))?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        stream.set_nodelay(true)?;

        let login_log = login(&stream, callsign)?;

        Ok(TelnetTransport { stream, address: address.to_string(), login_log })
    }
}

/// Answer the gateway's login prompts, returning the exchange for the log.
///
/// The prompts are `\r`-terminated and matched case-insensitively on their
/// first word: the exact wording has changed over the years, the leading token
/// has not.
///
/// Reads **one byte at a time** rather than through a `BufReader`.
///
/// A buffered reader would be free to pull in whatever else has arrived, and
/// the bytes after the password prompt are the start of the B2F handshake —
/// which belongs to the session, not to us. Anything this function swallowed
/// would be silently lost when it returns, and the symptom is a session that
/// sees no SID for reasons nothing in the transcript explains.
fn login(stream: &TcpStream, callsign: &str) -> Result<Vec<String>, TransportError> {
    let mut reader = stream.try_clone()?;
    let mut writer = stream.try_clone()?;
    let mut log = Vec::new();
    let mut line = Vec::new();

    loop {
        let mut byte = [0u8; 1];
        if reader.read(&mut byte)? == 0 {
            return Err(TransportError::NoPrompt);
        }
        if byte[0] != b'\r' && byte[0] != b'\n' {
            line.push(byte[0]);
            continue;
        }

        let text = String::from_utf8_lossy(&line).trim().to_string();
        line.clear();
        if text.is_empty() {
            continue;
        }
        log.push(format!("< {text}"));

        let lower = text.to_lowercase();
        if lower.starts_with("callsign") {
            write!(writer, "{callsign}\r")?;
            writer.flush()?;
            log.push(format!("> {callsign}"));
        } else if lower.starts_with("password") {
            write!(writer, "{CMS_PASSWORD}\r")?;
            writer.flush()?;
            log.push(format!("> {CMS_PASSWORD}"));
            return Ok(log);
        }
    }
}

impl Read for TelnetTransport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for TelnetTransport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Transport for TelnetTransport {
    fn target_call(&self) -> &str {
        CMS_TARGET_CALL
    }
    fn describe(&self) -> String {
        format!("CMS telnet {}", self.address)
    }
}

// ───────────────────────────── the radio lane ─────────────────────────────

/// How long the transport waits before checking whether it has been cancelled.
const CANCEL_POLL: Duration = Duration::from_millis(250);

/// A B2F session over an AX.25 connected-mode link.
///
/// The second implementer the trait above was written for. Everything radio-
/// shaped lives behind [`sdroxide_ax25::PortHandle`]: this holds no modem, no
/// timers and no PTT, only a byte stream and the callsign at the far end.
///
/// # Why `read` never times out
///
/// [`crate::session`]'s reader turns any `io::Error` into a dead session, and
/// `Ok(0)` into a disconnect. Over TCP a 30-second silence means something is
/// wrong; over 300-baud HF packet, with T1 retries against a fading path, a
/// minute of silence is an ordinary part of a working session. A socket-style
/// timeout here would kill sessions that were about to succeed.
///
/// AX.25 already has a failure model — T1 retried N2 times — and when it gives
/// up the link says so. That is what this surfaces as `BrokenPipe`. `Ok(0)` is
/// reserved for a clean DISC/UA. The belt-and-braces upper bound belongs in
/// [`crate::manager`], as a whole-session watchdog, not here.
#[derive(Debug)]
pub struct Ax25Transport {
    port: sdroxide_ax25::PortHandle,
    /// Held for as long as the transport lives, so nothing else can drive the
    /// link underneath a session in progress.
    _lease: sdroxide_ax25::PortLease,
    gateway: String,
    /// Set when the operator asks to stop. Watched everywhere this would
    /// otherwise block, so an abort takes effect in well under a second rather
    /// than at the end of a two-minute dial timeout.
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Bytes delivered by the link but not yet read by the session.
    inbox: std::collections::VecDeque<u8>,
    /// The link has closed cleanly; hand back what is left, then `Ok(0)`.
    closed: bool,
}

impl Ax25Transport {
    /// Connect to `gateway` and wait for the link to come up.
    ///
    /// Blocks until the far end answers or the link gives up, which on HF can
    /// be tens of seconds — this runs on the Winlink worker thread, which
    /// exists precisely so that is somebody else's problem.
    pub fn connect(
        port: sdroxide_ax25::PortHandle,
        gateway: &str,
        via: &[String],
        timeout: Duration,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<Self, TransportError> {
        let lease = port.try_claim().ok_or_else(|| {
            TransportError::Connect(
                gateway.into(),
                std::io::Error::other("the packet link is already in use"),
            )
        })?;
        let peer = sdroxide_ax25::Addr::new(gateway).map_err(|e| {
            TransportError::Connect(gateway.into(), std::io::Error::other(e.to_string()))
        })?;
        let digis = via
            .iter()
            .map(|d| sdroxide_ax25::Addr::new(d))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                TransportError::Connect(gateway.into(), std::io::Error::other(e.to_string()))
            })?;

        port.send(sdroxide_ax25::PortRequest::Connect { peer, via: digis, ext: false }).map_err(
            |e| TransportError::Connect(gateway.into(), std::io::Error::other(e.to_string())),
        )?;

        let deadline = std::time::Instant::now() + timeout;
        let mut inbox = std::collections::VecDeque::new();
        loop {
            // Checked every quarter second rather than once at the end: a call
            // to an unattended gateway takes two minutes to time out, and an
            // operator who has changed their mind should not have to watch it.
            if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(TransportError::Connect(
                    gateway.into(),
                    std::io::Error::new(std::io::ErrorKind::Interrupted, "stopped"),
                ));
            }
            let left =
                deadline.saturating_duration_since(std::time::Instant::now()).min(CANCEL_POLL);
            if deadline <= std::time::Instant::now() {
                return Err(TransportError::Connect(
                    gateway.into(),
                    std::io::Error::new(std::io::ErrorKind::TimedOut, "the gateway did not answer"),
                ));
            }
            match port.recv_timeout(left).map_err(|e| {
                TransportError::Connect(gateway.into(), std::io::Error::other(e.to_string()))
            })? {
                Some(sdroxide_ax25::PortEvent::Connected) => {
                    return Ok(Ax25Transport {
                        port,
                        _lease: lease,
                        gateway: gateway.to_string(),
                        cancel,
                        inbox,
                        closed: false,
                    });
                }
                // A gateway that starts talking before we have processed the
                // connect event is not an error; keep the bytes.
                Some(sdroxide_ax25::PortEvent::Data(d)) => inbox.extend(d),
                Some(sdroxide_ax25::PortEvent::Disconnected) => {
                    return Err(TransportError::Connect(
                        gateway.into(),
                        std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            "the gateway hung up during the handshake",
                        ),
                    ));
                }
                Some(sdroxide_ax25::PortEvent::Failed(why)) => {
                    return Err(TransportError::Connect(
                        gateway.into(),
                        std::io::Error::other(why),
                    ));
                }
                None => {}
            }
        }
    }

    /// Pull whatever the link has, blocking until there is something.
    ///
    /// Waits in short hops rather than one long block, so an abort is noticed
    /// promptly. There is no read *timeout* — see the type's note — only a
    /// cancel check between hops.
    fn fill(&mut self) -> std::io::Result<()> {
        while self.inbox.is_empty() && !self.closed {
            if self.cancel.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "stopped by the operator",
                ));
            }
            match self.port.recv_timeout(CANCEL_POLL) {
                Ok(Some(sdroxide_ax25::PortEvent::Data(d))) => self.inbox.extend(d),
                Ok(Some(sdroxide_ax25::PortEvent::Connected)) => {}
                Ok(Some(sdroxide_ax25::PortEvent::Disconnected)) => self.closed = true,
                Ok(Some(sdroxide_ax25::PortEvent::Failed(why))) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, why));
                }
                // Nothing yet: go round, check the cancel flag, wait again.
                Ok(None) => {}
                // The controller is gone — a mode change under a running
                // session. Fail rather than wait for ever on a link that no
                // longer has another end.
                Err(e) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string()));
                }
            }
        }
        Ok(())
    }
}

impl Read for Ax25Transport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.fill()?;
        let n = buf.len().min(self.inbox.len());
        for slot in buf.iter_mut().take(n) {
            *slot = self.inbox.pop_front().unwrap_or(0);
        }
        // `Ok(0)` only once the buffered bytes are gone: a peer that sends its
        // last block and hangs up in the same breath still gets read.
        Ok(n)
    }
}

impl Write for Ax25Transport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Blocks when the link's window is full, which is the backpressure —
        // without it B2F would believe a whole message had gone out while none
        // of it had left the antenna.
        self.port
            .send(sdroxide_ax25::PortRequest::Data(buf.to_vec()))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string()))?;
        Ok(buf.len())
    }

    /// Nothing to flush: `write` has already handed the bytes to the link, and
    /// when they reach the air is CSMA's business, not ours.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Ax25Transport {
    fn drop(&mut self) {
        let _ = self.port.send(sdroxide_ax25::PortRequest::Disconnect);
    }
}

impl Transport for Ax25Transport {
    /// The gateway's callsign — which is why [`Transport::target_call`] exists
    /// and why the CMS constant must stop being hard-coded in the greeting.
    fn target_call(&self) -> &str {
        &self.gateway
    }

    fn describe(&self) -> String {
        format!("AX.25 packet to {}", self.gateway)
    }
}
