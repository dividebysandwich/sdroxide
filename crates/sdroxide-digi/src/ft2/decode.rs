//! FT2 decode: one 3.75 s slot of 12 kHz audio → messages.
//!
//! mfsk-core drives FT4 and FST4 through `engine::pipeline`, but that entry
//! point is `pub(crate)` and its FT4-specific behaviour is selected by
//! comparing `P::ID` against [`ProtocolId::Ft4`] at runtime — an out-of-crate
//! protocol cannot opt in, and `engine::ft4_coarse` hard-codes `NSPS = 576` so
//! it would be wrong for FT2 even if it could. What the pipeline is *made of*
//! is public and generic over `P: Protocol` though, so this module reassembles
//! the same stages with FT2's constants and FT4's calibration:
//!
//! ```text
//! build_fft_cache ─ coarse_sync ─┬─ downsample_cached ─ RMS normalise
//!                                ├─ ft4_sync_search_window  (coherent Δt/Δf)
//!                                ├─ symbol_spectra ─ sync_quality gate
//!                                ├─ LLR ladder (nsym 1 → 2 → 4 → bit-norm)
//!                                └─ LDPC BP, then OSD 2/3 → 4 on good sync
//! ```
//!
//! Every stage is mfsk-core's own code, so FT2 inherits FT4's numerics rather
//! than a second implementation of them. What is chosen here is only what the
//! pipeline would have chosen for FT4: 40 BP iterations
//! (`ft4_decode.f90:194`), the `N_SYNC`-scaled OSD escalation gates, and no
//! coarse-score gate on attempting OSD.

use mfsk_core::engine::dsp::downsample::{DownsampleCfg, build_fft_cache, downsample_cached};
use mfsk_core::engine::dsp::subtract::{GfskParams, SubtractCfg, refine_freq, subtract_tones_lpf};
use mfsk_core::engine::equalize::equalize_local;
use mfsk_core::engine::llr::{
    compute_llr_fast, compute_llr_partial, compute_snr_db, descramble_info, symbol_spectra,
    sync_quality,
};
use mfsk_core::engine::pipeline::DecodeStrictness;
use mfsk_core::engine::sync::{
    AudioSource, RxGrid, SyncCandidate, coarse_sync, fine_sync_power_per_block,
};
use mfsk_core::engine::sync2d::{freq_shift_cd0, ft4_sync_search_window};
use mfsk_core::engine::{
    FecCodec, FecOpts, FrameLayout, MessageCodec, ModulationParams, Protocol, SyncFrontEnd,
};
use num_complex::Complex;
use rayon::prelude::*;

use super::Ft2;
use crate::params::DECODE_RATE;

/// One FT2 decode: the fields of mfsk-core's own `DecodeResult`, which is
/// `#[non_exhaustive]` from 0.14 on, so a decoder outside the crate can no
/// longer build one.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// The LDPC's 91 information bits: the 77 message bits, then CRC-14.
    pub info: Box<[u8]>,
    pub freq_hz: f32,
    pub dt_sec: f32,
    pub hard_errors: u32,
    pub sync_score: f32,
    pub pass: u8,
    pub sync_cv: f32,
    pub snr_db: f32,
}

impl DecodeResult {
    /// The leading 77 message bits.
    pub fn message77(&self) -> &[u8; 77] {
        self.info[..77].try_into().expect("FT2 info holds 91 bits")
    }
}

/// 12 kHz → 1333.3 Hz baseband, wide enough for four tones 41.667 Hz apart
/// plus headroom.
///
/// `fft1_size` mirrors how FT4's is picked: the smallest highly-composite
/// length at or above one slot of audio (3.75 s × 12 kHz = 45 000). 46 080 =
/// 2^10 · 3^2 · 5 divides cleanly by `NDOWN = 9`, giving the same 5120-point
/// inverse FFT FT4 uses — half the input, half the decimation, identical
/// output length.
pub const FT2_DOWNSAMPLE: DownsampleCfg = DownsampleCfg {
    input_rate: 12_000,
    fft1_size: 46_080,
    fft2_size: 5_120,
    tone_spacing_hz: Ft2::TONE_SPACING_HZ,
    leading_pad_tones: 1.5,
    trailing_pad_tones: 1.5,
    ntones: 4,
    edge_taper_bins: 101,
};

