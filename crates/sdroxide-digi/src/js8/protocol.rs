//! The four JS8 speeds as `mfsk-core` protocol types.
//!
//! Every speed shares one frame — 79 symbols, 8-FSK, three 7-tone Costas blocks
//! at symbols 0/36/72 bracketing two 29-symbol data blocks — and differs only in
//! how long a symbol lasts. `SyncDims::of::<P>()` derives the whole sync and LLR
//! geometry from the constants below, so four zero-sized types are the entire
//! cost of supporting four speeds.
//!
//! Two constants here would be wrong if copied from FT8, and both are the sort
//! of mistake that leaves a mode able to talk only to itself:
//!
//! * [`JS8_GRAY_MAP`] is the **identity**. `genjs8.f90:75` writes the raw 3-bit
//!   codeword index as the tone; FT8's `genft8.f90` writes `graymap(indx)`.
//! * Normal uses a Costas set in which all three blocks are **the same array**,
//!   while the faster speeds use three **different** arrays. There are two sets,
//!   not one, and Normal's is not FT8's.

use mfsk_core::engine::dsp::downsample::DownsampleCfg;
use mfsk_core::engine::{
    FrameLayout, ModulationParams, Protocol, ProtocolId, SyncBlock, SyncFrontEnd, SyncMode,
};
use sdroxide_types::Js8Speed;

use super::message::Js8Message;

/// JS8 does not Gray-code its data symbols — the tone *is* the 3-bit codeword
/// group. Transcribed from `lib/js8/genjs8.f90:70-76`.
pub const JS8_GRAY_MAP: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// The Costas set JS8 shipped with, still used by Normal. All three blocks
/// carry the same array. `lib/js8/genjs8.f90:25-27`.
pub const COSTAS_ORIGINAL: [u8; 7] = [4, 2, 5, 6, 1, 3, 0];

/// The "three symmetrical Costas arrays" set used by Fast, Turbo and Slow.
/// `lib/js8/genjs8.f90:29-31`.
pub const COSTAS_MODIFIED: [[u8; 7]; 3] =
    [[0, 6, 2, 3, 5, 4, 1], [1, 5, 0, 2, 3, 6, 4], [2, 5, 0, 6, 4, 1, 3]];

const SYNC_ORIGINAL: [SyncBlock; 3] = [
    SyncBlock { start_symbol: 0, pattern: &COSTAS_ORIGINAL },
    SyncBlock { start_symbol: 36, pattern: &COSTAS_ORIGINAL },
    SyncBlock { start_symbol: 72, pattern: &COSTAS_ORIGINAL },
];

const SYNC_MODIFIED: [SyncBlock; 3] = [
    SyncBlock { start_symbol: 0, pattern: &COSTAS_MODIFIED[0] },
    SyncBlock { start_symbol: 36, pattern: &COSTAS_MODIFIED[1] },
    SyncBlock { start_symbol: 72, pattern: &COSTAS_MODIFIED[2] },
];

/// What the four speed types have in common beyond [`Protocol`].
pub trait Js8Proto: SyncFrontEnd {
    /// The operator-facing speed this type implements.
    const SPEED: Js8Speed;
    /// Samples per symbol after downsampling — JS8Call's `NDOWNSPS`, and the
    /// per-symbol FFT size the LLR stage uses.
    const NDOWNSPS: u32;
    /// Band-extraction geometry for the downsampler. `fft1_size / fft2_size`
    /// must equal `NDOWN`, and `fft1_size` must cover the decode window.
    const DOWNSAMPLE: DownsampleCfg;
    /// Two candidates closer than this in frequency are the same signal.
    /// JS8Call's `AZ` (`js8*_params.f90`).
    const DEDUPE_HZ: f32;
    /// Added to the computed signal-to-noise so our reports line up with
    /// JS8Call's for the same audio.
    ///
    /// Empirically fitted, not derived — the reference implementation does the
    /// same thing (`js8dec.f90:349-350` carries bare `-27.0` and `-32.0`
    /// constants). The measurement is recorded in `vendor/js8call/PROVENANCE.md`;
    /// residual disagreement is about ±3 dB, which is the resolution anyone
    /// reads an SNR at anyway.
    const SNR_OFFSET_DB: f32;
}

