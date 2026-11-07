//! Decoding the EU VHF contest exchange in FT8 (issue #223).
//!
//! Every other layout in the 77-bit family reaches sdroxide through mfsk-core's
//! own FT8 decoder. This one cannot, and the reason is structural rather than a
//! setting: that decoder finishes each candidate with
//!
//! ```text
//! let (bp, pass_id) = accepted?;
//! let text = unpack77(&bp.message77)?;   // ← `None` for i3 = 5
//! ```
//!
//! and mfsk-core's `unpack77` has no `i3 = 5` arm. So a contest exchange that
//! passed CRC-14 and LDPC perfectly well is thrown away *inside* the decoder,
//! before any result reaches us — there is no message to filter and no hook to
//! catch it with. FT4 and FT2 run through a different pipeline that does not
//! unpack, which is exactly why the reporter found the exchange working there
//! and never in FT8.
//!
//! What this module is, then, is a second pass over the same slot that runs
//! mfsk-core's own FT8 stages — every one of them public, and every one of them
//! the same code the primary decode uses — and keeps **only** `i3 = 5` results.
//! That gate is what makes it safe: this pass can only ever add messages the
//! primary decode is structurally incapable of returning, so it cannot
//! duplicate, contradict or second-guess an ordinary FT8 decode.
//!
//! ```text
//! compute_spectrogram ─ coarse_sync ─┬─ downsample_cached ─ fine_refine_3stage
//!                                    ├─ fill_symbol_spectra ─ sync_quality gate
//!                                    ├─ LLR ladder (a → d → nsym 2 → nsym 3)
//!                                    └─ LDPC BP, then OSD, then the i3 = 5 gate
//! ```
//!
//! It is only run with an EU VHF contest set up, because it costs about what
//! the primary decode costs and buys nothing at all outside one. The same
//! reassemble-from-public-stages approach [`crate::ft2`] takes, for the same
//! reason: what the pipeline is *made of* is public even where the entry point
//! that assembles it is not.

use mfsk_core::engine::dsp::downsample::downsample_cached;
use mfsk_core::engine::scalar::Cmplx;
use mfsk_core::engine::tx::message_to_tones;
use mfsk_core::fec::ldpc::bp::{bp_decode, check_crc14};
use mfsk_core::fec::ldpc::osd_npre::NDEEP2_174_91;
use mfsk_core::fec::ldpc::osd174_91;
use mfsk_core::ft8::decode_block::{
    SymMask, coarse_sync, compute_spectrogram, fill_symbol_spectra,
};
use mfsk_core::ft8::downsample::{FT8_CFG, build_fft_cache};
use mfsk_core::ft8::llr::{compute_llr_fast, compute_llr_partial, compute_snr_db, sync_quality};
use mfsk_core::ft8::params::LDPC_N;
use mfsk_core::ft8::refine_fine::fine_refine_3stage;
use rayon::prelude::*;

/// WSJT-X's own BP iteration budget for FT8 (`ft8b.f90`), which is what
/// mfsk-core's decoder uses too.
const BP_MAX_ITER: u32 = 30;

/// Below this many correct sync symbols a candidate is noise. mfsk-core's own
/// gate, in the same place in the chain.
const SYNC_Q_MIN: u32 = 6;

/// One rescued message: the bits, and where in the slot they came from.
#[derive(Debug, Clone)]
pub struct EuDecode {
    pub msg77: [u8; 77],
    pub freq_hz: f32,
    pub dt_sec: f32,
    pub snr_db: f32,
}

/// The `i3` field of a 77-bit message — the three bits at 74..77 that say which
/// layout it is. `5` is the EU VHF contest exchange.
fn i3_of(msg: &[u8; 77]) -> u8 {
    (msg[74] & 1) << 2 | (msg[75] & 1) << 1 | (msg[76] & 1)
}

/// The EU VHF contest layout's `i3`.
const I3_EU_VHF: u8 = 5;

/// Decode one 15 s slot of 12 kHz audio, keeping only EU VHF contest messages.
///
/// `freq_min`/`freq_max` bound the audio search in Hz, `sync_min` is the
/// coarse-sync floor and `max_cand` caps how many candidates reach BP — the
/// same four numbers the primary decode is given, so this pass searches
/// exactly the same band.
pub fn decode_slot(
    audio: &[i16],
    freq_min: f32,
    freq_max: f32,
    sync_min: f32,
    max_cand: usize,
) -> Vec<EuDecode> {
    let spec = compute_spectrogram(audio, freq_max);
    let candidates = coarse_sync(&spec, freq_min, freq_max, sync_min, max_cand);
    drop(spec);
    if candidates.is_empty() {
        return Vec::new();
    }
    let fft_cache = build_fft_cache(audio);

    let mut out: Vec<EuDecode> =
        candidates.par_iter().filter_map(|c| decode_candidate(c, audio, &fft_cache)).collect();

    // Two candidates a few hertz apart routinely resolve to the same
    // transmission. Keep one of each: the list this feeds is a decode list, and
    // the same exchange twice reads as two stations.
    out.sort_by(|a, b| b.snr_db.total_cmp(&a.snr_db));
    let mut kept: Vec<EuDecode> = Vec::with_capacity(out.len());
    for d in out {
        if !kept.iter().any(|k| k.msg77 == d.msg77 && (k.freq_hz - d.freq_hz).abs() < 10.0) {
            kept.push(d);
        }
    }
    kept.sort_by(|a, b| a.freq_hz.total_cmp(&b.freq_hz));
    kept
}