/// Absolute Δt search window for the coherent sync search, in downsampled
/// samples (1333.3 Hz).
///
/// FT4 searches `[-344, 1012]` at 666.7 Hz — an absolute range covering
/// roughly ±1 s either side of its 0.5 s frame origin, independent of any
/// candidate's own dt guess. FT2's origin is 0.1 s (133 samples here) and its
/// symbols are half as long, so the same ±0.5 s span is taken around it:
/// tighter in seconds than FT4's, but 14 symbols either way, and a mode
/// running 16 slots to the minute needs clocks better than that regardless.
/// The cell count — and so the cost — comes out the same as FT4's.
const IB_MIN: i32 = -534;
const IB_MAX: i32 = 800;

/// FT2 has 16 sync symbols (4 × Costas-4); require at least half correct
/// before spending BP on a candidate. FT4's gate, for FT4's frame.
const SYNC_Q_MIN: u32 = 8;

/// `ft4_decode.f90:194` uses 40 BP iterations where FT8 and FST4 use 30. FT2
/// runs FT4's decoder, so it inherits FT4's budget.
const BP_MAX_ITER: u32 = 40;

/// Rounds of successive interference cancellation.
///
/// FT2 takes the same treatment WSJT-X gives FT4: decode, subtract each
/// decoded signal from the audio, and look again on what is left — which is
/// the only way a weak signal inside a stronger neighbour's occupied
/// bandwidth (167 Hz here, twice FT4's) is ever decoded. Two rounds are the
/// measured choice for FT4 and find nothing more from a third. A round that
/// decodes nothing new ends the loop: nothing was subtracted, so the next
/// round would search the same audio at a lower floor, which costs a second
/// full search in a 3.75 s slot and is not subtraction at all.
const SIC_ROUNDS: usize = 2;

/// Per-round coarse-sync floor multiplier, as mfsk-core's FT4 SIC uses: a
/// later round looks deeper into the noise on a residual the strong signals
/// have been removed from.
const SIC_SYNC_FACTORS: [f32; 3] = [1.0, 0.75, 0.5];

/// LPF half-length for the channel-tracking subtract — `subtractft4.f90`'s
/// `NFILT=1400`. FT2's frame is FT4's at half the symbol length, so the same
/// tracking bandwidth applies.
const SIC_LPF_HALF: usize = 700;

/// FT2 subtract configuration: 24 ms symbols, frame origin at 0.1 s, GFSK
/// shaping exactly as [`crate::ft2::encode`] synthesizes it.
const FT2_SUBTRACT: SubtractCfg = SubtractCfg {
    sample_rate: 12_000.0,
    tone_spacing_hz: Ft2::TONE_SPACING_HZ,
    samples_per_symbol: Ft2::NSPS as usize,
    base_offset_s: Ft2::TX_START_OFFSET_S,
    gfsk: Some(GfskParams {
        bt: Ft2::GFSK_BT,
        hmod: Ft2::GFSK_HMOD,
        ramp_samples: Ft2::NSPS as usize / 8,
    }),
};

/// Decode one slot. `freq_min`/`freq_max` bound the audio search in Hz,
/// `sync_min` is the coarse-sync floor (1.0–2.0 is the usual range) and
/// `max_cand` caps how many candidates reach BP.
///
/// Multi-pass: decode, subtract each accepted signal from the residual, and
/// search again — see [`SIC_ROUNDS`]. A single pass reports only the strongest
/// signal at each audio offset, which on a busy slot is most of the band lost.
pub fn decode_slot(
    audio: &[i16],
    freq_min: f32,
    freq_max: f32,
    sync_min: f32,
    max_cand: usize,
) -> Vec<DecodeResult> {
    let mut residual = audio.to_vec();
    let mut all: Vec<DecodeResult> = Vec::new();

    for &factor in SIC_SYNC_FACTORS.iter().take(SIC_ROUNDS) {
        let candidates = coarse_sync::<Ft2>(
            AudioSource::Real(&residual),
            freq_min,
            freq_max,
            sync_min * factor,
            None,
            max_cand,
            RxGrid::real(DECODE_RATE as f32),
        );
        if candidates.is_empty() {
            break;
        }
        let fft_cache = build_fft_cache(&residual, &FT2_DOWNSAMPLE);
        let mut new: Vec<DecodeResult> =
            candidates.par_iter().filter_map(|c| process_candidate(c, &fft_cache)).collect();

        // Two candidates a few hertz apart routinely resolve to the same
        // transmission, and already-decoded transmissions reappear in a later
        // round's residual (subtraction is a model, not a perfect
        // cancellation). Keep each message once: within a round the copy with
        // the cleaner sync, across rounds the first round's.
        new.sort_by(|a, b| b.sync_score.total_cmp(&a.sync_score));
        let mut fresh: Vec<DecodeResult> = Vec::new();
        for r in new {
            if all.iter().any(|k| k.info == r.info) || fresh.iter().any(|k| k.info == r.info) {
                continue;
            }
            fresh.push(r);
        }
        if fresh.is_empty() {
            break;
        }
        for r in &fresh {
            let tones = crate::ft2::encode::message_to_tones(r.message77());
            let refined =
                refine_freq(&residual, &tones, r.freq_hz, r.dt_sec, &FT2_SUBTRACT, 1.0, 0.1);
            subtract_tones_lpf(
                &mut residual,
                &tones,
                refined,
                r.dt_sec,
                &FT2_SUBTRACT,
                SIC_LPF_HALF,
                false,
            );
        }
        all.extend(fresh);
    }

    all.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
    all
}

