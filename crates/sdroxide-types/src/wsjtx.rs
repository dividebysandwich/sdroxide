use serde::{Deserialize, Serialize};

/// WSJT-X UDP broadcast configuration (`wsjtx.json`).
///
/// This is sdroxide *being* WSJT-X for the logging ecosystem: GridTracker,
/// JTAlert, N1MM+ and Log4OM all learn about decodes and contacts from the
/// datagrams WSJT-X sends to UDP 2237. It complements [`crate::RigctldConfig`]
/// and [`crate::TciServerConfig`], which offer control surfaces — this one is
/// output only, and nothing on the socket can touch the radio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WsjtxConfig {
    /// Off by default: broadcasting where the station is and who it works is
    /// the operator's decision, even on the loopback interface.
    pub enabled: bool,
    /// Where to send. `127.0.0.1` reaches clients on this machine; a LAN
    /// address or a multicast group (`224.0.0.1`) reaches others.
    pub host: String,
    /// 2237 is the port every client defaults to.
    pub port: u16,
    /// The name clients see. Some loggers only accept traffic identifying
    /// itself as `WSJT-X`, which is why that — and not `sdroxide` — is the
    /// default.
    pub id: String,
    /// The N1MM+ contactinfo broadcast, which is a second dialect of the same
    /// idea (issue #337). Carried here rather than in a file of its own: it is
    /// the same setting — "tell my loggers what I worked" — and one page, one
    /// file and one command is what that should cost. Appended, as the wire
    /// requires.
    #[serde(default)]
    pub n1mm: N1mmConfig,
}

impl Default for WsjtxConfig {
    fn default() -> Self {
        WsjtxConfig {
            enabled: false,
            host: "127.0.0.1".into(),
            port: 2237,
            id: "WSJT-X".into(),
            n1mm: N1mmConfig::default(),
        }
    }
}

impl WsjtxConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// N1MM+ "contactinfo" UDP broadcast configuration, carried inside
/// [`WsjtxConfig`]'s file (issue #337).
///
/// A second dialect for the same purpose: a logger that does not speak WSJT-X's
/// protocol may well speak N1MM's, and the World Radio League's own desktop
/// bridge listens for both. N1MM sends one XML datagram per logged contact —
/// there is no decode stream and no status, so this is the logging half alone.
///
/// Its own destination rather than the WSJT-X one, because they are different
/// programs on different ports: 12060 is what N1MM's documentation recommends,
/// where WSJT-X's clients sit on 2237.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct N1mmConfig {
    /// Off by default, for the reason [`WsjtxConfig::enabled`] gives.
    pub enabled: bool,
    /// Where to send. N1MM's own documentation suggests `127.0.0.1` for this
    /// machine and a subnet broadcast (`192.168.1.255`) for the rest of a
    /// contest network.
    pub host: String,
    /// 12060, the port N1MM's documentation recommends.
    pub port: u16,
    /// What N1MM calls the `StationName`: the name of the computer that sent
    /// the packet. Loggers show it to tell one position of a multi-operator
    /// station from another.
    pub station: String,
}

impl Default for N1mmConfig {
    fn default() -> Self {
        N1mmConfig {
            enabled: false,
            host: "127.0.0.1".into(),
            port: 12_060,
            station: "SDROXIDE".into(),
        }
    }
}

impl N1mmConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
