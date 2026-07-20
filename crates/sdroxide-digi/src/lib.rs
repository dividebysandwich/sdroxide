//! FT8/FT4 digital-mode engine for sdroxide.
//!
//! **Licensing:** this crate links `mfsk-core` (GPL-3.0-or-later); it is the
//! only crate in the workspace that does. It is used only in the native
//! binary — the wasm remote client links none of it (all decode/encode runs
//! server-side).

pub mod controller;
pub mod modem;
pub mod params;
pub mod qso;
pub mod scheduler;

pub use controller::{DigiAction, DigiController};
pub use modem::Ft8Modem;
pub use params::{DECODE_RATE, DigiParams};
pub use qso::QsoMachine;
pub use scheduler::SlotScheduler;
