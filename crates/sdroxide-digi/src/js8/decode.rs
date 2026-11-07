//! The JS8 candidate loop: a slot of 12 kHz audio in, decoded frames out.
//!
//! ## Why this does not call `mfsk_core::engine::pipeline`
//!
//! `pipeline::decode_frame` and `process_candidate_basic` would give us this
//! loop for free, but both branch on `P::ID` — at `pipeline.rs:184`, `:299`,
//! `:316`, `:336` and `:446` — and `ProtocolId` has no JS8 variant, so a JS8
//! type must borrow another protocol's tag and inherit its tuning. Borrowing
//! `Ft8` in particular leaves `bypass_osd_score_min = false`
//! (`pipeline.rs:316`), which gates OSD behind `cand.score >= 1.8`. That gate
//! was calibrated against FT8's *own* coarse sync, not the generic one we use;
//! `pipeline.rs:300-316` documents at length how the same gate silently
//! suppressed OSD for FT4 on real signals. JS8 would inherit the identical bug
//! with no way to opt out.
//!
//! Owning the loop costs about a hundred lines and makes every threshold ours.
//! It also leaves a clean seat for a faithful port of JS8Call's `syncjs8.f90`
//! should the coarse-sync recall gap below prove to matter.
//!
//! **If you are tempted to collapse this back onto `pipeline::decode_frame`,
//! read the paragraph above first.**
//!
//! ## The known weakness
//!
//! `core::sync::coarse_sync` carries its own warning (`sync.rs:264`): FT8
//! callers should not use it, because `ft8::decode_block::coarse_sync` has a
//! `sync8.f90`-faithful noise estimator that recovers more signals on a busy
//! band. That better version is hard-wired to one repeated Costas array
//! (`ft8/decode_block/coarse_sync.rs:18` imports `params::{COSTAS, COSTAS_POS}`)
//! and so cannot serve JS8's three distinct arrays. We use the generic one and
//! accept the gap; measuring it against JS8Call on a live band is the job of the
//! on-air milestone.

use mfsk_core::engine::dsp::downsample::{build_fft_cache, downsample_cached};
use mfsk_core::engine::dsp::subtract::{
    GfskParams, SubtractCfg, refine_freq, subtract_tones_lpf_refine_dt,
};
use mfsk_core::engine::llr::{compute_llr, symbol_spectra, sync_quality};
use mfsk_core::engine::protocol::FecOpts;
use mfsk_core::engine::sync::{AudioSource, RxGrid, SyncCandidate, coarse_sync, refine_candidate};
use mfsk_core::engine::{FrameLayout, ModulationParams, SyncFrontEnd};
use num_complex::Complex;
use sdroxide_types::Js8Speed;

use super::crc12;
use super::ldpc::Ldpc174_87;
use super::message::Js8Payload;
use super::modem::DECODE_RATE;
use super::protocol::Js8Proto;
use crate::with_js8_speed;

/// Lowest tone-0 frequency searched, in Hz.
pub const AUDIO_MIN_HZ: f32 = 100.0;
/// Highest tone-0 frequency searched. Tone 7 must still fit in the passband,
/// which for Turbo's 160 Hz footprint means leaving real headroom.
pub const AUDIO_MAX_HZ: f32 = 3000.0;

/// Minimum coarse-sync score to consider a candidate — JS8Call's `ASYNCMIN`,
/// which is 1.5 for every submode.
pub const SYNC_MIN: f32 = 1.5;

/// Candidate ceiling — JS8Call's `NMAXCAND`.
pub const MAX_CAND: usize = 300;

/// Costas symbols that must be the dominant tone before a candidate is worth
/// a full decode. JS8 has 21 sync symbols; requiring a bit over half of them
/// rejects noise cheaply without touching weak-but-real signals.
const SYNC_QUALITY_MIN: u32 = 11;

/// Baseband samples either side of the coarse position that fine sync searches.
/// Coarse sync steps a quarter-symbol, so half of that is the most it can be
/// out; the rest is margin.
const REFINE_STEPS: i32 = 8;

/// Noise bandwidth every WSJT-family SNR is quoted in, in Hz.
const REFERENCE_BW_HZ: f64 = 2500.0;

