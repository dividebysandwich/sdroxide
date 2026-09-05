//! Errors, and the translation of connection failures into sentences an
//! operator can act on.
//!
//! Everything here ends up in front of a user: [`Error`] is what
//! `PlutoSource::open` returns and what `IqSource::open_status` puts on screen.
//! "connection refused (os error 111)" tells nobody what to do; "nothing is
//! listening on 192.168.2.1:30431 — check the USB cable" does.

use std::net::SocketAddr;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Could not reach `iiod` at all. Carries the actionable sentence, not the
    /// errno.
    #[error("{0}")]
    Unreachable(String),

    /// Socket I/O failed once the connection was up.
    #[error("IIOD {op} failed: {source}")]
    Io { op: &'static str, source: std::io::Error },

    /// The server answered, but with a negative return code (`-errno`).
    #[error("IIOD rejected `{cmd}`: {}", errno_text(*.code))]
    Remote { cmd: String, code: i64 },

    /// The server's framing did not match what the protocol says it should be.
    /// The most likely fault in a from-scratch IIOD client, so it says what was
    /// expected and what arrived.
    #[error("IIOD protocol error after `{cmd}`: {what}")]
    Protocol { cmd: String, what: String },

    /// The context description could not be parsed.
    #[error("IIOD context XML: {0}")]
    Xml(String),

    /// We reached an IIO device, but not one this driver drives.
    #[error(
        "{addr} is an IIO device but not a PlutoSDR: it has no `{missing}`. \
         Devices present: {present}"
    )]
    NotAPluto { addr: String, missing: &'static str, present: String },

    /// A setting the hardware cannot produce.
    #[error("{0}")]
    Unsupported(String),

    #[error("{0}")]
    Msg(String),
}

impl Error {
    /// The `-errno` the server answered with, where the failure was one of
    /// those. Lets a caller recognise a specific refusal — the AD9361's
    /// `-EOPNOTSUPP` on a gain register the part currently owns, say — without
    /// matching on the sentence built for the operator.
    pub fn remote_code(&self) -> Option<i64> {
        match self {
            Error::Remote { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Whether this is a read that stalled part-way through a payload the
    /// server had already announced.
    ///
    /// The one failure that says "this link is slower than it is being asked
    /// to be" rather than "this connection is finished": the bytes exist and
    /// the server is trying to send them, it just could not get them across in
    /// the time allowed. See `Connection::set_payload_deadline`.
    pub fn is_payload_stall(&self) -> bool {
        use std::io::ErrorKind::{Interrupted, TimedOut, WouldBlock};
        matches!(
            self,
            Error::Io { op: "read payload", source }
                if matches!(source.kind(), WouldBlock | TimedOut | Interrupted)
        )
    }

    /// Translate a failure to connect into an instruction.
    ///
    /// These three cases are nearly the whole support burden of this backend.
    /// A Pluto presents itself over USB as a *network* interface, which is the
    /// part that surprises people — so the refused/timed-out messages say so
    /// rather than talking about sockets.
    pub fn from_connect(e: std::io::Error, addr: SocketAddr) -> Error {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::ConnectionRefused => Error::Unreachable(format!(
                "nothing is listening on {addr} — the host is up but `iiod` is not \
                 running on it. On a Pluto this usually means the firmware is still \
                 booting (give it ~20 s after plugging in)"
            )),
            ErrorKind::TimedOut => Error::Unreachable(format!(
                "no answer from {addr} — a Pluto appears as a USB *network* adapter, \
                 not a serial port, so check that the interface came up and that your \
                 host has an address on its subnet (the device is 192.168.2.1, the host \
                 192.168.2.10 by default)"
            )),
            ErrorKind::NetworkUnreachable | ErrorKind::HostUnreachable => {
                Error::Unreachable(format!(
                    "no route to {addr} — the Pluto's USB network interface is not \
                     configured on this host, or the address is on a subnet you are not \
                     attached to"
                ))
            }
            _ => Error::Unreachable(format!("cannot reach {addr}: {e}")),
        }
    }

    pub(crate) fn io(op: &'static str, source: std::io::Error) -> Error {
        Error::Io { op, source }
    }
}

/// Name the `-errno` codes `iiod` actually returns, because the number alone
/// sends people looking in the wrong place. `-EBUSY` in particular is almost
/// always another program (iio-oscilloscope, GNU Radio, a second sdroxide)
/// still holding the buffer, not a broken device.
fn errno_text(code: i64) -> String {
    let n = -code;
    let name = match n {
        1 => "operation not permitted",
        2 => "no such device or attribute",
        5 => "I/O error — the device rejected the value",
        9 => "bad file descriptor (the buffer is not open)",
        11 => "resource temporarily unavailable (timeout)",
        16 => {
            "device busy — another program is holding this buffer \
               (iio-oscilloscope, GNU Radio, another sdroxide)"
        }
        19 => "no such device",
        22 => "invalid argument — the value is outside what the hardware accepts",
        25 => "not a typewriter / unsupported ioctl",
        110 => "timed out",
        _ => return format!("error {code}"),
    };
    format!("{name} (errno {n})")
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::*;

    #[test]
    fn busy_names_the_usual_culprit() {
        let e = Error::Remote { cmd: "OPEN cf-ad9361-lpc".into(), code: -16 };
        let s = e.to_string();
        assert!(s.contains("device busy"), "{s}");
        assert!(s.contains("iio-oscilloscope"), "{s}");
    }

    /// The one I/O failure worth telling apart from the rest: the server had
    /// already committed to the bytes, so the link was slow rather than dead.
    #[test]
    fn a_payload_that_ran_out_of_time_is_a_stall_and_not_a_dead_socket() {
        let stalled = Error::io("read payload", std::io::Error::from(ErrorKind::WouldBlock));
        assert!(stalled.is_payload_stall());

        // Same failure, but before the reply started: not a payload stall.
        let waiting = Error::io("read line", std::io::Error::from(ErrorKind::WouldBlock));
        assert!(!waiting.is_payload_stall());

        // Same read, but the peer hung up: the connection really is finished.
        let closed = Error::io("read payload", std::io::Error::from(ErrorKind::ConnectionReset));
        assert!(!closed.is_payload_stall());

        // And a refusal the server spelled out is not an I/O failure at all.
        assert!(!Error::Remote { cmd: "READBUF".into(), code: -11 }.is_payload_stall());
    }

    #[test]
    fn an_unknown_errno_still_reports_the_number() {
        let e = Error::Remote { cmd: "READ".into(), code: -12345 };
        assert!(e.to_string().contains("-12345"));
    }

    /// The whole point of `from_connect`: a Pluto is a network device on a USB
    /// cable, and every one of these messages has to say something the operator
    /// can go and check.
    #[test]
    fn connect_failures_explain_themselves() {
        let addr: SocketAddr = "192.168.2.1:30431".parse().unwrap();
        let refused =
            Error::from_connect(std::io::Error::from(std::io::ErrorKind::ConnectionRefused), addr);
        assert!(refused.to_string().contains("still booting"));
        let timeout = Error::from_connect(std::io::Error::from(std::io::ErrorKind::TimedOut), addr);
        assert!(timeout.to_string().contains("network"));
        assert!(timeout.to_string().contains("192.168.2.10"));
    }
}
