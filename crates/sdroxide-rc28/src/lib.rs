//! The Icom RC-28 remote encoder: a weighted tuning knob, TRANSMIT, F-1 and
//! F-2, and an LED over each plus a LINK light, on USB HID.
//!
//! [`proto`] is the wire format and builds everywhere, the browser included.
//! The rest is native: a worker thread that finds the device, reads it, keeps
//! its LEDs in step, and finds it again after a replug — the shape
//! `sdroxide-midi` has, because the app consumes both the same way. No
//! type from the HID layer escapes this crate.

pub mod proto;

#[cfg(not(target_arch = "wasm32"))]
mod worker;

#[cfg(not(target_arch = "wasm32"))]
pub use worker::{Rc28Handle, spawn};

use proto::Input;

/// Something the RC-28 did, or something that happened to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rc28Event {
    Input(Input),
    /// Found and opened. Carries the product name.
    Connected(String),
    /// Unplugged, or switched off. Whatever it was holding down has to be
    /// released by whoever acted on it.
    Disconnected,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rc28Status {
    pub connected: bool,
    pub name: String,
    /// The version string the device reports, once it has.
    pub firmware: String,
    /// Last failure to open, cleared once one succeeds.
    pub error: Option<String>,
}