/// Signal-to-noise in a 2500 Hz reference bandwidth, from the slot spectrum.
///
/// `mfsk_core`'s `compute_snr_db` takes its noise from a tone four places away
/// from the transmitted one. That works for FT8 but saturates here: on a strong
/// clean signal the neighbouring bin holds the signal's own spectral leakage
/// rather than noise, so the estimate floors out around +3 dB where JS8Call
/// reports +20. Publishing that to PSK Reporter would mean 15 dB-pessimistic
/// spots.
///
/// The downsampled baseband is no better a source — its out-of-band bins sit in
/// the anti-alias filter's stopband, so they measure the filter, not the band.
/// So do what JS8Call's `baselinejs8.f90` does and read the noise floor off the
/// full-slot spectrum, which `build_fft_cache` has already computed:
///
/// * signal + noise, summed over the eight tones' occupied bins;
/// * the noise floor, as the *median* per-bin power in guard bands either side
///   — median rather than mean so a neighbouring JS8 signal raises it barely at
///   all, which on a crowded band is the difference between a plausible number
///   and a meaningless one;
/// * subtract the noise's share of the occupied band, then scale the floor to
///   2500 Hz.
fn snr_db<P: Js8Proto>(fft_cache: &[Complex<f32>], f0: f32) -> f32 {
    let bin_hz = DECODE_RATE / fft_cache.len() as f64;
    let spacing = f64::from(<P as ModulationParams>::TONE_SPACING_HZ);
    let occupied_hz = spacing * f64::from(<P as ModulationParams>::NTONES);

    let bin_of = |hz: f64| (hz / bin_hz).round() as usize;
    let nyquist = fft_cache.len() / 2;
    let power = |k: usize| {
        let c = fft_cache[k.min(nyquist)];
        f64::from(c.re) * f64::from(c.re) + f64::from(c.im) * f64::from(c.im)
    };

    // The occupied band, half a tone either side of the tone bank.
    let lo = bin_of(f64::from(f0) - spacing * 0.5);
    let hi = bin_of(f64::from(f0) + occupied_hz);
    if hi <= lo || hi >= nyquist {
        return -28.0;
    }
    let total: f64 = (lo..=hi).map(power).sum();
    let occupied_bins = (hi - lo + 1) as f64;

    // Guard bands: far enough out to miss this signal's skirts, near enough to
    // describe the same patch of band.
    let guard = (200.0f64).max(occupied_hz * 2.0);
    let mut floor: Vec<f64> = Vec::new();
    for (from, to) in [
        (f64::from(f0) - guard - 300.0, f64::from(f0) - guard),
        (f64::from(f0) + occupied_hz + guard, f64::from(f0) + occupied_hz + guard + 300.0),
    ] {
        if from <= 0.0 {
            continue;
        }
        let (a, b) = (bin_of(from), bin_of(to));
        if b < nyquist && b > a {
            floor.extend((a..=b).map(power));
        }
    }
    if floor.is_empty() {
        return -28.0;
    }
    floor.sort_by(f64::total_cmp);
    let per_bin = floor[floor.len() / 2];
    if per_bin <= 0.0 {
        return -28.0;
    }

    let signal = total - per_bin * occupied_bins;
    if signal <= 0.0 {
        return -28.0;
    }
    let noise_2500 = per_bin * (REFERENCE_BW_HZ / bin_hz);
    let raw = 10.0 * (signal / noise_2500).log10();
    // The ceiling only bites on synthetic noise-free audio, where the
    // floor estimate collapses and the ratio runs away; no real signal comes
    // close. JS8Call floors at -28 for the same reason at the other end.
    (raw as f32 + P::SNR_OFFSET_DB).clamp(-28.0, 40.0)
}

/// One decoded JS8 frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Js8Decode {
    /// The 72 payload bits and frame type, CRC already verified.
    pub payload: Js8Payload,
    /// Tone-0 frequency in the audio passband.
    pub audio_hz: f32,
    /// Time offset from the expected frame start, in seconds.
    pub dt_sec: f32,
    pub snr_db: f32,
    pub sync_score: f32,
    /// Bits the FEC had to flip — a rough confidence measure.
    pub hard_errors: u32,
    pub speed: Js8Speed,
}

