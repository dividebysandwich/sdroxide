//! The NET station: a port of AtChat's `client.py`.
//!
//! One `Station` per callsign. It runs the LBT+backoff channel access, the
//! dynamic master election / backup / failover, the roster, common and
//! directed chat, and block-CRC-ARQ file/image transfer with drop/reconnect
//! resume. It is generic over [`crate::channel::Connector`], so the same state
//! machine drives the virtual TCP channel and (via `AtChatEngine`) the radio.
//!
//! Ported bug fixes worth keeping: every send triggered by an incoming frame
//! is `tokio::spawn`ed rather than awaited inside the receive loop (else the
//! reply it waits for can never arrive), and `_send_blocks` pauses every few
//! blocks so chat and beacons are not starved during a bulk transfer.

mod station;
mod types;

pub use station::{Station, StationShared};
pub use types::{
    ChatScope, Role, RosterEntry, RosterStatus, StationConfig, StationEvent, StationSnapshot,
    TransferDir, TransferIn, TransferOut, TransferSnapshot,
};