/// One candidate through the ladder, answering `Some` only for an `i3 = 5`
/// message. Every stage here is mfsk-core's, in mfsk-core's order.
fn decode_candidate(
    cand: &mfsk_core::engine::sync::SyncCandidate,
    audio: &[i16],
    fft_cache: &[num_complex::Complex<f32>],
) -> Option<EuDecode> {
    // Fine refinement, on the downsampled baseband — the 3-stage search
    // `ft8b.f90` runs, without which a candidate one or two hertz off the real
    // carrier still correlates and still decodes to nothing.
    let cd0 = downsample_cached(fft_cache, cand.freq_hz, &FT8_CFG);
    let refine = fine_refine_3stage(&cd0, cand.dt_sec);
    drop(cd0);
    let (freq_hz, dt_sec) = (cand.freq_hz + refine.delf_hz, refine.dt_sec);

    // Sync symbols first, and the gate on them before the 58 data symbols are
    // paid for: on a busy band most candidates die here.
    let mut cs: Box<[[Cmplx<f32>; 8]; 79]> =
        vec![[Cmplx::<f32>::default(); 8]; 79].try_into().ok()?;
    fill_symbol_spectra(&mut cs, audio, freq_hz, dt_sec, SymMask::SyncOnly, Some(fft_cache));
    let q = sync_quality(&cs);
    if q <= SYNC_Q_MIN {
        return None;
    }
    fill_symbol_spectra(&mut cs, audio, freq_hz, dt_sec, SymMask::DataOnly, Some(fft_cache));

    // The LLR ladder, cheapest first. `llra` and `llrd` come out of the fast
    // pass together; the nsym=2 and nsym=3 variants are only computed if the
    // cheap ones failed, which is where most of the cost would otherwise be.
    let fast = compute_llr_fast::<f32>(&cs);
    let bp = |llr: &[f32; LDPC_N]| bp_decode(llr, None, BP_MAX_ITER, Some(check_crc14));
    let mut msg = bp(&fast.llra).or_else(|| bp(&fast.llrd)).map(|r| r.message77);
    let llrb = msg.is_none().then(|| compute_llr_partial::<f32>(&cs, 2));
    if let Some(llr) = &llrb {
        msg = msg.or_else(|| bp(llr).map(|r| r.message77));
    }
    let llrc = msg.is_none().then(|| compute_llr_partial::<f32>(&cs, 3));
    if let Some(llr) = &llrc {
        msg = msg.or_else(|| bp(llr).map(|r| r.message77));
    }
    // …and ordered statistics on what belief propagation could not carry, on a
    // candidate whose sync is good enough to be worth it. mfsk-core's own gate
    // for the same step.
    if msg.is_none() && q > 6 {
        let full = mfsk_core::ft8::llr::compute_llr::<f32>(&cs);
        for llr in [&full.llra, &full.llrb, &full.llrc, &full.llrd] {
            // Upstream's `ndeep = 2`, CRC-14 checked inside.
            if let Some(o) = osd174_91(llr, NDEEP2_174_91, None) {
                msg = Some(o.message77);
                break;
            }
        }
    }
    let msg77 = msg?;

    // The gate this whole module rests on. Everything else that decodes here
    // has already reached the operator through the primary pass, and returning
    // it a second time would be a duplicate at best and an argument at worst.
    if i3_of(&msg77) != I3_EU_VHF {
        return None;
    }
    let tones: [u8; 79] = message_to_tones::<mfsk_core::Ft8>(&msg77).try_into().ok()?;
    let snr_db = compute_snr_db(&cs, &tones);
    Some(EuDecode { msg77, freq_hz, dt_sec, snr_db })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three bits that decide the layout are the last three of the message,
    /// most significant first — the same order every packer here writes them.
    #[test]
    fn the_layout_field_is_read_from_the_last_three_bits() {
        let mut m = [0u8; 77];
        assert_eq!(i3_of(&m), 0);
        (m[74], m[75], m[76]) = (1, 0, 1);
        assert_eq!(i3_of(&m), 5);
        (m[74], m[75], m[76]) = (1, 1, 1);
        assert_eq!(i3_of(&m), 7);
        (m[74], m[75], m[76]) = (0, 0, 1);
        assert_eq!(i3_of(&m), 1);
    }
}