/// How hard to work. More depth costs time and finds weaker signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Js8Depth {
    /// Belief propagation only.
    #[default]
    Bp,
    /// Belief propagation, then ordered-statistics decoding where BP fails.
    BpOsd,
}

/// Rounds of successive interference cancellation.
///
/// JS8Call subtracts each decoded frame and searches the residual again, so a
/// weak frame inside a stronger neighbour's ~50 Hz occupied bandwidth is
/// decoded there and not here unless we do the same. As in JS8Call
/// (`js8a_decode.f90`), every round searches at the same sync floor, and a
/// round that decodes nothing new ends the loop: with nothing subtracted the
/// residual is unchanged, so another round would only repeat the search.
const SIC_ROUNDS: usize = 2;

/// The two waveforms a JS8 frame arrives in, and how to cancel each.
///
/// JS8Call — nearly every JS8 station — transmits unshaped CPFSK: hard tone
/// switches, no Gaussian pulse (`genjs8refsig.f90` builds its own subtract
/// reference the same way), cancelled as `subtractjs8.f90` does with
/// `NFILT=1400` and no end correction. Our own transmitter shapes with GFSK
/// at BT 2 (see [`crate::js8::modem`]), which cancels best with FT8's
/// `NFILT=4000` and end correction. Each model leaves roughly −20 dB of the
/// other waveform behind, against −34 dB and −39 dB for its own, and a
/// residue that strong buries a 20:1 co-channel neighbour.
#[derive(Clone, Copy)]
enum Waveform {
    Cpfsk,
    Gfsk,
}

impl Waveform {
    fn subtract_cfg<P: Js8Proto>(self) -> SubtractCfg {
        let nsps = <P as ModulationParams>::NSPS as usize;
        SubtractCfg {
            sample_rate: 12_000.0,
            tone_spacing_hz: <P as ModulationParams>::TONE_SPACING_HZ,
            samples_per_symbol: nsps,
            base_offset_s: <P as FrameLayout>::TX_START_OFFSET_S,
            gfsk: match self {
                Self::Cpfsk => None,
                Self::Gfsk => Some(GfskParams {
                    bt: <P as ModulationParams>::GFSK_BT,
                    hmod: <P as ModulationParams>::GFSK_HMOD,
                    ramp_samples: nsps / 8,
                }),
            },
        }
    }

    /// LPF half-length and end correction for [`subtract_tones_lpf_refine_dt`].
    fn lpf(self) -> (usize, bool) {
        match self {
            Self::Cpfsk => (700, false),
            Self::Gfsk => (2000, true),
        }
    }
}

/// The model the carrier refine searches with. `refine_freq` takes GFSK only;
/// at this bandwidth the pulse switches tones within a sample or two and there
/// is no end ramp, so it stands for either waveform as far as a carrier
/// estimate can tell.
fn refine_cfg<P: Js8Proto>() -> SubtractCfg {
    SubtractCfg {
        gfsk: Some(GfskParams {
            bt: 100.0,
            hmod: <P as ModulationParams>::GFSK_HMOD,
            ramp_samples: 0,
        }),
        ..Waveform::Cpfsk.subtract_cfg::<P>()
    }
}

/// Cancel one decoded frame from the residual, as whichever waveform leaves
/// less behind.
///
/// The decode does not say which program sent the frame, so both models are
/// tried on a copy and the one with the lower residual energy is kept; nothing
/// else in the buffer differs between the two. Each probes a few samples of
/// `dt` either side of the decode's own, which alone is worth ~8 dB: a
/// decode's `dt` is typically a couple of milliseconds off.
fn subtract_frame<P: Js8Proto>(residual: &mut Vec<i16>, tones: &[u8], freq_hz: f32, dt_sec: f32) {
    let energy = |a: &[i16]| a.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>();
    let mut best: Option<(f64, Vec<i16>)> = None;
    for w in [Waveform::Cpfsk, Waveform::Gfsk] {
        let mut trial = residual.clone();
        let (lpf_half, endcorrection) = w.lpf();
        subtract_tones_lpf_refine_dt(
            &mut trial,
            tones,
            freq_hz,
            dt_sec,
            &w.subtract_cfg::<P>(),
            lpf_half,
            endcorrection,
        );
        let e = energy(&trial);
        if best.as_ref().is_none_or(|(b, _)| e < *b) {
            best = Some((e, trial));
        }
    }
    if let Some((_, trial)) = best {
        *residual = trial;
    }
}