macro_rules! js8_speed {
    ($ty:ident, $speed:ident, $nsps:expr, $ndownsps:expr, $window:expr, $delay:expr,
     $sync:expr, $fft1:expr, $fft2:expr, $dedupe:expr, $snroff:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $ty;

        impl ModulationParams for $ty {
            const NTONES: u32 = 8;
            const BITS_PER_SYMBOL: u32 = 3;
            const NSPS: u32 = $nsps;
            const SYMBOL_DT: f32 = $nsps as f32 / 12_000.0;
            const TONE_SPACING_HZ: f32 = 12_000.0 / $nsps as f32;
            const GRAY_MAP: &'static [u8] = &JS8_GRAY_MAP;
            // JS8Call transmits unshaped CPFSK (`Modulator.cpp`); we shape ours
            // to FT8's Gaussian instead. See `super::modem` for why that is a
            // deliberate improvement rather than a mismatch.
            const GFSK_BT: f32 = 2.0;
            const GFSK_HMOD: f32 = 1.0;
        }

        impl SyncFrontEnd for $ty {
            // NFFT1 = 2 * NSPS and NSTEP = NSPS / 4, per `js8*_params.f90`.
            const NFFT_PER_SYMBOL_FACTOR: u32 = 2;
            const NSTEP_PER_SYMBOL: u32 = 4;
            const NDOWN: u32 = $nsps / $ndownsps;
        }

        impl FrameLayout for $ty {
            const N_DATA: u32 = 58;
            const N_SYNC: u32 = 21;
            const N_SYMBOLS: u32 = 79;
            const N_RAMP: u32 = 0;
            const SYNC_MODE: SyncMode = SyncMode::Block(&$sync);
            // The window a decoder examines, which is the slot period for every
            // speed except Slow — it analyses 28 s of its 30 s cycle.
            const T_SLOT_S: f32 = $window;
            const TX_START_OFFSET_S: f32 = $delay;
        }

        impl Protocol for $ty {
            // Both of the following are inert tags, for the same reason:
            // `super::decode` owns its own candidate loop and never enters
            // `mfsk_core::engine::pipeline`, the only place either is read.
            // Do not "simplify" that away without reading the note in
            // `super::decode`.
            //
            // `ProtocolId` has no JS8 variant, and it is only an FFI/registry
            // tag; entering the pipeline would apply FT8/FT4 gates to us.
            //
            // `Fec` cannot name JS8's real codec at all since mfsk-core 0.8
            // sealed `FecCodec` — see [`super::ldpc`], which carries the
            // (174,87) code as an inherent impl and is called directly. The
            // FT8 codec stands in only to satisfy the associated-type bound;
            // it shares JS8's 174-bit codeword length and differs in K
            // (91 vs 87), so it would be wrong if anything ever read it.
            type Fec = mfsk_core::fec::Ldpc174_91;
            type Msg = Js8Message;
            // No precomputed Δt phasors; every `df` is built on demand.
            type SyncPhasors = ();
            const ID: ProtocolId = ProtocolId::Ft8;
        }

        impl Js8Proto for $ty {
            const SPEED: Js8Speed = Js8Speed::$speed;
            const NDOWNSPS: u32 = $ndownsps;
            const DOWNSAMPLE: DownsampleCfg = DownsampleCfg {
                input_rate: 12_000,
                fft1_size: $fft1,
                fft2_size: $fft2,
                tone_spacing_hz: 12_000.0 / $nsps as f32,
                leading_pad_tones: 1.5,
                trailing_pad_tones: 1.5,
                ntones: 8,
                edge_taper_bins: 101,
            };
            const DEDUPE_HZ: f32 = $dedupe;
            const SNR_OFFSET_DB: f32 = $snroff;
        }
    };
}

