//! VDL Mode 2 — the VHF datalink airliners and ground stations exchange ACARS
//! over, on fourteen 25 kHz channels between 136.650 and 136.975 MHz.
//!
//! Native-only, and driven by the engine like the ADS-B and ISM decoders: it
//! takes complex baseband from a wideband window and hands back a message log
//! and a station table.
//!
//! # Sources
//!
//! ETSI EN 301 841-1, published by ICAO as Annex 10 Volume III Part I Chapter 6
//! — the VDL Mode 2 SARPs — for the physical and link layers, and ARINC 618 for
//! the ACARS message structure carried over them. Each module cites the part it
//! implements.
//!
//! Cross-checked against `szpajder/dumpvdl2`'s behaviour; no code is taken from
//! it.
//!
//! # Verified against a recording
//!
//! Issue #265 contributed 24 seconds of 2.4 Msps baseband centred on
//! 136.8135 MHz, and it is the only external evidence this decoder has. It
//! settled three things the standard's text alone had left wrong or unproven,
//! and later a fourth — that six of the fourteen channels it now covers were
//! missing from the plan, and that most of what the gate was opening on was the
//! *neighbouring* channel:
//! a D8PSK symbol's three bits come out **most significant first**; the data
//! field is an ordinary **HDLC frame** — `0x7E` flags and bit stuffing — whose
//! length in bits is what the transmission header states; and the
//! Reed-Solomon parameters in [`rs`] fit nothing on the air, so that layer is
//! advisory and the frame check sequence is what admits a frame. See
//! `tests/real_burst.rs`, which carries two of those transmissions.

pub mod acars;
pub mod avlc;
pub mod block;
pub mod channel;
mod controller;
pub mod demod;
pub mod gate;
pub mod hdlc;
pub mod header;
pub mod plan;
pub mod rs;
pub mod scramble;
pub mod station;
pub mod sync;
pub mod tx;
pub mod xid;

pub use controller::{Vdl2Action, Vdl2Controller};

/// Whether a stream of this rate, centred here, reaches any VDL2 channel at all.
///
/// The engine's "can this receiver do it" predicate, the same shape
/// `sdroxide_adsb::window_covers` has.
pub fn window_covers(center_hz: f64, rate_hz: f64) -> bool {
    !plan::channels_in_window(center_hz, rate_hz).is_empty()
}