/// Grid-search radius and step for the subtract's carrier refine, per speed.
///
/// A decode's carrier comes out of the fine sync already, so the refine only
/// trims what is left: a quarter of the tone spacing either side, in 1/64ths
/// of it — 33 trial passes over the frame at every speed rather than a fixed
/// ±5 Hz at 0.1 Hz, which is 101 passes over Slow's 25 s frame.
fn refine_grid<P: Js8Proto>() -> (f32, f32) {
    let spacing = <P as ModulationParams>::TONE_SPACING_HZ;
    (spacing / 4.0, spacing / 64.0)
}

/// Decode every JS8 frame in one slot of 12 kHz audio.
///
/// `audio` should start at the slot boundary; the frame is expected
/// `TX_START_OFFSET_S` into it, and anything from roughly ±2.5 s either side of
/// that is still found.
///
/// Multi-pass: decode, subtract each accepted frame from the residual, and
/// search again — see [`SIC_ROUNDS`]. A single pass reports only the strongest
/// frame at each audio offset.
pub fn decode_slot<P: Js8Proto>(audio: &[i16], depth: Js8Depth) -> Vec<Js8Decode> {
    let window = (f64::from(<P as FrameLayout>::T_SLOT_S) * DECODE_RATE) as usize;
    if audio.len() < window / 2 {
        return Vec::new();
    }

    let mut residual = audio.to_vec();
    let mut out: Vec<Js8Decode> = Vec::new();

    for _ in 0..SIC_ROUNDS {
        let candidates = coarse_sync::<P>(
            AudioSource::Real(&residual),
            AUDIO_MIN_HZ,
            AUDIO_MAX_HZ,
            SYNC_MIN,
            None,
            MAX_CAND,
            RxGrid::real(DECODE_RATE as f32),
        );
        if candidates.is_empty() {
            break;
        }
        let fft_cache = build_fft_cache(&residual, &P::DOWNSAMPLE);

        let mut fresh: Vec<Js8Decode> = Vec::new();
        for cand in &candidates {
            // Within a round, coarse sync reports one signal several times, so
            // a candidate already nearest a fresh decode is that same signal.
            if fresh.iter().any(|k| (k.audio_hz - cand.freq_hz).abs() < P::DEDUPE_HZ) {
                continue;
            }
            let Some(d) = process_candidate::<P>(cand, &fft_cache, depth) else { continue };
            // Across rounds, dedup on the payload and not on frequency: a later
            // round sees the residual of a frame already decoded, but two
            // stations can share a frequency, and that co-channel case is
            // exactly what subtraction recovers.
            if out.iter().any(|k| k.payload == d.payload) {
                continue;
            }
            fresh.push(d);
        }

        if fresh.is_empty() {
            break;
        }
        let search = refine_cfg::<P>();
        let (radius, step) = refine_grid::<P>();
        for d in &fresh {
            let tones = crate::js8::modem::frame_tones::<P>(d.payload);
            let refined =
                refine_freq(&residual, &tones, d.audio_hz, d.dt_sec, &search, radius, step);
            subtract_frame::<P>(&mut residual, &tones, refined, d.dt_sec);
        }
        out.extend(fresh);
    }

    out.sort_by(|a, b| a.audio_hz.total_cmp(&b.audio_hz));
    out
}

