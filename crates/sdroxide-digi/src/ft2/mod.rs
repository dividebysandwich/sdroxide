//! FT2 — the 3.75-second sibling of FT4.
//!
//! FT2 is IU8LMC's speed variant of the WSJT family, first on the air in
//! February 2026 and a certified ADIF 3.1.7 submode. Protocol-wise it is *FT4
//! with the symbol rate doubled* and nothing else changed: same LDPC(174,91),
//! same CRC-14, same 77-bit payload, same four Costas-4 arrays at symbols
//! 0/33/66/99, same Gray map, same pre-LDPC scrambling vector, same 87 data +
//! 16 sync + 2 ramp frame. Only `NSPS` moves, 576 → 288, and everything
//! derived from it follows:
//!
//! | | FT4 | FT2 |
//! |---|---|---|
//! | Samples/symbol @ 12 kHz | 576 | **288** |
//! | Keying rate | 20.833 baud | **41.667 baud** |
//! | Tone spacing | 20.833 Hz | **41.667 Hz** |
//! | Occupied bandwidth | 83 Hz | **167 Hz** |
//! | Burst (105 symbols) | 5.04 s | **2.52 s** |
//! | T/R period | 7.5 s | **3.75 s** |
//!
//! Sourced from the reference implementation (`iu8lmc/Decodium-3.0-Shannon`,
//! `lib/ft2/ft2_params.f90` and `lib/ft2/genft2.f90`, whose only diff against
//! `lib/ft4/` is `NSPS` and its derived constants) plus `Modulator.cpp` and
//! `MainWindow::on_actionFT2_triggered` for the slot timing.
//!
//! **On the "33.3 Hz" figure:** the project's own errata of 2026-02-27 states
//! FT2's tone spacing as `hmod × baud = 0.8 × 41.67 ≈ 33.3 Hz`, which
//! contradicts the 167 Hz bandwidth quoted in the same table. The 0.8 comes
//! from `lib/ft2/ft2_gfsk_iwave.f90` — an unused simulator stub still carrying
//! FST4/MSK leftovers (binary `2·itone−1` tones and a hard-coded 160-sample
//! stride that cannot be FT2's 288). The routine that actually generates the
//! on-air waveform, `lib/ft2/gen_ft2wave.f90`, sets `hmod=1.0`, and
//! `MainWindow` sets `m_toneSpacing=12000.0/288.0`. So the spacing is the full
//! 41.667 Hz — orthogonal tones at `1/T`, exactly like FT4 — and 4 × 41.667 is
//! the 167 Hz everyone quotes. That is what this module implements.
//!
//! # Why the protocol lives here rather than in mfsk-core
//!
//! mfsk-core reserves [`ProtocolId::Ft2`] but ships no implementation. Its
//! engine is generic over `P: Protocol` and public, though, so FT2 is a ZST
//! plus trait impls here and every heavy stage — Costas sync, downsample,
//! LLR, LDPC BP/OSD, GFSK synthesis — is mfsk-core's, shared bit-for-bit with
//! FT4.

pub mod decode;
pub mod encode;

use mfsk_core::engine::{
    FrameLayout, ModulationParams, Protocol, ProtocolId, SpectrumWindow, SyncBlock, SyncFrontEnd,
    SyncMode,
};
use mfsk_core::fec::Ldpc174_91;
use mfsk_core::msg::Wsjt77Message;

/// FT2 protocol marker: 4-GFSK at 41.667 baud, 103 symbols in a 3.75 s slot,
/// four Costas-4 arrays, LDPC(174,91), WSJT 77-bit payload.
#[derive(Copy, Clone, Debug, Default)]
pub struct Ft2;

