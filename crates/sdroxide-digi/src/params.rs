//! Per-mode FT8/FT4 protocol timing.

use sdroxide_types::Mode;

/// mfsk-core works entirely at this sample rate.
pub const DECODE_RATE: f64 = 12_000.0;

/// Frequency search window for decode/display (Hz within the passband).
pub const AUDIO_MIN_HZ: f32 = 100.0;
pub const AUDIO_MAX_HZ: f32 = 3300.0;

#[derive(Debug, Clone, Copy)]
pub struct DigiParams {
    pub mode: Mode,
    /// Slot length in seconds (FT8 15, FT4 7.5).
    pub slot_s: f64,
    /// Transmit start offset into the slot (FT8 0, FT4 0.5).
    pub tx_offset_s: f64,
    /// Nominal on-air burst length in seconds.
    pub burst_s: f64,
    /// How far into a slot to wait before decoding (collect ~90% of the slot).
    pub decode_at_s: f64,
}

impl DigiParams {
    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Ft4 => DigiParams {
                mode,
                slot_s: 7.5,
                tx_offset_s: 0.5,
                burst_s: 4.48,
                decode_at_s: 6.0,
            },
            // FT8 (and any non-FT4 digital fallback). Symbol 0 is nominally
            // 0.5 s into the slot (matches WSJT-X / mfsk-core dt reference).
            _ => DigiParams {
                mode: Mode::Ft8,
                slot_s: 15.0,
                tx_offset_s: 0.5,
                burst_s: 12.64,
                decode_at_s: 13.5,
            },
        }
    }

    /// Samples of 12 kHz audio in one slot.
    pub fn slot_samples(&self) -> usize {
        (self.slot_s * DECODE_RATE) as usize
    }
}
