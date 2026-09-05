//! The **WSJT-X UDP protocol**: sdroxide announcing its FT8/FT4 traffic the way
//! WSJT-X does, so the ecosystem built around it works unchanged.
//!
//! GridTracker, JTAlert, N1MM+ and Log4OM all listen for the same datagrams —
//! decodes, station status, and logged QSOs — on UDP port 2237. Speaking that
//! protocol means none of them needs to know sdroxide exists.
//!
//! The wire format is Qt's `QDataStream` (big-endian) behind a four-word header,
//! as defined by WSJT-X's `NetworkMessage.hpp`. All of it is built by pure
//! functions over a byte buffer in [`msg`], because UDP gives no feedback
//! whatsoever: a field of the wrong width doesn't fail, it just silently stops
//! a logger from seeing contacts. The tests are the only check there is.
//!
//! Output only. sdroxide is driven from its own UI (and from rigctld/TCI for
//! outside control), so the inbound half of the protocol — Reply, Halt Tx,
//! Free Text — is not implemented; nothing is read from the socket.
//!
//! NATIVE ONLY — it binds a UDP socket.

pub mod msg;

use std::net::{ToSocketAddrs, UdpSocket};

use sdroxide_types::{QsoRecord, WsjtxConfig};
use tracing::{debug, info};

/// A configured sender. Every method is fire-and-forget: a send error is logged
/// and dropped, because a logger that isn't running must never stall the radio.
pub struct WsjtxUdp {
    sock: UdpSocket,
    dest: std::net::SocketAddr,
    id: String,
    addr: String,
}

impl WsjtxUdp {
    /// Bind a socket and resolve the destination. Fails only on a bad address
    /// or an unusable local port.
    pub fn start(cfg: &WsjtxConfig) -> Result<Self, String> {
        let host = if cfg.host.trim().is_empty() { "127.0.0.1" } else { cfg.host.trim() };
        let dest = (host, cfg.port)
            .to_socket_addrs()
            .map_err(|e| format!("{host}:{}: {e}", cfg.port))?
            .next()
            .ok_or_else(|| format!("{host}:{}: no address", cfg.port))?;
        let sock = UdpSocket::bind(if dest.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })
            .map_err(|e| e.to_string())?;
        // Multicast groups are a supported destination in WSJT-X, and a
        // datagram sent to one needs a TTL that leaves this host.
        if dest.ip().is_multicast() {
            let _ = sock.set_multicast_ttl_v4(2);
        }
        let addr = format!("{host}:{}", cfg.port);
        info!(dest = %addr, id = %cfg.id, "WSJT-X UDP broadcast started");
        Ok(WsjtxUdp { sock, dest, id: cfg.id.clone(), addr })
    }

    /// The destination this sender was built for, so the engine can tell a
    /// config change that needs a rebuild from one that doesn't.
    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn send(&self, packet: Vec<u8>) {
        if let Err(e) = self.sock.send_to(&packet, self.dest) {
            debug!(dest = %self.addr, error = %e, "WSJT-X UDP send failed");
        }
    }

    /// "Still here" — clients use this to notice us and to time us out.
    pub fn heartbeat(&self, version: &str) {
        self.send(msg::heartbeat(&self.id, version));
    }

    /// One decoded message.
    pub fn decode(&self, d: &msg::DecodeInfo) {
        self.send(msg::decode(&self.id, d));
    }

    /// Where the station is and what it's doing.
    pub fn status(&self, s: &msg::StatusInfo) {
        self.send(msg::status(&self.id, s));
    }

    /// Clear the clients' decode windows (a fresh band or session).
    pub fn clear(&self) {
        self.send(msg::clear(&self.id));
    }

    /// A completed contact, in both the forms loggers accept: the structured
    /// message and the ADIF record. Which one a logger takes is its own choice;
    /// sending both is what WSJT-X does.
    ///
    /// The ADIF half is the bare record — the fields and `<EOR>`, nothing
    /// before them. This used to carry a whole file export, header line and
    /// all, which is a well-formed *file* and not what this message is defined
    /// to hold: a logger reading the datagram as the single record the protocol
    /// promises found a line of prose where the first tag should be
    /// (issue #341).
    pub fn qso_logged(&self, q: &QsoRecord) {
        self.send(msg::qso_logged(&self.id, q));
        self.send(msg::logged_adif(&self.id, &sdroxide_types::qso_to_adif_record(q)));
    }

    /// We're going away — clients drop us from their station lists.
    pub fn close(&self) {
        self.send(msg::close(&self.id));
    }
}