// The `fft1_size` values are the smallest highly-composite lengths that cover
// each speed's decode window (window x 12000 samples) while dividing exactly by
// `NDOWN` — the ratio `fft1/fft2` *is* the decimation factor, so it has to land
// on the nose. `the_downsampler_geometry_is_self_consistent` pins all of it.
js8_speed!(
    Js8Normal,
    Normal,
    1920,
    32,
    15.0,
    0.5,
    SYNC_ORIGINAL,
    192_000,
    3_200,
    4.0,
    -2.5,
    "JS8A — 15 s slots, 6.25 baud, 50 Hz wide. The band convention."
);
js8_speed!(
    Js8Fast,
    Fast,
    1200,
    20,
    10.0,
    0.2,
    SYNC_MODIFIED,
    122_880,
    2_048,
    8.0,
    -6.5,
    "JS8B — 10 s slots, 10 baud, 80 Hz wide."
);
js8_speed!(
    Js8Turbo,
    Turbo,
    600,
    12,
    6.0,
    0.1,
    SYNC_MODIFIED,
    76_800,
    1_536,
    12.0,
    -10.0,
    "JS8C — 6 s slots, 20 baud, 160 Hz wide."
);
js8_speed!(
    Js8Slow,
    Slow,
    3840,
    32,
    28.0,
    0.5,
    SYNC_MODIFIED,
    345_600,
    2_880,
    2.0,
    -0.5,
    "JS8E — 30 s slots, 3.125 baud, 25 Hz wide. Analyses a 28 s window."
);

