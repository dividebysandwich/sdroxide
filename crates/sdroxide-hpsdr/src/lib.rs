//! Native OpenHPSDR (ethernet SDR) support — **Protocol 2**.
//!
//! NATIVE ONLY. Pure-Rust UDP; this crate must never be a dependency of any
//! wasm-targeted crate (mirrors the `sdroxide-cat` invariant). It is reached
//! only from the root binary and `local_controller.rs`; the settings UI talks to
//! it exclusively through the `RadioController` trait.
//!
//! Protocol 1 (Metis / Hermes-Lite 2) is intentionally out of scope for now:
//! discovery reports P1 boards but flags them unsupported. Adding P1 later is a
//! sibling framing module behind the same discovery + `HpsdrHandle` abstraction.

mod discovery;
mod net;
mod protocol1;
mod protocol2;

use std::time::Duration;

pub use discovery::{discover, probe};
pub use net::{HpsdrError, HpsdrHandle, TX_RATE_HZ};
pub use sdroxide_types::HpsdrDevice;

/// Convenience: broadcast-scan the LAN with a default 1.5 s timeout.
pub fn discover_default() -> Vec<HpsdrDevice> {
    discover(Duration::from_millis(1500))
}
