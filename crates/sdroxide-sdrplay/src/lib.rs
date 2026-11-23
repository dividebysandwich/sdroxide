//! Native SDRplay RSP support, through the vendor's `sdrplay_api` service.
//!
//! # Why this one is different from the other native backends
//!
//! The RTL-SDR and RX-888 backends own their hardware over USB because those
//! protocols are public. For every RSP after the original RSP1 there is no
//! open protocol — the devices are driven by SDRplay's closed userland
//! library and its background service (`sdrplay_apiService`), the same way
//! SDR++ and SDRangel drive them. So this crate speaks to that library
//! instead, and it finds it with dlopen at **runtime**: nothing is linked at
//! build time, the crate compiles everywhere, and on a machine without the
//! vendor API installed [`list`] returns nothing and [`spawn`] explains what
//! to install. The FFI surface lives in `ffi.rs`, pinned to the 3.15 ABI by
//! compile-time layout asserts.
//!
//! Receive only: 1 kHz–2 GHz, complex 16-bit IQ delivered by service
//! callbacks, effective rates 62.5 kHz–10.66 MHz (the ADC floor is 2 Msps;
//! lower rates use the service's decimator).
//!
//! An RSPduo is two receivers in one box, and [`SdrPlayDevice`] is the session
//! both share: one API handle, one thread, and a [`SdrPlayHandle`] stream per
//! tuner — combined into one span for diversity, or handed to two radios on
//! two frequencies (issues #153 and #165).
//!
//! Must never be a dependency of any wasm-targeted crate.

mod api;
pub mod auto_lna;
mod device;
mod error;
mod ffi;
mod handle;
mod linux;
mod pair;
mod stream;

pub use api::{list, resolve_serial, try_list};
pub use error::{Error, Result};
pub use handle::{DuoMode, SdrPlayDevice, SdrPlayHandle};
pub use stream::{open, spawn};
