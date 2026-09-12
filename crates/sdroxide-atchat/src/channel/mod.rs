//! The virtual channel: channel physics (half-duplex arbitration, AWGN,
//! multipath) plus the station-link abstraction.
//!
//! Deliberate layering: the *physics* lives here; the protocol *logic*
//! (master election, ARQ, chat) is in [`crate::protocol`]. On the real radio
//! neither this module nor its TCP server is used — `AtChatEngine` bridges the
//! `Station` straight onto sdroxide's SSB transmit/receive path. The virtual
//! channel exists for radio-less development and for the standalone test tools,
//! and its wire is `channel_server.py`-compatible.

pub mod config;
pub mod core;
pub mod link;
pub mod radio;
pub mod tcp_server;

pub use config::{ChannelConfig, apply_channel};
pub use core::{ChannelCore, ChannelEvent, ChannelSnapshot, ClientId};
pub use link::{
    Connector, InProcConnector, InProcRx, InProcTx, LinkRx, LinkTx, TcpConnector, TcpRx, TcpTx,
    b64_to_samples, samples_to_b64,
};
pub use radio::{RadioBridge, RadioConnector};