/// Run `$body` with `$p` bound to the protocol type for a runtime speed.
///
/// This is the crate's only runtime branch on speed; everything underneath is
/// monomorphised.
#[macro_export]
macro_rules! with_js8_speed {
    ($speed:expr, |$p:ident| $body:expr) => {
        match $speed {
            ::sdroxide_types::Js8Speed::Normal => {
                type $p = $crate::js8::protocol::Js8Normal;
                $body
            }
            ::sdroxide_types::Js8Speed::Fast => {
                type $p = $crate::js8::protocol::Js8Fast;
                $body
            }
            ::sdroxide_types::Js8Speed::Turbo => {
                type $p = $crate::js8::protocol::Js8Turbo;
                $body
            }
            ::sdroxide_types::Js8Speed::Slow => {
                type $p = $crate::js8::protocol::Js8Slow;
                $body
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims<P: Js8Proto>() -> (u32, u32, f32, f32) {
        (P::NSPS, P::NDOWN, P::TONE_SPACING_HZ, P::T_SLOT_S)
    }

    #[test]
    fn each_speed_matches_its_published_parameters() {
        assert_eq!(dims::<Js8Normal>(), (1920, 60, 6.25, 15.0));
        assert_eq!(dims::<Js8Fast>(), (1200, 60, 10.0, 10.0));
        assert_eq!(dims::<Js8Turbo>(), (600, 50, 20.0, 6.0));
        assert_eq!(dims::<Js8Slow>(), (3840, 120, 3.125, 28.0));
    }

    #[test]
    fn the_downsampled_symbol_fft_holds_all_eight_tones() {
        // NSPS / NDOWN is the per-symbol FFT size. Turbo's is only 12 bins for
        // 8 tones, which is tight but is what JS8Call uses; anything smaller
        // would alias the tone bank.
        for (nsps, ndown, ndownsps) in
            [(1920u32, 60u32, 32u32), (1200, 60, 20), (600, 50, 12), (3840, 120, 32)]
        {
            assert_eq!(nsps / ndown, ndownsps);
            assert!(ndownsps > 8, "{ndownsps} bins cannot separate 8 tones");
        }
    }

    #[test]
    fn the_gray_map_is_the_identity() {
        // If this ever becomes FT8's map, JS8 stops decoding on the air while
        // every round-trip test in this crate keeps passing.
        for (i, &v) in JS8_GRAY_MAP.iter().enumerate() {
            assert_eq!(i as u8, v);
        }
    }

    #[test]
    fn normal_repeats_one_costas_array_and_the_rest_use_three() {
        let orig = SyncMode::Block(&SYNC_ORIGINAL).blocks();
        assert_eq!(orig[0].pattern, orig[1].pattern);
        assert_eq!(orig[1].pattern, orig[2].pattern);

        let modif = SyncMode::Block(&SYNC_MODIFIED).blocks();
        assert_ne!(modif[0].pattern, modif[1].pattern);
        assert_ne!(modif[1].pattern, modif[2].pattern);
        assert_ne!(modif[0].pattern, modif[2].pattern);

        // And the two sets are genuinely different, so a copy-paste that used
        // one everywhere would be caught here.
        assert_ne!(orig[0].pattern, modif[0].pattern);
    }

    #[test]
    fn the_costas_arrays_are_permutations_of_the_tone_alphabet() {
        let mut all: Vec<&[u8]> = vec![&COSTAS_ORIGINAL];
        all.extend(COSTAS_MODIFIED.iter().map(|a| a.as_slice()));
        for pattern in all {
            let mut seen = pattern.to_vec();
            seen.sort_unstable();
            assert_eq!(seen, (0..7).collect::<Vec<u8>>(), "{pattern:?} is not a Costas array");
        }
    }

    #[test]
    fn sync_blocks_sit_at_zero_thirtysix_and_seventytwo() {
        for blocks in [&SYNC_ORIGINAL, &SYNC_MODIFIED] {
            let starts: Vec<u32> = blocks.iter().map(|b| b.start_symbol).collect();
            assert_eq!(starts, vec![0, 36, 72]);
            assert!(blocks.iter().all(|b| b.pattern.len() == 7));
        }
    }

    #[test]
    fn the_frame_adds_up() {
        assert_eq!(<Js8Normal as FrameLayout>::N_DATA + <Js8Normal as FrameLayout>::N_SYNC, 79);
        // 58 data symbols x 3 bits = the 174-bit codeword, exactly.
        assert_eq!(
            <Js8Normal as FrameLayout>::N_DATA * <Js8Normal as ModulationParams>::BITS_PER_SYMBOL,
            174
        );
    }

    #[test]
    fn the_downsampler_geometry_is_self_consistent() {
        fn check<P: Js8Proto>() {
            let cfg = P::DOWNSAMPLE;
            // The FFT-length ratio is the decimation factor; if these disagree
            // the baseband comes out at the wrong rate and nothing decodes.
            assert_eq!(cfg.fft1_size / cfg.fft2_size, P::NDOWN as usize, "ratio");
            assert_eq!(cfg.fft1_size % cfg.fft2_size, 0, "ratio is not exact");
            // And the forward FFT must cover the whole decode window.
            let window = (f64::from(P::T_SLOT_S) * 12_000.0) as usize;
            assert!(cfg.fft1_size >= window, "fft1 {} < window {window}", cfg.fft1_size);
            assert_eq!(cfg.tone_spacing_hz, P::TONE_SPACING_HZ);
        }
        check::<Js8Normal>();
        check::<Js8Fast>();
        check::<Js8Turbo>();
        check::<Js8Slow>();
    }

    #[test]
    fn the_speed_macro_dispatches_to_the_matching_type() {
        for speed in Js8Speed::ALL {
            let nsps = with_js8_speed!(speed, |P| <P as ModulationParams>::NSPS);
            let expect = match speed {
                Js8Speed::Normal => 1920,
                Js8Speed::Fast => 1200,
                Js8Speed::Turbo => 600,
                Js8Speed::Slow => 3840,
            };
            assert_eq!(nsps, expect, "{}", speed.label());
        }
    }
}