/// One coarse candidate through refine → LLR → BP → OSD. `None` when nothing
/// under it survives CRC-14.
fn process_candidate(cand: &SyncCandidate, fft_cache: &[Complex<f32>]) -> Option<DecodeResult> {
    let ds_rate = 12_000.0 / Ft2::NDOWN as f32;

    let mut cd0 = downsample_cached(fft_cache, cand.freq_hz, &FT2_DOWNSAMPLE);
    // Unit-RMS the baseband. `LLR_SCALE` is calibrated against unit-power
    // input; without this the magnitudes feeding `tanh(llr/2)` inside BP land
    // at the wrong scale and BP converges on wrong codewords that happen to
    // satisfy CRC-14. Matches `ft4_decode.f90:231-232`.
    let sum2: f32 = cd0.iter().map(|c| c.norm_sqr()).sum::<f32>() / cd0.len() as f32;
    if sum2 > f32::EPSILON {
        let inv = 1.0 / sum2.sqrt();
        for c in cd0.iter_mut() {
            *c *= inv;
        }
    }

    // Coherent (Δf, Δt) search over the absolute window — the candidate's own
    // dt guess is ignored, exactly as WSJT-X's FT4 sync does.
    let s2 = ft4_sync_search_window::<Ft2>(&cd0, cand, IB_MIN, IB_MAX);
    let cd0 = freq_shift_cd0(&cd0, s2.freq_hz - cand.freq_hz, ds_rate);
    let refined = SyncCandidate {
        freq_hz: s2.freq_hz,
        dt_sec: (s2.i0 as f32) / ds_rate - Ft2::TX_START_OFFSET_S,
        score: s2.score,
    };

    let cs_raw = symbol_spectra::<Ft2>(&cd0, s2.i0);
    let nsync = sync_quality::<Ft2>(&cs_raw);
    if nsync <= SYNC_Q_MIN {
        return None;
    }

    // Spread of the four Costas blocks' powers: flat on a steady signal,
    // elevated under QSB. Reported, not gated on.
    let per_block = fine_sync_power_per_block::<Ft2>(&cd0, s2.i0);
    let sync_cv = coefficient_of_variation(&per_block);

    // `EqMode::Local` is what FT4's wide-band pass uses, and FT2's frame is
    // FT4's; the sniper path is the only place mfsk-core turns it off.
    let mut cs = cs_raw;
    equalize_local::<Ft2>(&mut cs);
    decode_symbols(&cs, &refined, nsync, sync_cv)
}

