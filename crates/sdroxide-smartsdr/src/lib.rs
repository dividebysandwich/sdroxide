//! Native SmartSDR (FlexRadio FLEX-6000 / FLEX-8000) support — both halves.
//!
//! NATIVE ONLY. Plain `std` TCP/UDP sockets; this crate must never be a
//! dependency of any wasm-targeted crate.
//!
//! - [`FlexHandle`] is the **client**: sdroxide operating a FlexRadio. Receive is
//!   wideband IQ over a DAX IQ stream (sdroxide demodulates); transmit is audio
//!   over a DAX TX stream (the radio modulates it). Control rides the ASCII
//!   command protocol on TCP 4992.
//! - [`discovery`] finds radios, which announce themselves on UDP 4992 without
//!   being asked.
//! - [`sim`] is a **wire-level radio simulator** — real sockets, real protocol —
//!   so the client can be developed and tested with no radio present.
//! - [`protocol`] holds the wire format all three share.
//! - [`trace`] records a session in a form a user can paste into a bug report.
//!
//! # Protocol reference
//!
//! The wire format is documented from AetherSDR's C++ implementation
//! (GPL-3.0-or-later, as is this crate) and from FlexLib's published behaviour.
//! No code is shared; the formats are. Three of AetherSDR's hard-won details are
//! reproduced here because a simulator cannot discover them:
//!
//! - the UDP registration datagram goes to the radio's *control* port and must
//!   be **repeated** until packets come back, because firmware speaking protocol
//!   v1.4.0.0 may not honour `client udpport` at all;
//! - a freshly created panadapter needs a moment before it will accept
//!   configuration, and the `daxiq_channel` binding is one of the sets that goes
//!   missing if it does not get one;
//! - a GUI client id is worth checking against the radio's live client list
//!   before registering it, because a duplicate is resolved by eviction rather
//!   than by refusal.
//!
//! # Diagnostics
//!
//! Nothing here has been run against real hardware — see the
//! [`FIELD_REPORT_HINT`], and note that the simulator agrees with the client by
//! construction, so its passing is evidence about self-consistency and nothing
//! else. [`sim::SimConfig`] therefore carries the failure modes a real radio has
//! shown (`require_udp_register`, `unbound_dax_iq`, `existing_clients`,
//! `evict_after`); a fault worth fixing is worth teaching the simulator first.
//!
//! Every connection therefore carries a [`trace::Trace`]
//! that is always on and costs nothing to enable, and
//! [`FlexHandle::trace`]`.dump()` renders it as one block of text. `tracing`
//! targets `smartsdr`, `smartsdr::control` and `smartsdr::vita` carry the same
//! material for anyone who prefers to filter it live.

pub mod discovery;
pub mod net;
pub mod protocol;
pub mod sim;
pub mod trace;
mod udp_rx;

use std::time::Duration;

pub use discovery::{FlexRadioInfo, discover, discover_default};
pub use net::{
    ConnectOptions, DEFAULT_IQ_CHANNEL, DEFAULT_NETWORK_MTU, FlexError, FlexHandle, FlexUpdate,
    IQ_RATES, RadioIdent, VITA_PORT, gui_client_id,
};
pub use protocol::RADIO_PORT;
pub use trace::Trace;

/// What to ask a user for when this backend misbehaves.
///
/// Kept here rather than in the UI so the CLI, the settings dialog and the logs
/// all say the same thing.
pub const FIELD_REPORT_HINT: &str = "This backend has not been verified against \
    real hardware. If it misbehaves, please attach the session trace \
    (Settings → Radio → Copy diagnostic report) to a bug report — it contains \
    every protocol line exchanged with the radio.";

/// Test a radio at `address`: connect, run the handshake far enough to learn its
/// identity, then disconnect. Returns a one-line summary or an error message.
///
/// Deliberately stops before GUI registration. A "Test connection" button that
/// claimed a GUI slot would evict whatever client the operator is actually using
/// on a radio without multiFLEX — a test that breaks the thing it tests.
pub fn test_connection(address: &str, timeout: Duration) -> Result<String, String> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Instant;

    let (host, port) = net::split_addr(address).map_err(|e| e.to_string())?;
    let sockaddr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host}:{port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))?;
    let stream = TcpStream::connect_timeout(&sockaddr, timeout.min(Duration::from_secs(3)))
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_millis(200))).ok();

    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);

    let deadline = Instant::now() + timeout;
    let mut version = String::new();
    let mut handle = 0u32;
    let mut model = String::new();
    let mut nickname = String::new();

    while Instant::now() < deadline {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err("the radio closed the connection".into()),
            Ok(_) => match protocol::parse_line(&line) {
                protocol::Line::Version(v) => version = v,
                protocol::Line::Handle(h) => {
                    handle = h;
                    // `sub radio all` gets the radio's identity without claiming
                    // anything of it.
                    let _ =
                        writer.write_all(protocol::build_command(1, "sub radio all").as_bytes());
                    let _ = writer.flush();
                }
                protocol::Line::Status { object, kvs, .. } if object == "radio" => {
                    model = kvs.get("model").cloned().unwrap_or_default();
                    nickname = kvs.get("nickname").cloned().unwrap_or_default();
                    return Ok(summary(&model, &nickname, &version, handle));
                }
                _ => {}
            },
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(format!("read: {e}")),
        }
    }

    // The handle is the proof it is a FlexRadio and that it accepted us. Some
    // firmware is slow to answer `sub radio all`, so a missing model is not a
    // failed test — report what we learned.
    if handle != 0 {
        return Ok(summary(&model, &nickname, &version, handle));
    }
    Err("timed out waiting for the radio's handshake".into())
}

fn summary(model: &str, nickname: &str, version: &str, handle: u32) -> String {
    let name = match (nickname.is_empty(), model.is_empty()) {
        (false, false) => format!("{nickname} ({model})"),
        (false, true) => nickname.to_string(),
        (true, false) => model.to_string(),
        (true, true) => "FlexRadio".to_string(),
    };
    let mut s = name;
    if !version.is_empty() {
        s.push_str(&format!("  v{version}"));
    }
    s.push_str(&format!("  handle 0x{handle:08X}"));
    s
}