impl ModulationParams for Ft2 {
    const NTONES: u32 = 4;
    const BITS_PER_SYMBOL: u32 = 2;
    /// `ft2_params.f90`: `NSPS=288` — 24 ms at 12 kHz. The one parameter that
    /// distinguishes FT2 from FT4.
    const NSPS: u32 = 288;
    const SYMBOL_DT: f32 = 0.024;
    /// `12000/288`. Orthogonal tone spacing (`hmod = 1`), as
    /// `gen_ft2wave.f90` generates and `MainWindow` sets.
    const TONE_SPACING_HZ: f32 = 41.666_668;
    const GRAY_MAP: &'static [u8] = &[0, 1, 3, 2];
    const GFSK_BT: f32 = 1.0;
    const GFSK_HMOD: f32 = 1.0;

    /// Same reasoning as FT4's: Nuttall is what WSJT-X does, but mfsk-core's
    /// noise-floor estimator misranks the signal bin against its sidelobes on
    /// clean synthesised frames, which is exactly what the round-trip test
    /// feeds it.
    const SPECTRUM_WINDOW: SpectrumWindow = SpectrumWindow::Rectangular;

    /// 4-symbol coherent integration on the deepest LLR variant, as FT4 —
    /// `get_ft4_bitmetrics.f90:71`, and FT2's bit metrics are the same code.
    const LLR_NSYM_MAX: u32 = 4;

    /// `genft2.f90` XORs the 77 message bits with the same `rvec` as
    /// `genft4.f90` before CRC-14 and LDPC; the two arrays are identical, so
    /// mfsk-core's is reused rather than transcribed a second time.
    const INFO_SCRAMBLE_RVEC: Option<&'static [u8]> = Some(&mfsk_core::ft4::FT4_RVEC);
}

impl FrameLayout for Ft2 {
    const N_DATA: u32 = 87;
    const N_SYNC: u32 = 16;
    const N_SYMBOLS: u32 = 103;
    /// One ramp symbol each side; `NN2 = 105`.
    const N_RAMP: u32 = 2;
    const SYNC_MODE: SyncMode = SyncMode::Block(&FT2_SYNC_BLOCKS);
    const T_SLOT_S: f32 = 3.75;
    /// `Modulator.cpp`: `if(mode=="FT2") delay_ms=100;` — FT2 keys 0.1 s into
    /// the slot, against FT4's 0.3 s and FT8's 0.5 s. This is the `dt = 0`
    /// reference, so a station transmitting on time reads DT ≈ 0.
    const TX_START_OFFSET_S: f32 = 0.1;
}

impl Protocol for Ft2 {
    type Fec = Ldpc174_91;
    type Msg = Wsjt77Message;
    /// No precomputed Δt phasors: each `df` is built when the search asks for
    /// it. mfsk-core's FT4 tables are laid out for FT4's 666.7 Hz baseband,
    /// not this one's 1333.3 Hz.
    type SyncPhasors = ();
    const ID: ProtocolId = ProtocolId::Ft2;
}

impl SyncFrontEnd for Ft2 {
    /// `ft2_params.f90`: `NFFT1 = 1152 = 4 × NSPS`, as FT4's 2304 is 4 × 576.
    const NFFT_PER_SYMBOL_FACTOR: u32 = 4;
    /// `ft2_params.f90`: `NSTEP = NSPS`. mfsk-core's FT4 uses half-symbol
    /// coarse steps; at 24 ms a half step is 12 ms, and the coherent Δt search
    /// in [`decode`] re-finds the position to a fraction of a sample anyway,
    /// so the reference's symbol-granular step is kept.
    const NSTEP_PER_SYMBOL: u32 = 1;
    /// `ft2_params.f90`: `NDOWN=9` → 1333.3 Hz baseband, double FT4's 666.7
    /// for double the tone spacing.
    const NDOWN: u32 = 9;
}

/// The four Costas-4 arrays, from `genft2.f90` — byte-identical to FT4's
/// `icos4a`..`icos4d`. mfsk-core keeps its copies private, so they are
/// transcribed here rather than borrowed.
const FT2_COSTAS_A: [u8; 4] = [0, 1, 3, 2];
const FT2_COSTAS_B: [u8; 4] = [1, 0, 2, 3];
const FT2_COSTAS_C: [u8; 4] = [2, 3, 1, 0];
const FT2_COSTAS_D: [u8; 4] = [3, 2, 0, 1];