/// Run the LLR ladder and the FEC over one candidate's symbol spectra.
fn decode_symbols(
    cs: &[Complex<f32>],
    refined: &SyncCandidate,
    nsync: u32,
    sync_cv: f32,
) -> Option<DecodeResult> {
    let fec = <Ft2 as Protocol>::Fec::default();
    let verify: Option<fn(&[u8]) -> bool> =
        Some(<<Ft2 as Protocol>::Msg as MessageCodec>::verify_info);
    let strictness = DecodeStrictness::Normal;

    let finish = |mut r: mfsk_core::engine::FecResult, pass: u8| -> DecodeResult {
        // SNR is measured against the codeword as transmitted, so re-encode
        // before undoing the scrambler.
        let mut cw = vec![0u8; <<Ft2 as Protocol>::Fec as FecCodec>::N];
        fec.encode(&r.info, &mut cw);
        let itone = mfsk_core::engine::tx::codeword_to_itone::<Ft2>(&cw);
        let snr_db = compute_snr_db::<Ft2>(cs, &itone);
        descramble_info::<Ft2>(&mut r.info);
        DecodeResult {
            info: r.info.into_boxed_slice(),
            freq_hz: refined.freq_hz,
            dt_sec: refined.dt_sec,
            hard_errors: r.hard_errors,
            sync_score: refined.score,
            pass,
            sync_cv,
            snr_db,
        }
    };

    let bp_opts = FecOpts {
        bp_max_iter: BP_MAX_ITER,
        osd_depth: 0,
        verify_info: verify,
        ..Default::default()
    };

    // Coherent-integration ladder, cheapest rung first: nsym = 1, 2, 4, then
    // the bit-normalised nsym=1. FT2's `CODEWORD_INTERLEAVE` is `None`, as
    // every WSJT-family protocol's is, so there is no deinterleave step.
    let mut set = compute_llr_fast::<Ft2, f32>(cs);
    if let Some(r) = fec.decode_soft(&set.llra, &bp_opts) {
        return Some(finish(r, 0));
    }
    set.llrb = compute_llr_partial::<Ft2, f32, f32>(cs, 2);
    if let Some(r) = fec.decode_soft(&set.llrb, &bp_opts) {
        return Some(finish(r, 1));
    }
    set.llrc = compute_llr_partial::<Ft2, f32, f32>(cs, Ft2::LLR_NSYM_MAX as usize);
    if let Some(r) = fec.decode_soft(&set.llrc, &bp_opts) {
        return Some(finish(r, 2));
    }
    if let Some(r) = fec.decode_soft(&set.llrd, &bp_opts) {
        return Some(finish(r, 3));
    }

    // Ordered-statistics fallback. The attempt/escalation gates scale with
    // `N_SYNC` the way `osd_escalation_gates` does for FT4 — with 16 sync
    // symbols, FT8's flat 12/18 pair would put depth 3 out of reach entirely,
    // since `nsync` tops out at 16. No coarse-score gate: WSJT-X's FT4
    // decoder has none, and mfsk-core bypasses it for FT4 for that reason.
    let osd_attempt_min = (12 * Ft2::N_SYNC + 10) / 21;
    let osd_depth3_min = (18 * Ft2::N_SYNC + 10) / 21;
    if nsync < osd_attempt_min {
        return None;
    }
    let variants = [&set.llra, &set.llrb, &set.llrc, &set.llrd];
    let depth: u8 = if nsync >= osd_depth3_min { 3 } else { 2 };
    for extra in [depth, 4] {
        if extra == 4 && nsync < osd_depth3_min {
            break;
        }
        let opts = FecOpts {
            bp_max_iter: BP_MAX_ITER,
            osd_depth: u32::from(extra),
            verify_info: verify,
            ..Default::default()
        };
        for llr in variants {
            let Some(r) = fec.decode_soft(llr, &opts) else { continue };
            // A CRC-14 pass on its own is not enough once OSD is flipping
            // bits: cap how far it was allowed to reach.
            if r.hard_errors >= strictness.osd_max_errors(extra) {
                continue;
            }
            let pass = match extra {
                4 => 13,
                3 => 5,
                _ => 4,
            };
            return Some(finish(r, pass));
        }
    }
    None
}

