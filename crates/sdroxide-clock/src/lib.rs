//! The station's clock: what the radio says the time is, served to the
//! network, and the computer's own clock set from it.
//!
//! * [`TimeHub`] — the latest measurement from any radio, shared by the whole
//!   process, and the one place a request to set the clock goes through.
//! * [`ntp`] — the NTP server that hands the measured time on.
//! * [`os`] — reading, and with the right privilege setting, the operating
//!   system's clock; [`helper_main`] is the root helper's entry point.

mod hub;
pub mod ntp;
pub mod os;

pub use hub::{Fix, Refusal, StepOutcome, TimeHub, check_step};
pub use os::helper_main;
