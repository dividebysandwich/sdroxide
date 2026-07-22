//! Wideband multi-signal skimmers. Today: a **CW skimmer** that decodes many
//! simultaneous Morse signals across a wide window and reports each as a
//! [`sdroxide_types::SkimmerSpot`]. Native-only (runs in the engine); the wire
//! event and UI overlay are skimmer-kind-agnostic so future skimmers plug in.

mod callsign;
mod controller;
mod cw;
mod digi;
mod morse;

pub use controller::{SkimmerAction, SkimmerController};
pub use cw::CwSkimmer;
pub use digi::DigiSkimmer;
