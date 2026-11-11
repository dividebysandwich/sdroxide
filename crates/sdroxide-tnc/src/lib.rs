//! The TNC host interface shared by sdroxide's HF data modems.
//!
//! Mercury and Data2G both speak the de-facto standard command language of HF
//! ARQ modems: CR-terminated command lines on one TCP port
//! (`MYCALL`, `LISTEN`, `CONNECT`, `DISCONNECT`, …, answered `OK`/`WRONG` and
//! followed by asynchronous `CONNECTED`, `PTT ON`, `BUFFER n` lines), and the
//! link's payload on the next. Each modem parses that language itself; this
//! crate never interprets a command, it only decides *who* may speak to the
//! modem and carries their lines and bytes:
//!
//! - [`host`]: the TCP ports and the in-process clients (the operator's panel,
//!   the Winlink window), one host at a time.
//! - [`reply`]: a reading of the modem's reply lines, for status displays.
//! - [`modem`]: what a modem has to offer the controller that clocks it.

pub mod host;
pub mod modem;
pub mod reply;

pub use host::{Holder, HostMux, Input, LocalClient, TncHandle};
pub use modem::{TncKind, TncModem};
pub use reply::Reply;

/// The sample rate both modems run at: their audio is 8 kHz mono.
pub const RATE: f64 = 8_000.0;
