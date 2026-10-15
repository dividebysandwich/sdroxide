//! Radiosonde decoder via rs1729 `*mod` subprocesses (OpenWebRX style).
//!
//! The engine feeds a fixed-rate complex-float IQ lane into one or more
//! `rs41mod` / `dfm09mod` / `m10mod` / `m20mod` / `mts01mod` helpers on PATH
//! (a channel per spectral peak in the lane). This crate parses `--json`
//! lines into the sonde table the panel draws. Native-only.

mod channels;
pub mod controller;
mod json;
mod modproc;
mod package;
mod track;

pub use controller::{Rs41Action, Rs41Controller};
pub use json::parse_mod_json;
pub use package::Package;