fn process_candidate<P: Js8Proto>(
    cand: &SyncCandidate,
    fft_cache: &[Complex<f32>],
    depth: Js8Depth,
) -> Option<Js8Decode> {
    let ds_rate = DECODE_RATE as f32 / <P as SyncFrontEnd>::NDOWN as f32;
    let tx_start = <P as FrameLayout>::TX_START_OFFSET_S;

    let mut cd0 = downsample_cached(fft_cache, cand.freq_hz, &P::DOWNSAMPLE);

    // Normalise the baseband to unit RMS. `LLR_SCALE` is calibrated against
    // unit-power input, and without this the magnitudes reaching `tanh(llr/2)`
    // inside belief propagation land at the wrong scale — which does not fail
    // loudly, it converges on wrong codewords that happen to satisfy the CRC.
    let sum2: f32 = cd0.iter().map(|c| c.norm_sqr()).sum::<f32>() / cd0.len() as f32;
    if sum2 <= f32::EPSILON {
        return None;
    }
    let inv = 1.0 / sum2.sqrt();
    for c in cd0.iter_mut() {
        *c *= inv;
    }

    let refined = refine_candidate::<P>(&cd0, cand, REFINE_STEPS);
    let i0 = ((refined.dt_sec + tx_start) * ds_rate).round() as i32;

    let cs = symbol_spectra::<P>(&cd0, i0);
    if sync_quality::<P>(&cs) < SYNC_QUALITY_MIN {
        return None;
    }

    let llr_set = compute_llr::<P, f32>(&cs);
    // WSJT-X's ladder: increasing coherent-integration depth, cheapest first.
    let variants = [&llr_set.llra, &llr_set.llrb, &llr_set.llrc, &llr_set.llrd];

    let fec = Ldpc174_87;
    let osd_depth = match depth {
        Js8Depth::Bp => 0,
        Js8Depth::BpOsd => 2,
    };

    for llr in variants {
        let opts = FecOpts {
            bp_max_iter: 30,
            osd_depth,
            // The CRC-12 is what stops a parity-valid but wrong codeword being
            // reported; without it belief propagation happily returns noise
            // that satisfies all 87 checks.
            verify_info: Some(crc12::check),
            ..FecOpts::default()
        };
        let Some(r) = fec.decode_soft(llr, &opts) else { continue };
        let mut info = [0u8; super::message::INFO_BITS];
        info.copy_from_slice(&r.info);
        let Some(payload) = Js8Payload::from_info(&info) else { continue };

        return Some(Js8Decode {
            payload,
            audio_hz: refined.freq_hz,
            dt_sec: refined.dt_sec,
            snr_db: snr_db::<P>(fft_cache, refined.freq_hz),
            sync_score: refined.score,
            hard_errors: r.hard_errors,
            speed: P::SPEED,
        });
    }
    None
}

/// [`decode_slot`] for a runtime-selected speed.
pub fn decode_slot_for(speed: Js8Speed, audio: &[i16], depth: Js8Depth) -> Vec<Js8Decode> {
    with_js8_speed!(speed, |P| decode_slot::<P>(audio, depth))
}

