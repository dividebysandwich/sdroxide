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
pub mod sstv_controller;
pub mod text_modem;

pub use controller::{DigiAction, DigiController};
pub use sstv_controller::SstvController;
pub use modem::Ft8Modem;
pub use params::{DECODE_RATE, DigiParams};
pub use qso::QsoMachine;
pub use scheduler::SlotScheduler;
pub use text_modem::TextModemController;

use std::time::SystemTime;

use sdroxide_types::{DigiConfig, DigiStatus, Mode, SstvMode};

/// The engine-facing digital-mode seam, implemented by the slotted FT8/FT4
/// [`DigiController`] and the continuous-keyboard [`TextModemController`]. The
/// engine holds one as `Box<dyn DigiEngine>` and never branches on the mode.
///
/// Method-syntax note: the FT8 controller keeps inherent methods of the same
/// names, so its trait impl delegates with fully-qualified calls.
pub trait DigiEngine: Send {
    fn mode(&self) -> Mode;
    fn on_rx_audio(&mut self, tap: &[f32]);
    fn poll(&mut self, now: SystemTime, dial_hz: f64) -> Vec<DigiAction>;
    fn tx_burst_active(&self) -> bool;
    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool;
    fn on_burst_done(&mut self);
    fn abort(&mut self);
    fn abort_tx(&mut self);
    fn set_config(&mut self, cfg: DigiConfig);
    fn set_audio_hz(&mut self, hz: f32);
    fn audio_hz(&self) -> f32;
    fn status(&self) -> DigiStatus;

    // Actions that only some modes use; default to no-ops.
    fn call_cq(&mut self) {}
    fn start_qso(&mut self, _from: String, _grid: Option<String>, _snr: i16, _audio_hz: f32) {}
    fn stop_qso(&mut self) {}
    /// Continuous keyboard modes: replace the outgoing text buffer.
    fn set_tx_text(&mut self, _text: String) {}
    /// Continuous keyboard modes: enter/leave transmit.
    fn set_tx_active(&mut self, _on: bool) {}
    /// SSTV: select the mode (`None` = auto-detect on RX, Martin 1 on TX).
    fn set_sstv_mode(&mut self, _mode: Option<SstvMode>) {}
    /// SSTV: queue a composed image (interleaved RGB) and start transmitting.
    fn set_sstv_image(&mut self, _mode: SstvMode, _rgb: Vec<u8>, _w: u16, _h: u16) {}
}
