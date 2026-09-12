//! AtCHAT NET — the live state of the multi-station keyboard/file mode, as it
//! reaches a client in [`crate::DigiStatus::atchat`].
//!
//! The mode runs its own protocol station on a background thread (roster,
//! master election, chat, block-CRC-ARQ file transfer); this is the flattened,
//! serialisable view of that station. The panel that draws it is its own "are
//! we in AtCHAT?" test — every field is empty in every other mode — the same
//! rule [`crate::DigiStatus::js8`] and [`crate::DigiStatus::aprs`] follow.

use serde::{Deserialize, Serialize};

/// One line of chat: common (broadcast) or directed (unicast).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtChatChatLine {
    /// Callsign that sent it.
    pub from: String,
    /// Callsign it was addressed to, empty for a common-channel line.
    pub dst: String,
    pub text: String,
    /// True when this station sent it.
    pub own: bool,
    /// True for a directed (private) line, false for the common channel.
    pub private: bool,
    /// Unix seconds when it was sent or received.
    pub when: u64,
}

/// One roster entry: a station heard on the channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtChatRosterEntry {
    pub call: String,
    /// `"active"` or `"lost"` — the age of the last beacon or frame heard.
    pub status: String,
    /// Seconds since that station was last heard.
    pub age_s: f64,
}

/// One file or image transfer, incoming or outgoing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtChatTransfer {
    /// Transfer id, unique within the session.
    pub id: String,
    pub filename: String,
    /// The other station: the receiver for an outgoing transfer, the sender for
    /// an incoming one.
    pub peer: String,
    /// True for a transfer coming to us, false for one we are sending.
    pub incoming: bool,
    /// Blocks received (incoming) or acknowledged (outgoing) so far.
    pub have: u32,
    /// Total blocks in the transfer.
    pub total: u32,
    pub complete: bool,
}

/// A received file that landed on disk, with the image ones flagged so the
/// panel can show them inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtChatFile {
    /// Callsign that sent it.
    pub from: String,
    pub filename: String,
    /// Absolute path it was written to.
    pub path: String,
    /// True when the name looks like an image the panel can display.
    pub is_image: bool,
    /// Unix seconds when it finished arriving.
    pub when: u64,
}

/// Live state of the AtCHAT NET station.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AtChatStatus {
    /// This station's callsign on the channel.
    pub my_call: String,
    /// True while the station is joined to a channel (virtual or radio).
    pub connected: bool,
    /// `"listener"`, `"master"` or `"backup"` — this station's role, or `None`
    /// before it has one.
    pub role: Option<String>,
    /// The callsign currently acting as net master, if one is known.
    pub master: Option<String>,
    pub roster: Vec<AtChatRosterEntry>,
    /// Chat lines, oldest first, capped.
    pub chat: Vec<AtChatChatLine>,
    pub transfers: Vec<AtChatTransfer>,
    /// Received files, newest last.
    pub files: Vec<AtChatFile>,
    /// The station's own event log, oldest first, capped.
    pub log: Vec<String>,
    /// True while this station is keying the transmitter (a frame is on the
    /// air).
    pub keyed: bool,
    /// True while a carrier from another station is being heard — the real
    /// listen-before-transmit test on the radio channel.
    pub carrier: bool,
    /// `Some("127.0.0.1:6000")` when the station is on the virtual TCP channel,
    /// `None` when it is on the radio.
    pub virtual_addr: Option<String>,
}