/// Samples of 12 kHz audio in one slot at this speed.
pub fn slot_samples(speed: Js8Speed) -> usize {
    (speed.slot_s() * DECODE_RATE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js8::modem;
    use crate::js8::protocol::{Js8Fast, Js8Normal, Js8Slow, Js8Turbo};

    /// Deterministic noise, so a failure is reproducible from the seed.
    struct Xorshift(u64);
    impl Xorshift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// Uniform in [-1, 1).
        fn noise(&mut self) -> f32 {
            (self.next() % 20_000) as f32 / 10_000.0 - 1.0
        }
    }

    /// Lay a frame into a full slot at `hz`, `dt` seconds off nominal, with
    /// noise at `noise_amp`. Returns i16 at 12 kHz, as the engine delivers it.
    fn slot(
        speed: Js8Speed,
        text: &str,
        frame_type: u8,
        hz: f32,
        dt: f32,
        amp: f32,
        noise_amp: f32,
        seed: u64,
    ) -> Vec<i16> {
        let payload = Js8Payload::from_chars(text, frame_type).expect("packs");
        let tones = modem::frame_tones_for(speed, payload);
        let frame = modem::synth_gfsk_for(speed, &tones, hz, amp);
        let n = slot_samples(speed);
        let mut audio = vec![0.0f32; n.max(frame.len() * 2)];
        let start = modem::start_offset_samples(speed) as i64 + (dt * DECODE_RATE as f32) as i64;
        let start = start.max(0) as usize;
        for (i, &s) in frame.iter().enumerate() {
            if start + i < audio.len() {
                audio[start + i] += s;
            }
        }
        let mut rng = Xorshift(seed | 1);
        audio
            .iter()
            .take(n)
            .map(|&s| ((s + rng.noise() * noise_amp).clamp(-1.0, 1.0) * 28_000.0) as i16)
            .collect()
    }

    #[test]
    fn a_clean_frame_decodes_at_every_speed() {
        for speed in Js8Speed::ALL {
            let audio = slot(speed, "KM4ACKtestin", 4, 1500.0, 0.0, 0.5, 0.0, 1);
            let got = decode_slot_for(speed, &audio, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "{}: {got:?}", speed.label());
            assert_eq!(got[0].payload.to_chars(), "KM4ACKtestin", "{}", speed.label());
            assert_eq!(got[0].payload.frame_type, 4);
            assert_eq!(got[0].speed, speed);
            assert!(
                (got[0].audio_hz - 1500.0).abs() < 2.0,
                "{}: {} Hz",
                speed.label(),
                got[0].audio_hz
            );
            assert!(got[0].dt_sec.abs() < 0.1, "{}: dt {}", speed.label(), got[0].dt_sec);
        }
    }

    #[test]
    fn frames_decode_across_the_passband() {
        for hz in [300.0, 800.0, 1500.0, 2200.0, 2700.0] {
            let audio = slot(Js8Speed::Normal, "HELLOWORLD12", 3, hz, 0.0, 0.5, 0.0, 7);
            let got = decode_slot::<Js8Normal>(&audio, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "{hz} Hz: {got:?}");
            assert_eq!(got[0].payload.to_chars(), "HELLOWORLD12");
            assert!((got[0].audio_hz - hz).abs() < 2.0, "{hz} Hz -> {}", got[0].audio_hz);
        }
    }

    #[test]
    fn a_frame_that_arrives_early_or_late_still_decodes() {
        // Clock error between stations is the normal case, not the exception.
        //
        // The range is bounded by the frame having to fit inside the slot at
        // all: Normal starts 0.5 s in and runs 12.64 s of a 15 s slot, so it
        // can arrive at most 0.5 s early before it spills into the previous
        // slot, and about 1.8 s late before it runs off the end. A station
        // further out than that is one neither decoder can help.
        for dt in [-0.4f32, -0.2, 0.2, 0.7, 1.2, 1.7] {
            let audio = slot(Js8Speed::Normal, "0123456789AB", 0, 1500.0, dt, 0.5, 0.0, 11);
            let got = decode_slot::<Js8Normal>(&audio, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "dt {dt}: {got:?}");
            assert!((got[0].dt_sec - dt).abs() < 0.1, "dt {dt} reported as {}", got[0].dt_sec);
        }
    }

    #[test]
    fn every_frame_type_survives_the_round_trip() {
        for frame_type in 0..8u8 {
            let audio =
                slot(Js8Speed::Normal, "QRZdeVK3ABC1", frame_type, 1500.0, 0.0, 0.5, 0.0, 3);
            let got = decode_slot::<Js8Normal>(&audio, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "type {frame_type}");
            assert_eq!(got[0].payload.frame_type, frame_type);
        }
    }

    #[test]
    fn several_signals_in_one_slot_all_decode() {
        // A real JS8 slot is a band, not a channel.
        let speed = Js8Speed::Normal;
        let n = slot_samples(speed);
        let mut audio = vec![0.0f32; n];
        let wanted =
            [("0123456789AB", 600.0f32), ("HELLOWORLD12", 1200.0), ("QRZdeVK3ABC1", 2000.0)];
        for (text, hz) in wanted {
            let payload = Js8Payload::from_chars(text, 3).expect("packs");
            let tones = modem::frame_tones_for(speed, payload);
            let frame = modem::synth_gfsk_for(speed, &tones, hz, 0.25);
            let start = modem::start_offset_samples(speed);
            for (i, &s) in frame.iter().enumerate() {
                audio[start + i] += s;
            }
        }
        let pcm: Vec<i16> = audio.iter().map(|&s| (s.clamp(-1.0, 1.0) * 28_000.0) as i16).collect();
        let got = decode_slot::<Js8Normal>(&pcm, Js8Depth::Bp);
        assert_eq!(got.len(), 3, "{got:?}");
        for (text, hz) in wanted {
            let found = got
                .iter()
                .find(|d| (d.audio_hz - hz).abs() < 3.0)
                .unwrap_or_else(|| panic!("nothing near {hz} Hz in {got:?}"));
            assert_eq!(found.payload.to_chars(), text);
        }
    }

    #[test]
    fn a_frame_still_decodes_under_noise() {
        // Not a sensitivity claim — just that the decoder is not relying on a
        // noise-free channel, which a synthetic round-trip would never catch.
        for noise in [0.1f32, 0.3, 0.6] {
            let audio = slot(Js8Speed::Normal, "KM4ACKtestin", 4, 1500.0, 0.0, 0.3, noise, 23);
            let got = decode_slot::<Js8Normal>(&audio, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "noise {noise}: {got:?}");
            assert_eq!(got[0].payload.to_chars(), "KM4ACKtestin");
        }
    }

    /// A weak JS8 frame 10 Hz from a strong one, well inside its ~50 Hz
    /// occupied bandwidth, must still decode. This is what the SIC rounds exist for: the strong
    /// frame's ~50 Hz occupied bandwidth covers the weak one, and only
    /// subtraction of the strong decode exposes it in the residual. Without
    /// the rounds, single-pass reports the strong frame alone — and JS8Call,
    /// which subtracts each decode, would have shown both.
    ///
    /// 20:1 in amplitude, in both waveforms JS8 is sent in: JS8Call's
    /// hard-switched CPFSK and our own GFSK. Subtracting either with the other's
    /// model leaves a residue that buries the weak frame at that ratio. (Not
    /// the very same offset: two frames sample-aligned on one carrier share
    /// their Costas arrays exactly, and the channel estimate takes the weak
    /// frame's sync away with the strong one — which no two real stations do.)
    #[test]
    fn a_masked_frame_is_recovered_by_subtraction() {
        let speed = Js8Speed::Normal;
        let strong = Js8Payload::from_chars("KM4ACKtestin", 4).expect("packs");
        let weak = Js8Payload::from_chars("HELLOWORLD12", 3).expect("packs");
        type Synth = fn(Js8Speed, &[u8], f32, f32) -> Vec<f32>;
        let synths: [(&str, Synth); 2] =
            [("CPFSK", modem::synth_cpfsk_for), ("GFSK", modem::synth_gfsk_for)];
        for (name, synth) in synths {
            let st = synth(speed, &modem::frame_tones_for(speed, strong), 1500.0, 0.5);
            let wk = synth(speed, &modem::frame_tones_for(speed, weak), 1510.0, 0.025);
            let n = slot_samples(speed);
            let start = modem::start_offset_samples(speed);
            let mut audio = vec![0.0f32; n.max(st.len() * 2)];
            for frame in [&st, &wk] {
                for (i, &s) in frame.iter().enumerate() {
                    if start + i < audio.len() {
                        audio[start + i] += s;
                    }
                }
            }
            let pcm: Vec<i16> =
                audio.iter().take(n).map(|&s| (s.clamp(-1.0, 1.0) * 28_000.0) as i16).collect();
            let got = decode_slot_for(speed, &pcm, Js8Depth::BpOsd);
            let texts: Vec<String> = got.iter().map(|d| d.payload.to_chars()).collect();
            assert!(texts.iter().any(|t| t == "KM4ACKtestin"), "{name}: {texts:?}");
            assert!(
                texts.iter().any(|t| t == "HELLOWORLD12"),
                "{name}: weak frame lost: {texts:?}"
            );
        }
    }

    #[test]
    fn noise_alone_produces_no_decodes() {
        // The one test a too-loose threshold cannot pass. A false decode here
        // is worse than a missed one: it puts a fabricated callsign on screen.
        let mut false_decodes = 0;
        for seed in 0..50u64 {
            for speed in Js8Speed::ALL {
                let mut rng = Xorshift(seed * 2_654_435_761 | 1);
                let pcm: Vec<i16> =
                    (0..slot_samples(speed)).map(|_| (rng.noise() * 8_000.0) as i16).collect();
                false_decodes += decode_slot_for(speed, &pcm, Js8Depth::Bp).len();
            }
        }
        assert_eq!(false_decodes, 0, "{false_decodes} decodes from pure noise");
    }

    #[test]
    fn noise_alone_produces_no_decodes_with_osd_either() {
        // The one that matters most. Ordered-statistics decoding re-encodes an
        // information set, so it *always* produces a structurally valid
        // codeword — it cannot fail the way belief propagation does. Only the
        // CRC and the hard-error ceiling stand between it and a fabricated
        // callsign on screen, so this has to be checked separately rather than
        // assumed to follow from the BP case.
        let mut false_decodes = 0;
        for seed in 0..50u64 {
            for speed in Js8Speed::ALL {
                let mut rng = Xorshift(seed * 2_654_435_761 | 1);
                let pcm: Vec<i16> =
                    (0..slot_samples(speed)).map(|_| (rng.noise() * 8_000.0) as i16).collect();
                false_decodes += decode_slot_for(speed, &pcm, Js8Depth::BpOsd).len();
            }
        }
        assert_eq!(false_decodes, 0, "{false_decodes} decodes invented from pure noise");
    }

    #[test]
    fn osd_never_costs_a_decode_that_belief_propagation_found() {
        // It is a fallback: it runs only where BP gave up, so it can only add.
        // It did not, once — selecting among CRC-gated candidates let an
        // impostor outrank the true codeword. See `js8::osd`.
        for speed in [Js8Speed::Normal, Js8Speed::Slow] {
            for amp in [0.5f32, 0.2, 0.1] {
                let audio = slot(speed, "KM4ACKtestin", 4, 1500.0, 0.0, amp, 0.25, 17);
                let bp = decode_slot_for(speed, &audio, Js8Depth::Bp);
                let osd = decode_slot_for(speed, &audio, Js8Depth::BpOsd);
                assert!(
                    osd.len() >= bp.len(),
                    "{} amp {amp}: osd {} < bp {}",
                    speed.label(),
                    osd.len(),
                    bp.len()
                );
                if let Some(d) = bp.first() {
                    assert_eq!(osd[0].payload, d.payload, "osd changed a good decode");
                }
            }
        }
    }

    #[test]
    fn silence_produces_no_decodes() {
        for speed in Js8Speed::ALL {
            let pcm = vec![0i16; slot_samples(speed)];
            assert!(decode_slot_for(speed, &pcm, Js8Depth::Bp).is_empty(), "{}", speed.label());
        }
    }

    #[test]
    fn a_short_buffer_is_declined_rather_than_decoded() {
        let pcm = vec![0i16; 1000];
        assert!(decode_slot::<Js8Normal>(&pcm, Js8Depth::Bp).is_empty());
    }

    #[test]
    fn the_wrong_speed_does_not_decode_a_frame() {
        // Normal and the faster speeds use different Costas sets, so a Normal
        // frame must not appear to a Fast decoder. If this ever passes, the
        // two Costas sets have been conflated.
        let audio = slot(Js8Speed::Normal, "KM4ACKtestin", 4, 1500.0, 0.0, 0.5, 0.0, 5);
        assert!(decode_slot::<Js8Fast>(&audio, Js8Depth::Bp).is_empty());
        assert!(decode_slot::<Js8Turbo>(&audio, Js8Depth::Bp).is_empty());
        assert!(decode_slot::<Js8Slow>(&audio, Js8Depth::Bp).is_empty());
    }

    #[test]
    fn cpfsk_from_js8calls_own_shaping_decodes_too() {
        // JS8Call transmits unshaped CPFSK; our receiver has to read it.
        for speed in Js8Speed::ALL {
            let payload = Js8Payload::from_chars("HELLOWORLD12", 3).expect("packs");
            let tones = modem::frame_tones_for(speed, payload);
            let frame = modem::synth_cpfsk_for(speed, &tones, 1500.0, 0.5);
            let n = slot_samples(speed);
            let mut audio = vec![0.0f32; n];
            let start = modem::start_offset_samples(speed);
            for (i, &s) in frame.iter().enumerate() {
                if start + i < n {
                    audio[start + i] = s;
                }
            }
            let pcm: Vec<i16> =
                audio.iter().map(|&s| (s.clamp(-1.0, 1.0) * 28_000.0) as i16).collect();
            let got = decode_slot_for(speed, &pcm, Js8Depth::Bp);
            assert_eq!(got.len(), 1, "{}: {got:?}", speed.label());
            assert_eq!(got[0].payload.to_chars(), "HELLOWORLD12");
        }
    }
}
