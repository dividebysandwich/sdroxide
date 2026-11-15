//! The HFDL (ARINC 635) ground-network decoder: the threading wrapper and the
//! wire-type mapping around `xng-mode-hfdl`'s channel decoder. Native-only
//! (runs in the engine's audio-block loop); see [`controller`] for how it is
//! driven and [`crate::types`] for the wire types.
//!
//! The demodulation *is* xng's: a 24 kS/s complex baseband lane centred on
//! the channel (the rate the reference off-air capture was validated at),
//! with the +1440 Hz USB subcarrier handled inside `HfdlChannelDecoder`. What
//! this crate owns is the worker thread, the bounded I/Q queue, and the
//! mapping from `xng_mode_hfdl::pdu::HfdlEvent` onto
//! [`sdroxide_types::HfdlStatus`]'s rolling decode log.

pub mod aircraft;
pub mod controller;
pub mod stations;

pub use controller::HfdlController;