/// Standard deviation over mean, 0 for an empty or silent set.
fn coefficient_of_variation(xs: &[f32]) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let n = xs.len() as f32;
    let mean = xs.iter().sum::<f32>() / n;
    if mean <= f32::EPSILON {
        return 0.0;
    }
    let var = xs.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n;
    var.sqrt() / mean
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ft2::encode::{message_to_tones, tones_to_i16};
    use mfsk_core::msg::wsjt77::{pack77, unpack77};

    /// Place a synthesised burst where a real one would sit in the slot.
    fn slot_with(text0: &str, text1: &str, text2: &str, f0: f32, amp: i16) -> Vec<i16> {
        let msg = pack77(text0, text1, text2).expect("pack");
        let burst = tones_to_i16(&message_to_tones(&msg), f0, amp);
        let mut slot = vec![0i16; (Ft2::T_SLOT_S * 12_000.0) as usize];
        let at = (Ft2::TX_START_OFFSET_S * 12_000.0) as usize;
        for (i, &s) in burst.iter().enumerate() {
            if at + i < slot.len() {
                slot[at + i] = s;
            }
        }
        slot
    }

    #[test]
    fn round_trips_a_cq() {
        let slot = slot_with("CQ", "OE1ABC", "JN88", 1500.0, 20_000);
        let got = decode_slot(&slot, 200.0, 3000.0, 1.0, 100);
        assert!(!got.is_empty(), "a clean FT2 frame must decode");
        let bits = *got[0].message77();
        assert_eq!(unpack77(&bits).expect("unpack"), "CQ OE1ABC JN88");
        // Transmitted exactly on time, so DT reads ~0 and the frequency is
        // recovered to within a fraction of a tone.
        assert!(got[0].dt_sec.abs() < 0.05, "dt {}", got[0].dt_sec);
        assert!((got[0].freq_hz - 1500.0).abs() < 5.0, "f {}", got[0].freq_hz);
    }

    #[test]
    fn round_trips_an_exchange_and_a_signal_report() {
        for (a, b, c, want) in [
            // Two digits either side of the sign, WSJT-X's `i3.2`: what goes
            // in comes back unchanged. (`unpack77` used to drop the padding
            // on single-digit negatives — mfsk-core 0.10 fixed that.)
            ("OE1ABC", "IU8LMC", "-07", "OE1ABC IU8LMC -07"),
            ("IU8LMC", "OE1ABC", "RR73", "IU8LMC OE1ABC RR73"),
        ] {
            let slot = slot_with(a, b, c, 1200.0, 20_000);
            let got = decode_slot(&slot, 200.0, 3000.0, 1.0, 100);
            assert!(!got.is_empty(), "{want}");
            let bits = *got[0].message77();
            assert_eq!(unpack77(&bits).expect("unpack"), want);
        }
    }

    /// Two stations in the same slot, 400 Hz apart — the everyday case, and
    /// the one that catches a decoder that only ever reports its best
    /// candidate.
    #[test]
    fn decodes_two_signals_in_one_slot() {
        let mut slot = slot_with("CQ", "OE1ABC", "JN88", 800.0, 12_000);
        let other = slot_with("CQ", "IU8LMC", "JN70", 1200.0, 12_000);
        for (s, o) in slot.iter_mut().zip(other) {
            *s = s.saturating_add(o);
        }
        let got = decode_slot(&slot, 200.0, 3000.0, 1.0, 100);
        let texts: Vec<String> = got.iter().filter_map(|r| unpack77(r.message77())).collect();
        assert!(texts.iter().any(|t| t == "CQ OE1ABC JN88"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "CQ IU8LMC JN70"), "{texts:?}");
    }

    /// A weak FT2 signal buried under a strong one at the same audio offset
    /// must still decode. This is what the SIC rounds exist for: the strong
    /// signal's 167 Hz occupied bandwidth covers the weak one, and only
    /// subtraction of the strong decode exposes it in the residual. Without
    /// the rounds, single-pass reports the strong signal alone.
    #[test]
    fn a_masked_signal_is_recovered_by_subtraction() {
        let strong = pack77("CQ", "OE1ABC", "JN88").expect("pack");
        let weak = pack77("CQ", "IU8LMC", "JN70").expect("pack");
        let mut slot = vec![0i16; (Ft2::T_SLOT_S * 12_000.0) as usize];
        let at = (Ft2::TX_START_OFFSET_S * 12_000.0) as usize;
        for (msg, amp) in [(&strong, 18_000i16), (&weak, 1_800i16)] {
            let burst = tones_to_i16(&message_to_tones(msg), 1500.0, amp);
            for (i, &s) in burst.iter().enumerate() {
                if at + i < slot.len() {
                    slot[at + i] = slot[at + i].saturating_add(s);
                }
            }
        }
        let got = decode_slot(&slot, 200.0, 3000.0, 1.0, 100);
        let texts: Vec<String> = got.iter().filter_map(|r| unpack77(r.message77())).collect();
        assert!(texts.iter().any(|t| t == "CQ OE1ABC JN88"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "CQ IU8LMC JN70"), "weak signal lost: {texts:?}");
    }

    /// Noise alone must not produce messages; CRC-14 is the only thing
    /// standing between OSD and invented callsigns.
    #[test]
    fn silence_and_noise_decode_to_nothing() {
        let quiet = vec![0i16; (Ft2::T_SLOT_S * 12_000.0) as usize];
        assert!(decode_slot(&quiet, 200.0, 3000.0, 1.0, 100).is_empty());

        // Deterministic pseudo-noise, so a failure here is reproducible.
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let noise: Vec<i16> = (0..(Ft2::T_SLOT_S * 12_000.0) as usize)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 48) as i16) / 8
            })
            .collect();
        assert!(decode_slot(&noise, 200.0, 3000.0, 1.0, 100).is_empty());
    }
}