/// `r1 + s4 + d29 + s4 + d29 + s4 + d29 + s4 + r1` — the frame comment at the
/// top of `genft2.f90`.
const FT2_SYNC_BLOCKS: [SyncBlock; 4] = [
    SyncBlock { start_symbol: 0, pattern: &FT2_COSTAS_A },
    SyncBlock { start_symbol: 33, pattern: &FT2_COSTAS_B },
    SyncBlock { start_symbol: 66, pattern: &FT2_COSTAS_C },
    SyncBlock { start_symbol: 99, pattern: &FT2_COSTAS_D },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference's own derived constants, checked against ours.
    #[test]
    fn matches_ft2_params_f90() {
        // NZ2 = NSPS * NN2 = 288 * 105 = 30240 samples = 2.52 s.
        let nn2 = Ft2::N_SYMBOLS + Ft2::N_RAMP;
        assert_eq!(nn2, 105);
        assert_eq!(Ft2::NSPS * nn2, 30_240);
        assert!(((Ft2::NSPS * nn2) as f32 / 12_000.0 - 2.52).abs() < 1e-4);
        // The burst plus its start offset has to fit the slot.
        let burst_s = (Ft2::NSPS * nn2) as f32 / 12_000.0;
        assert!(Ft2::TX_START_OFFSET_S + burst_s < Ft2::T_SLOT_S);
        // Tone spacing is the keying rate: orthogonal 4-FSK.
        assert!((Ft2::TONE_SPACING_HZ - 12_000.0 / Ft2::NSPS as f32).abs() < 1e-3);
        // ...and four of them is the 167 Hz everyone quotes.
        assert!((Ft2::TONE_SPACING_HZ * 4.0 - 166.67).abs() < 0.01);
        assert_eq!(Ft2::N_DATA + Ft2::N_SYNC, Ft2::N_SYMBOLS);
        // 16 slots to the minute, so the slot grid closes on UTC minutes.
        assert!((60.0 / Ft2::T_SLOT_S).fract() < 1e-6);
    }

    /// Everything except the symbol rate has to stay FT4's, or we are not
    /// talking to Decodium.
    #[test]
    fn differs_from_ft4_only_in_symbol_rate() {
        use mfsk_core::Ft4;
        assert_eq!(Ft2::NSPS * 2, Ft4::NSPS);
        assert_eq!(Ft2::NTONES, Ft4::NTONES);
        assert_eq!(Ft2::GRAY_MAP, Ft4::GRAY_MAP);
        assert_eq!(Ft2::GFSK_BT, Ft4::GFSK_BT);
        assert_eq!(Ft2::GFSK_HMOD, Ft4::GFSK_HMOD);
        assert_eq!(Ft2::N_DATA, Ft4::N_DATA);
        assert_eq!(Ft2::N_SYNC, Ft4::N_SYNC);
        assert_eq!(Ft2::N_SYMBOLS, Ft4::N_SYMBOLS);
        assert_eq!(Ft2::N_RAMP, Ft4::N_RAMP);
        assert_eq!(Ft2::LLR_NSYM_MAX, Ft4::LLR_NSYM_MAX);
        assert_eq!(Ft2::INFO_SCRAMBLE_RVEC, Ft4::INFO_SCRAMBLE_RVEC);
        // Same Costas arrays in the same slots.
        let (ours, theirs) = (Ft2::SYNC_MODE.blocks(), Ft4::SYNC_MODE.blocks());
        assert_eq!(ours.len(), theirs.len());
        for (a, b) in ours.iter().zip(theirs) {
            assert_eq!(a.start_symbol, b.start_symbol);
            assert_eq!(a.pattern, b.pattern);
        }
    }
}
