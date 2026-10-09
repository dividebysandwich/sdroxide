//! Native OpenHPSDR (ethernet SDR) support — **Protocol 1 and Protocol 2**.
//!
//! NATIVE ONLY. Pure-Rust UDP; this crate must never be a dependency of any
//! wasm-targeted crate (mirrors the `sdroxide-cat` invariant). It is reached
//! only from the root binary and `local_controller.rs`; the settings UI talks to
//! it exclusively through the `RadioController` trait.
//!
//! Discovery probes for both protocols and each board is driven by the one it
//! answers on: Protocol 1 (Metis framing — Hermes-Lite 2 and the legacy
//! Metis/Hermes boards) or Protocol 2. The two live in sibling framing modules
//! behind the same discovery + [`HpsdrBoard`] abstraction: one connection per
//! board, one [`HpsdrRx`] stream per DDC — a Protocol 2 board serves several
//! radios from one connection, each on its own independently tuned DDC, while
//! Protocol 1 carries exactly one. The transmitter (DUC) belongs to DDC 0's
//! stream.

mod discovery;
mod ioboard;
mod net;
mod protocol1;
mod protocol2;

use std::time::Duration;

pub use discovery::{discover, probe};
pub use net::{
    AutoGain, HpsdrBoard, HpsdrError, HpsdrRx, LNA_GAIN_DEFAULT_DB, LNA_GAIN_ELEMENT,
    LNA_GAIN_MAX_DB, LNA_GAIN_MIN_DB, TX_RATE_HZ, TX_RATE_HZ_P2, board_has_io_board,
    board_has_lna_gain, board_is_hermes_lite, tx_rate_for_protocol,
};
/// The Protocol 2 NCO math, exported for the wire-level tests and for
/// diagnosing a board against the spec.
pub use protocol2::{CLOCK_HZ, phase_word};
pub use sdroxide_types::HpsdrDevice;

/// Convenience: broadcast-scan the LAN with a default 1.5 s timeout.
pub fn discover_default() -> Vec<HpsdrDevice> {
    discover(Duration::from_millis(1500))
}
