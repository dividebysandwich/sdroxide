//! The built-in **Hamlib rigctld server**: sdroxide acting as a rig for the
//! wider ham software ecosystem.
//!
//! WSJT-X, fldigi, JS8Call, N1MM, Log4OM, GPredict and CQRLOG all reach a
//! radio the same way — Hamlib's "NET rigctl" backend (rig model 2) over TCP,
//! by convention on port 4532. Speaking it directly means none of them needs a
//! separate `rigctld` process, a serial cable, or a virtual COM port pair.
//!
//! Shape mirrors the TCI server: the DSP engine owns a [`RigctldController`],
//! publishes a state snapshot on every change, and drains [`ServerRequest`]s
//! non-blocking each tick. Unlike TCI this protocol carries no audio and no IQ
//! and never pushes anything unsolicited — it is strictly request/response —
//! so there is no fan-out hub, just an acceptor and one thread per client.
//!
//! All the protocol lives in [`proto`] as pure functions over a [`RigState`],
//! because that is where every compatibility hazard is and none of it should
//! need a socket to test.
//!
//! NATIVE ONLY — it binds a TCP listener and spawns threads.

pub mod proto;
mod server;
mod state;

use sdroxide_types::Command;

pub use state::{RfGain, RigState, filter_for, from_hamlib_mode, to_hamlib_mode};

pub use server::RigctldController;

/// Something a client asked for that only the engine can carry out.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ServerRequest {
    /// Apply this command. The engine runs it through `Engine::apply`, so
    /// every safety rail and the usual state broadcast come along for free.
    Cmd(Command),
    /// The connected-client count changed.
    Clients(usize),
}
