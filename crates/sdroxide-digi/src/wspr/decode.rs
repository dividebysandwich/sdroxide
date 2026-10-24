//! The WSPR scan loop, rebuilt to keep the SNR and the drift.
//!
//! `mfsk_core::wspr::decode::decode_scan` does the right search and then throws
//! away two of its results. Its coarse stage produces a `BasebandCandidate`
//! carrying `snr_db` (wsprd-calibrated, referenced to the 2500 Hz bandwidth
//! WSPR reports in) and `drift_hz`; its `WsprResult` has neither field, so by
//! the time a decode comes back the numbers are gone. A WSPR spot without an
//! SNR is not a spot — it is the measurement the entire network exists to
//! pool — so the loop is reproduced here over the same crate's public
//! primitives, threading the originating candidate through to the result.
//!
//! It began as a faithful copy of upstream's structure and has since diverged
//! where measurement said it had to. What it does that upstream's does not, all
//! of it measured against `wsprd` on identical audio (see
//! `examples/wspr_bench.rs`):
//!
//! * **Refuses decodes the audio does not support.** The code carries no CRC
//!   and the OSD fallback answers with a valid codeword for any input, so
//!   pointed at noise it invents a callsign rather than failing. [`fit_of`] and
//!   [`is_plausible`] are what stop that, and on a synthesised band they took
//!   the invented stations from twenty-two to none.
//! * **Normalises the bit metrics** before the Fano decoder sees them, which
//!   `mfsk-core` says it needs and defers — see [`normalise`].
//! * **Retries the decode across a grid** rather than trusting the alignment
//!   with the best sync score, which at these SNRs is not reliable — see
//!   [`FREQ_RETRY_STEP_HZ`].
//! * **Computes the true sequential-decoding branch metric** instead of the
//!   linear approximation `mfsk-core` hands Fano — see [`branch_metric`].
//! * **Subtracts from the decoder's own recovered bits** rather than re-packing
//!   the message, so compound and hashed-callsign signals are cancelled too.
//!
//! The three-second front pad and the successive-interference-cancellation
//! shape are upstream's.

use mfsk_core::msg::WsprMessage;
use mfsk_core::wspr;
use rayon::prelude::*;

/// One decoded WSPR transmission, with the numbers a spot is made of.
#[derive(Clone, Debug, PartialEq)]
pub struct WsprDecode {
    pub message: WsprMessage,
    /// Frequency of tone 0 in the audio passband, Hz. The signal an operator
    /// would call "the carrier" sits 1.5 tone spacings above this; [`carrier_hz`]
    /// does that conversion, because it is the one the spot uploads.
    pub freq_hz: f32,
    /// Offset of the first symbol from the nominal slot anchor (slot start plus
    /// one second), in seconds. Negative means the signal arrived early.
    pub dt_sec: f32,
    /// Signal-to-noise in dB, referenced to 2500 Hz.
    pub snr_db: f32,
    /// Total frequency drift across the transmission, in Hz — see
    /// [`sdroxide_types::WsprSpot::drift_hz`] for why this is not a rate.
    pub drift_hz: f32,
    /// How much of the received tone energy this message accounts for — see
    /// [`fit_of`]. Near zero means the message does not explain the audio; a
    /// clean decode runs from about 0.3 at the noise floor up towards 0.8.
    pub fit: f32,
}

impl WsprDecode {
    /// The signal's centre frequency in the audio passband — what gets reported.
    ///
    /// The four tones straddle the centre, so tone 0 sits one and a half
    /// spacings below it. Upstream's coarse search and demodulator both work in
    /// the tone-0 convention and convert internally; a spot has to carry the
    /// centre, which is what every other client reports for the same signal.
    pub fn carrier_hz(&self) -> f32 {
        self.freq_hz + 1.5 * wspr::demod::TONE_SPACING_HZ
    }
}

/// Zero-padding prepended before the search, in seconds.
///
/// A transmission can start *before* the nominal anchor — a station whose clock
/// runs fast, reported by wsprd as `dt < -1.0`. The samples are not in the
/// recording and never will be, but with front padding the demodulator can
/// still align the rest of the frame and Fano recovers from a missing leading
/// symbol or two. Matches upstream's `NEGATIVE_DT_PAD_SEC`.
const NEGATIVE_DT_PAD_SEC: f32 = 3.0;

/// Frequency window either candidate list is searched over, relative to the
/// dial. The whole WSPR window and nothing else.
const FREQ_DEDUP_HZ: f32 = 5.0;
/// One symbol at 12 kHz.
const TIME_DEDUP_SAMPLES: i64 = 8192;

/// How many coarse candidates to attempt per pass. wsprd carries up to 200.
///
/// Sixteen is enough only for a band of rock-steady carriers, which is not a
/// band. A drifting signal smears its energy across neighbouring bins of the
/// coarse spectrum, so it makes a lower, broader peak than a stable one of the
/// same strength and is ranked below stations it is louder than. Measured on a
/// synthesised band of 26 beacons given ±2 Hz of drift, recall went from 101 of
/// 130 at sixteen candidates to 114 at twenty-four and then flat — the drifters
/// were being cut from the list before anything tried to decode them. Set above
/// that knee, because a real band is busier than the test one.
const MAX_CANDIDATES: usize = 32;

/// Successive-interference-cancellation passes, matching wsprd's `npasses`.
///
/// A pass that decodes nothing new stops the loop, so a quiet band costs one.
const N_PASSES: usize = 3;

/// Least of the received tone energy a decode must account for. See [`fit_of`].
///
/// Zero only rejects a message that *anti*-correlates with the audio, which is
/// nonsense by construction. Fano's own convergence is the real gate on this
/// path and it was measured not to produce a single false decode; anything
/// stricter here would start costing real weak signals instead.
const MIN_FIT: f32 = 0.0;

/// The same, for a decode that came out of the OSD fallback rather than Fano.
///
/// OSD does not converge or fail — it returns the nearest valid codeword to
/// whatever it is handed, so on noise it returns a well-formed callsign. On the
/// synthesised band above, ungated it invented 22 stations across five slots
/// while recovering two real ones; at this threshold it invents none. That the
/// two real ones go too is the trade, and it is the right way round: a spot is
/// pooled by the whole network, and a fabricated one corrupts everybody's path
/// statistics rather than merely this operator's screen.
const OSD_MIN_FIT: f32 = 0.40;

/// Spread the bit metrics are scaled to before Fano. See [`normalise`].
///
/// Measured, not derived: recall peaks here and falls off either side, because
/// what this really sets is the ratio between the metric spread and the
/// decoder's fixed bias.
const LLR_TARGET_SD: f32 = 2.8;

/// Where the metrics are cut off, in multiples of [`LLR_TARGET_SD`]. wsprd
/// clamps its soft symbols at ±127 on a 50-per-sigma scale.
const LLR_CLAMP_SD: f32 = 2.54;

/// Fixed-point scale the branch metrics are quantised to. The Fano decoder
/// works in integers, so this sets how finely it can compare two paths.
const FANO_SCALE: f32 = 50.0;

/// Floor on a single code bit's branch metric. See [`branch_metric`].
const FANO_METRIC_FLOOR: f32 = -8.0;

/// The Fano threshold step, in the same units as [`FANO_SCALE`]. WSJT-X uses
/// 3.4 metric units for this code (`jt9fano`'s `ndelta = 3.4·scale`).
const FANO_DELTA: i32 = (3.4 * FANO_SCALE) as i32;

/// Cycles per bit before the search gives up.
///
/// wsprd allows ten thousand and so did this, but wsprd runs one Fano attempt
/// per candidate where this runs a grid of them — and the budget is only ever
/// reached by attempts that are going to fail, so it sets the cost of the whole
/// decode. On a synthesised band of drifting beacons the trade runs 113 decodes
/// of 130 at two thousand, 118 at six, 119 at ten, for 39, 100 and 153 seconds
/// of five slots. Six thousand is the knee: it gives up one decode in 130
/// against an unlimited budget and takes a third off the wall clock, which
/// matters because this has to finish inside a two-minute slot on hardware
/// slower than the machine it was measured on.
const FANO_MAX_CYCLES: u64 = 6_000;

/// Drift search half-width, in Hz across the transmission.
const MAX_DRIFT_HZ: i32 = 4;

/// The thread pool the candidate attempts run on.
///
/// Deliberately not rayon's global pool, and deliberately not every core. This
/// is a burst of heavy arithmetic a few seconds long that happens once every
/// two minutes, sharing a machine with an SDR receive chain that has to make
/// its audio deadline the whole time. Taking every core for those few seconds
/// would trade a decode nobody is waiting on against a dropout everybody hears,
/// so it takes half of them and leaves the rest to the radio.
fn pool() -> &'static rayon::ThreadPool {
    static POOL: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        rayon::ThreadPoolBuilder::new()
            .num_threads((cores / 2).clamp(1, 8))
            .thread_name(|i| format!("wspr-decode-{i}"))
            .build()
            .expect("build the WSPR decode pool")
    })
}

/// Decode one slot of 12 kHz audio.
///
/// `audio` is the whole slot; `sample_rate` is 12 000 in every caller, and is a
/// parameter only because the demodulator's is.
pub fn decode_slot(audio: &[f32], sample_rate: u32) -> Vec<WsprDecode> {
    let pad = (NEGATIVE_DT_PAD_SEC * sample_rate as f32) as usize;
    let mut padded = vec![0f32; pad + audio.len()];
    padded[pad..].copy_from_slice(audio);

    // Decimate once: the coarse search and the demodulator both consume this
    // same 375 Hz baseband, so doing it per stage would cost a 1.4 M-point FFT
    // for nothing.
    let (idat, qdat) = wspr::baseband::decimate_to_baseband(&padded);

    let mut out: Vec<WsprDecode> = Vec::new();
    // Every decode so far keeps its padded-buffer alignment, so the next pass
    // can subtract it from the baseband at the right place.
    let mut decoded: Vec<(Found, usize)> = Vec::new();

    // The residual the current pass searches. Pass 1 sees the recording as it
    // arrived; every later pass sees it with the decodes so far subtracted out.
    let (mut ires, mut qres) = (idat.clone(), qdat.clone());

    // Coherent block detection over 1, 2 and 3 symbols runs in every pass. It
    // costs three Fano attempts per grid cell rather than one and is worth all
    // of it: at −30 dB on a lone beacon it takes the decode rate from two slots
    // in eight to six. Restricting it to the later passes — on the theory that
    // pass 1 holds the strong signals, which do not need it — quietly capped the
    // whole decoder at the strong signals, because a slot whose only station is
    // a weak one gives pass 1 nothing to find and so never reaches a later pass.
    for pass in 0..N_PASSES {
        let cands =
            wspr::coarse_baseband::coarse_baseband(&ires, &qres, pad, MAX_CANDIDATES, MAX_DRIFT_HZ);
        // Attempt every candidate, then fold the results in candidate order.
        //
        // Each attempt only reads the residual, so they are independent and the
        // work splits cleanly; the ordered fold afterwards is what keeps the
        // output identical to the sequential version, de-duplication and all.
        // The decoder spends four fifths of its time inside Fano and this is the
        // only way to spend it on more than one core.
        let attempts: Vec<Option<(Aligned, Hit)>> = pool().install(|| {
            cands
                .par_iter()
                .map(|c| {
                    // `reportable_drift`, not the raw coarse figure: below two
                    // hertz the coarse score cannot tell drift from no drift and
                    // returns ±1 arbitrarily, and the demodulator does not treat
                    // that as noise — it sweeps its oscillators half a hertz
                    // across the frame, a third of the tone spacing at the ends.
                    // Handing it a drift the signal never had is the single most
                    // expensive thing this loop can do to a weak frame.
                    align_and_decode(
                        &ires,
                        &qres,
                        c.start_sample,
                        c.freq_hz,
                        reportable_drift(c.drift_hz),
                    )
                })
                .collect()
        });

        let mut found_this_pass = 0usize;
        for (c, attempt) in cands.iter().zip(attempts) {
            let Some((al, hit)) = attempt else { continue };
            // The drift and SNR are read back off `c` in `from_result`.
            let r = Found {
                message: hit.message,
                freq_hz: al.tone0_hz,
                start_sample: (al.lag * 32).max(0) as usize,
                info_bits: hit.info,
            };
            // Does the message we are about to report actually explain the
            // tones that arrived? Every symbol the OSD fallback emits is a
            // valid codeword by construction — including the ones it builds
            // out of noise — so this is the only thing standing between a
            // marginal candidate and an invented callsign.
            let fit = fit_of(&al, &r.info_bits);
            // A plain callsign-and-grid message is the one layout that cannot
            // be faked cheaply. The other two spend their fifty bits
            // differently and a random vector lands on a well-formed one often
            // enough to matter — upstream puts the hashed-callsign layout at
            // roughly one in seven — so they answer to the same bar as OSD.
            let compound = !matches!(r.message, WsprMessage::Type1 { .. });
            let floor = if hit.from_osd || compound { OSD_MIN_FIT } else { MIN_FIT };
            if fit < floor {
                continue;
            }
            let refined = r.start_sample;
            let mut d = from_result(&r, c, pad, sample_rate);
            d.fit = fit;
            if !is_plausible(&d) {
                continue;
            }
            if push_unique(&mut out, d.clone(), refined.saturating_sub(pad)) {
                found_this_pass += 1;
                decoded.push((r, refined));
            }
        }
        // Nothing new to subtract means the next pass would search the same
        // residual and find the same nothing.
        if found_this_pass == 0 || pass + 1 >= N_PASSES {
            break;
        }
        // Subtract everything decoded so far and search what is left: a beacon
        // sitting under a strong neighbour's skirts is invisible until the
        // neighbour is gone, and this is the only path that reaches the weakest
        // signals in a busy window.
        //
        // The symbols come from the decoder's own recovered information bits
        // rather than from re-packing the message, so a compound or
        // hashed-callsign decode is subtracted too. Re-packing could only ever
        // rebuild the Type-1 layout, which left the other two sitting in the
        // residual masking whatever was behind them.
        ires.copy_from_slice(&idat);
        qres.copy_from_slice(&qdat);
        for (r, refined) in &decoded {
            let symbols = wspr::encode_channel_symbols(&r.info_bits);
            wspr::subtract::subtract_signal_baseband(
                &mut ires,
                &mut qres,
                r.freq_hz + 1.5 * wspr::demod::TONE_SPACING_HZ,
                (*refined as i32) / 32,
                0.0,
                &symbols,
            );
        }
    }

    out
}

/// The drift worth putting on a spot.
///
/// The coarse search scores drift by shifting each symbol's tone bin by
/// `((k−81)/81)·idrift/(2·0.732)` and *truncating* to an integer bin, as wsprd
/// does. Below two hertz across the frame that shift never reaches a whole bin,
/// so ±1 Hz and 0 Hz score identically on every symbol and which of the three
/// comes back is decided by nothing at all — the sort order among equal scores.
/// Reporting it made every stable carrier in a test band come out at −1 Hz
/// drift. What the search can actually resolve is reported; the rest is zero.
fn reportable_drift(hz: f32) -> f32 {
    if hz.abs() <= 1.0 { 0.0 } else { hz }
}

/// A candidate's refined alignment, and the per-tone amplitudes measured there.
struct Aligned {
    isqs: wspr::demod::IsQs,
    /// Tone-0 audio frequency after refinement, Hz.
    tone0_hz: f32,
    /// Baseband sample where symbol 0 starts.
    lag: i32,
}

/// One accepted decode out of [`decode_aligned`].
struct Hit {
    message: WsprMessage,
    info: [u8; 50],
    /// Whether this came from the OSD fallback rather than from Fano. OSD
    /// returns a valid codeword for *any* input, so its output is held to a
    /// stricter standard downstream.
    from_osd: bool,
}

/// The grid the decode is retried over, around the refined alignment.
///
/// The refinement below picks the alignment whose sync correlation is highest,
/// and at the SNRs this mode lives at that pick is not reliable — the sync
/// score is itself a noisy measurement, so its peak wanders off the true
/// alignment by more than the decoder will tolerate. Trusting it is what cost
/// the weakest signals: they decode perfectly at the right spot, and nothing
/// ever looked there.
///
/// So the decode itself is retried across a grid, which is a far better test
/// than the score — Fano either converges or it does not. Measured at −30 dB
/// with the alignment handed in deliberately wrong, the two axes are nothing
/// alike: a tenth of a second of timing error costs almost nothing, while
/// **0.2 Hz of frequency error halves the decode rate** and 0.3 Hz all but ends
/// it. That is a fifth of the 1.46 Hz tone spacing. So the frequency grid is
/// the fine one and the timing grid is coarse — the opposite way round from
/// wsprd's DT peak-up.
///
/// It only has to span ±0.4 Hz because [`interpolate_freq`] has already placed
/// the centre to about a tenth of a hertz. Before that it ran to ±1.0 Hz and
/// most of it was spent hunting for something the spectrum already knew.
const FREQ_RETRY_STEP_HZ: f32 = 0.2;
const FREQ_RETRY_STEPS: i32 = 2;
const LAG_RETRY_STEP: i32 = 24;
const LAG_RETRY_STEPS: i32 = 2;

/// Refine a coarse candidate's alignment and decode there.
///
/// The timing refinement is wsprd's `sync_and_demodulate` mode 0; the rest is
/// the retry grid above. Returns the alignment that decoded, so the caller can
/// score the message against the tones measured at that exact spot.
fn align_and_decode(
    idat: &[f32],
    qdat: &[f32],
    start_sample: usize,
    tone0_hz: f32,
    drift_hz: f32,
) -> Option<(Aligned, Hit)> {
    // The grid is centred on the *coarse* frequency, not on a refined one.
    // Refining it first by sync score actively hurts: at the noise floor that
    // score is noise too, so the "refinement" wanders up to a hertz away and
    // takes the grid with it, leaving the true frequency outside the search
    // entirely. The coarse bin is only 0.73 Hz wide, so its estimate is the
    // better centre — and the retry below is a better refinement than any score.
    let lag0 = align_lag(idat, qdat, start_sample, tone0_hz, drift_hz);
    let coarse_bb = tone0_hz + 1.5 * wspr::demod::TONE_SPACING_HZ - wspr::baseband::CENTER_HZ;
    let f0_bb = interpolate_freq(idat, qdat, coarse_bb, lag0, drift_hz);
    // Outward from the centre on both axes, nearest first, so a signal that
    // decodes at several cells is reported at the one closest to its measured
    // alignment.
    let outward = |n: i32| (0..=n).flat_map(|k| if k == 0 { vec![0] } else { vec![-k, k] });

    // Fano over the whole grid before OSD gets a turn at any of it. OSD returns
    // a codeword for whatever it is handed, so letting it answer at the first
    // cell would end the search there — with something built out of the noise at
    // an alignment the real signal was never at, and the cell that would have
    // decoded properly never tried.
    //
    // Two separate walks rather than asking each cell for both answers at once,
    // which is worth it for the arithmetic: OSD is a sixth of this decoder's
    // entire run time, and every cell that Fano goes on to decode — and every
    // cell after the first one OSD answers at — was paying for an OSD result
    // that would be thrown away. The walk order is the same, so the cell that
    // ends up reported is the same one either way.
    let cells = || {
        outward(LAG_RETRY_STEPS)
            .flat_map(move |dl| outward(FREQ_RETRY_STEPS).map(move |df| (dl, df)))
    };
    let at = |dl: i32, df: i32| {
        let lag = lag0 + dl * LAG_RETRY_STEP;
        let f = f0_bb + df as f32 * FREQ_RETRY_STEP_HZ;
        Aligned {
            isqs: wspr::demod::tone_amplitudes(idat, qdat, f, lag, drift_hz),
            tone0_hz: f + wspr::baseband::CENTER_HZ - 1.5 * wspr::demod::TONE_SPACING_HZ,
            lag,
        }
    };
    for (dl, df) in cells() {
        let here = at(dl, df);
        if hopeless(&here) {
            continue;
        }
        if let Some(hit) = fano_at(&here) {
            return Some((here, hit));
        }
    }
    for (dl, df) in cells() {
        let here = at(dl, df);
        if hopeless(&here) {
            continue;
        }
        if let Some(hit) = osd_at(&here) {
            return Some((here, hit));
        }
    }
    None
}

/// Sub-bin frequency estimate for a coarse candidate, by parabolic
/// interpolation of the energy its four tones capture.
///
/// The coarse search reports whichever 0.73 Hz bin the signal peaked in, and the
/// decoder wants the frequency five times finer than that — so most of the retry
/// grid exists to hunt for something the spectrum already knows to better
/// precision than it is willing to say. A peak that falls between two bins
/// leaves its trace in the neighbours: fitting a parabola through the three and
/// taking its vertex recovers where the signal actually was.
///
/// The statistic is the total energy in all four tones over the whole
/// transmission, which is the right one to interpolate on for two reasons. It
/// needs no idea of *which* tone was sent or when, so unlike the sync score it
/// does not fall apart at the noise floor — every symbol contributes whatever
/// tone it used. And a flat noise floor cancels out of the fit exactly, in both
/// the numerator and the denominator, so the estimate is unbiased by how noisy
/// the band is; only the variance costs anything.
fn interpolate_freq(idat: &[f32], qdat: &[f32], f0_bb: f32, lag: i32, drift_hz: f32) -> f32 {
    let e = |f| comb_energy(idat, qdat, f, lag, drift_hz);
    let (l, c, r) = (e(f0_bb - INTERP_SPAN_HZ), e(f0_bb), e(f0_bb + INTERP_SPAN_HZ));
    let curvature = l - 2.0 * c + r;
    // Concave-down or nothing: if the middle sample is not the largest, this is
    // not a peak and the vertex would be an extrapolation into a neighbouring
    // signal. Keep what the coarse search said.
    if !(curvature < 0.0) {
        return f0_bb;
    }
    let delta = 0.5 * INTERP_SPAN_HZ * (l - r) / curvature;
    // A genuine interior peak cannot sit more than half a sample away; anything
    // further is the fit being led astray by a neighbour.
    f0_bb + delta.clamp(-COARSE_BIN_HZ / 2.0, COARSE_BIN_HZ / 2.0)
}

/// Energy the four-tone comb centred at `f` collects across the transmission.
fn comb_energy(idat: &[f32], qdat: &[f32], f: f32, lag: i32, drift_hz: f32) -> f32 {
    let isqs = wspr::demod::tone_amplitudes(idat, qdat, f, lag, drift_hz);
    (0..wspr::demod::N_SYMBOLS)
        .map(|i| (0..4).map(|t| isqs.is[t][i].powi(2) + isqs.qs[t][i].powi(2)).sum::<f32>())
        .sum()
}

/// Bin spacing of the coarse spectrum the candidates come from: the 375 Hz
/// baseband over a 512-point transform.
const COARSE_BIN_HZ: f32 = 375.0 / 512.0;

/// How far either side of the coarse pick the parabola is sampled.
///
/// Not the bin spacing, which is the obvious choice and a worse one: a parabola
/// through three points underestimates a peak narrower than its own sample
/// spacing, and at a full bin the outer two samples sit out on the shoulders
/// where the shape has stopped being a parabola at all. Measured over a beacon
/// swept across a whole bin, sampling this close cuts the worst-case error to
/// 0.19 Hz and the mean to 0.04, against 0.39 for the coarse bin it started in.
const INTERP_SPAN_HZ: f32 = 0.5;

/// Whether a grid cell is so poorly aligned that no decoder will do anything
/// with it, cheaply enough to be worth asking before one tries.
///
/// The sync vector is a third of every WSPR symbol and is known in advance, so
/// correlating against it costs a few thousand operations against the millions
/// a Fano attempt can burn. It is much too noisy to *choose* an alignment with —
/// that mistake is what [`align_and_decode`] exists to undo — but it is a
/// perfectly good way to rule one out, and the two are not the same question.
///
/// The threshold is set from measurement, not from wsprd's (which is 0.12 on its
/// own scale): over five synthesised slots, the worst-aligned cell that ever
/// went on to decode scored 0.094, and every one of the 146 successful cells
/// cleared this. It skips four in five of the cells that would have failed,
/// which is most of the decoder's run time, and loses nothing.
fn hopeless(al: &Aligned) -> bool {
    wspr::demod::sync_score_isqs(&al.isqs) < MIN_CELL_SYNC
}

/// See [`hopeless`].
const MIN_CELL_SYNC: f32 = 0.05;

/// The best lag for a coarse candidate — wsprd's `sync_and_demodulate` mode 0.
///
/// Only the timing. The frequency deliberately is not refined here; see
/// [`align_and_decode`] for why picking it by sync score makes things worse.
fn align_lag(idat: &[f32], qdat: &[f32], start_sample: usize, tone0_hz: f32, drift_hz: f32) -> i32 {
    let f0 = tone0_hz + 1.5 * wspr::demod::TONE_SPACING_HZ - wspr::baseband::CENTER_HZ;
    let lag_init = start_sample as i32 / 32;
    let mut best = (f32::NEG_INFINITY, lag_init);
    for dlag in [-128i32, -64, 0, 64, 128] {
        let isqs = wspr::demod::tone_amplitudes(idat, qdat, f0, lag_init + dlag, drift_hz);
        let sync = wspr::demod::sync_score_isqs(&isqs);
        if sync > best.0 {
            best = (sync, lag_init + dlag);
        }
    }
    best.1
}

/// Decode at one alignment with Fano, over each coherent block length.
///
/// This is `mfsk_core::wspr::decode::decode_at_baseband_nblocks`' final stage,
/// reproduced over the same crate's public primitives for two reasons: that
/// function runs the OSD fallback unconditionally and hands back a single result
/// that does not say which produced it — and OSD synthesises a valid codeword
/// for *any* input, so a caller that cannot tell the two apart has to treat
/// every decode as suspect. Split here so each can be spent where it is worth
/// spending, and so the caller can hold OSD's answers to a stricter standard.
fn fano_at(al: &Aligned) -> Option<Hit> {
    best_of_blocks(al, |llrs| fano(llrs).map(|(info, errors)| (info, errors, false)))
}

/// The same alignment, asking the ordered-statistics fallback instead.
fn osd_at(al: &Aligned) -> Option<Hit> {
    best_of_blocks(al, |llrs| {
        wspr::osd::osd_decode(llrs).map(|(info, nhardmin)| (info, nhardmin, true))
    })
}

/// Try one decoder at one alignment over coherent block lengths 1, 2 and 3, and
/// keep the message it was surest of.
fn best_of_blocks(
    al: &Aligned,
    decode: impl Fn(&[f32; 162]) -> Option<([u8; 50], u32, bool)>,
) -> Option<Hit> {
    use mfsk_core::engine::{DecodeContext, MessageCodec};

    // Upstream prefers a Type-1/2 decode over a Type-3 one at equal cost, since
    // a hashed callsign this station cannot invert is the weaker result.
    let mut named: Option<(u32, Hit)> = None;
    let mut hashed_best: Option<(u32, Hit)> = None;

    for nblock in [1usize, 2, 3] {
        let mut llrs = wspr::demod::nblock_bit_metrics(&al.isqs, nblock);
        normalise(&mut llrs);
        deinterleave(&mut llrs);

        let Some((info, errors, from_osd)) = decode(&llrs) else { continue };
        let Some(message) = mfsk_core::msg::Wspr50Message.unpack(&info, &DecodeContext::default())
        else {
            continue;
        };
        let slot = if matches!(message, WsprMessage::Type3 { .. }) {
            &mut hashed_best
        } else {
            &mut named
        };
        if slot.as_ref().is_none_or(|(b, _)| errors < *b) {
            *slot = Some((errors, Hit { message, info, from_osd }));
        }
    }
    named.or(hashed_best).map(|(_, h)| h)
}

/// Run the Fano sequential decoder over one set of deinterleaved bit metrics.
///
/// Returns the fifty information bits and the number of channel bits the
/// recovered codeword disagrees with, or `None` if the search did not reach the
/// end of the trellis.
///
/// This calls `mfsk-core`'s decoder but not its `decode_soft`, because the
/// branch metrics are the whole point — see [`branch_metric`].
fn fano(llrs: &[f32; 162]) -> Option<([u8; 50], u32)> {
    use mfsk_core::engine::FecCodec;
    use mfsk_core::fec::conv::fano as f;

    let bm: Vec<[i32; 2]> =
        llrs.iter().map(|&l| [quantise(branch_metric(l)), quantise(branch_metric(-l))]).collect();
    // 81 = the 50 message bits plus the code's 31-bit zero tail.
    let res = f::fano_decode(&bm, 81, FANO_DELTA, FANO_MAX_CYCLES);
    if !res.converged {
        return None;
    }
    let mut info = [0u8; 50];
    for (i, b) in info.iter_mut().enumerate() {
        *b = (res.data[i / 8] >> (7 - (i % 8))) & 1;
    }
    // Re-encode and count the disagreements, which is how the caller ranks one
    // block length against another.
    let mut codeword = vec![0u8; 162];
    mfsk_core::fec::ConvFano.encode(&info, &mut codeword);
    let errors =
        llrs.iter().zip(&codeword).filter(|&(&l, &c)| (c == 1) != (l < 0.0)).count() as u32;
    Some((info, errors))
}

/// What one code bit is worth to the sequential decoder, given the metric the
/// demodulator produced for it.
///
/// This is the one piece `mfsk-core` cannot be asked for. Its
/// `build_branch_metrics` uses the max-log-MAP approximation, `m = ±l/2 − bias`
/// — a straight line, so the reward for a confident bit grows without limit,
/// and symmetric, so agreeing strongly with a bit pays exactly what disagreeing
/// with it costs. Neither is true of the real metric and both hurt at the noise
/// floor: a handful of loud symbols can carry the search down a wrong path
/// faster than the many quiet correct ones pull it back, and when it does go
/// wrong nothing punishes it hard enough to make it back out.
///
/// The real Fano metric is `log₂(P(y|b) / P(y)) − R`, which for a rate-½ code
/// and an `l` that is a log-likelihood ratio comes out in closed form as
/// `0.5 − log₂(1 + e^−l)`. It **saturates** at +0.5 however confident the bit
/// is and falls away linearly on the wrong side: bounded reward, unbounded
/// penalty. That asymmetry is what makes sequential decoding work, and it is
/// what WSJT-X's `metric_tables` are a tabulation of — computed here instead,
/// which is exact rather than interpolated and copies no data out of another
/// project.
///
/// The floor is there because the penalty really is unbounded, and one symbol
/// that arrived wrong and loud would otherwise decide the whole path.
fn branch_metric(l: f32) -> f32 {
    // log₂(1 + e^−l), arranged so a large negative `l` does not overflow the
    // exponential.
    let log2_1p_exp = if -l > 30.0 {
        -l * core::f32::consts::LOG2_E
    } else {
        (-l).exp().ln_1p() * core::f32::consts::LOG2_E
    };
    (0.5 - log2_1p_exp).max(FANO_METRIC_FLOOR)
}

fn quantise(m: f32) -> i32 {
    (m * FANO_SCALE).round() as i32
}

/// Scale the bit metrics to a fixed spread before Fano sees them.
///
/// A sequential decoder is not scale-invariant. Its bias — the metric it
/// expects to gain per correctly-decoded bit — is what decides when the search
/// gives up on a path and backs out, and `mfsk-core` fixes that bias at a
/// constant while the metrics arriving from the demodulator carry whatever
/// magnitude the signal happened to have. A weak signal produces small metrics,
/// the fixed bias then swamps them, and the decoder abandons paths it should
/// have followed: the frames that fail are the quiet ones, which is the whole
/// population this mode exists for. Its own source says as much, and defers the
/// fix to a normalisation pass here.
///
/// This is that pass, and it is `wsprd`'s: divide by the standard deviation of
/// the metrics and clamp the tails, so a −30 dB frame and a −5 dB one arrive at
/// the decoder looking the same size and the bias means the same thing for both.
fn normalise(llrs: &mut [f32; 162]) {
    let n = llrs.len() as f32;
    let mean = llrs.iter().sum::<f32>() / n;
    let var = llrs.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
    let sd = var.sqrt();
    if !(sd > 0.0) || !sd.is_finite() {
        return;
    }
    let k = LLR_TARGET_SD / sd;
    // One wild metric must not decide a frame.
    let clamp = LLR_CLAMP_SD * LLR_TARGET_SD;
    for v in llrs.iter_mut() {
        *v = (*v * k).clamp(-clamp, clamp);
    }
}

/// Deinterleave 162 channel LLRs: the WSPR permutation is a bit-reversal of the
/// symbol index, skipping the reversals that land past the end.
fn deinterleave(llrs: &mut [f32; 162]) {
    let mut tmp = [0f32; 162];
    let (mut p, mut i) = (0usize, 0u8);
    while p < 162 {
        let j = i.reverse_bits() as usize;
        if j < 162 {
            tmp[p] = llrs[j];
            p += 1;
        }
        i = i.wrapping_add(1);
    }
    *llrs = tmp;
}

/// How much of the received tone energy the decoded message accounts for.
///
/// WSPR's convolutional code carries no CRC, so nothing downstream of the Fano
/// decoder knows whether a message is real — and the OSD fallback underneath it
/// will synthesise a *valid codeword for any input*, which is how a slot of
/// noise turns into a plausible-looking callsign. The only thing that can tell
/// the two apart is the audio: re-encode the message we are about to report and
/// ask whether the tones it predicts are the tones that actually arrived.
///
/// Every symbol's tone is `2·data + sync`. The sync half is fixed and a phantom
/// gets it right by construction, so it says nothing; the *data* half is what
/// the message decides. This scores each symbol's claimed tone against the one
/// differing only in that data bit, normalised so the result is a correlation:
/// around zero when the message is unrelated to the audio, and climbing toward
/// one as the signal rises out of the noise.
fn fit_of(al: &Aligned, info: &[u8; 50]) -> f32 {
    let symbols = wspr::encode_channel_symbols(info);
    let (mut num, mut den) = (0.0f32, 0.0f32);
    for (i, &sym) in symbols.iter().enumerate() {
        let t = sym as usize;
        // Same sync bit, opposite data bit — the one the message chose against.
        let alt = t ^ 2;
        let mag = |k: usize| (al.isqs.is[k][i].powi(2) + al.isqs.qs[k][i].powi(2)).sqrt();
        num += mag(t) - mag(alt);
        den += mag(t) + mag(alt);
    }
    if den > 0.0 { num / den } else { 0.0 }
}

/// Whether a decode is physically possible, independent of how well it fits.
///
/// A WSPR transmission lives in a 200 Hz window and starts within a couple of
/// seconds of the slot anchor. A decode outside either bound did not come from
/// a beacon, however well-formed its callsign reads — and one reported to
/// WSPRnet is worse than one dropped, because the network pools it.
fn is_plausible(d: &WsprDecode) -> bool {
    let hz = d.carrier_hz();
    // The window plus a little, since the operator's dial can be off by a few
    // hertz and the whole signal moves with it.
    (sdroxide_types::WSPR_WINDOW_LO_HZ - 10.0..=sdroxide_types::WSPR_WINDOW_HI_HZ + 10.0)
        .contains(&hz)
        && (-2.5..=4.0).contains(&d.dt_sec)
        && match &d.message {
            WsprMessage::Type1 { callsign, grid, .. } => is_callsign(callsign) && is_grid4(grid),
            WsprMessage::Type2 { callsign, .. } => {
                // A compound callsign is a prefix or a suffix bolted onto a
                // base one; both halves still have to be callsign-shaped.
                callsign.split('/').all(|p| !p.is_empty() && is_call_chars(p))
                    && callsign.split('/').count() == 2
            }
            // The callsign is a hash this station cannot invert, so there is
            // nothing to check but the locator that came in the clear.
            WsprMessage::Type3 { grid6, .. } => {
                is_grid4(&grid6.chars().take(4).collect::<String>())
                    && grid6.len() == 6
                    && grid6[4..].chars().all(|c| c.is_ascii_alphabetic())
            }
        }
}

/// The callsign grammar WSPR's Type-1 layout can express: one or two leading
/// characters, a digit, then one to three letters.
///
/// This is the packer's own rule rather than a guess at what callsigns look
/// like — it right-aligns into six slots so the digit lands in slot 2, taking
/// the call as it stands when character 2 is a digit and shifting it one place
/// when character 1 is. A string that fits neither was never a Type-1 message,
/// whatever it reads as. The check that earns its keep is the plain one:
/// no spaces, which are the packer's blank padding surfacing mid-word.
fn is_callsign(c: &str) -> bool {
    let b = c.as_bytes();
    if !(3..=6).contains(&b.len()) || !is_call_chars(c) {
        return false;
    }
    let d = if b[2].is_ascii_digit() {
        2
    } else if b[1].is_ascii_digit() {
        1
    } else {
        return false;
    };
    let suffix = &b[d + 1..];
    (1..=3).contains(&suffix.len()) && suffix.iter().all(|x| x.is_ascii_alphabetic())
}

fn is_call_chars(c: &str) -> bool {
    !c.is_empty() && c.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// A four-character Maidenhead locator: field A–R, square 0–9.
fn is_grid4(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 4
        && (b'A'..=b'R').contains(&b[0])
        && (b'A'..=b'R').contains(&b[1])
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
}

/// One decode as the passes hold it: what a later pass needs to subtract it,
/// and what [`from_result`] makes a spot of. mfsk-core's `WsprResult` carried
/// this until 0.14 made it `#[non_exhaustive]`, which closed it to a decoder
/// outside the crate.
struct Found {
    message: WsprMessage,
    freq_hz: f32,
    /// Where the frame starts in the padded buffer.
    start_sample: usize,
    info_bits: [u8; 50],
}

/// Build our result from a decode, taking the SNR and drift from the coarse
/// candidate that found it — the whole point of this module.
fn from_result(
    r: &Found,
    c: &wspr::coarse_baseband::BasebandCandidate,
    pad: usize,
    sample_rate: u32,
) -> WsprDecode {
    WsprDecode {
        message: r.message.clone(),
        freq_hz: r.freq_hz,
        // Upstream computes this the same way, but `decode_at_baseband` is
        // unaware of our padding and would report the offset within the padded
        // buffer.
        dt_sec: (r.start_sample as i64 - pad as i64) as f32 / sample_rate as f32 - 1.0,
        snr_db: c.snr_db,
        drift_hz: reportable_drift(c.drift_hz),
        fit: 0.0,
    }
}

/// Append unless the same message is already present at nearly the same place.
///
/// Returns whether it was appended. `start_sample` is in the caller's
/// (unpadded) time base.
fn push_unique(out: &mut Vec<WsprDecode>, d: WsprDecode, start_sample: usize) -> bool {
    let dup = out.iter().any(|prev| {
        prev.message == d.message
            && (prev.freq_hz - d.freq_hz).abs() <= FREQ_DEDUP_HZ
            && (start_of(prev) as i64 - start_sample as i64).abs() <= TIME_DEDUP_SAMPLES
    });
    if !dup {
        out.push(d);
    }
    !dup
}

/// Reconstruct a decode's start sample from its `dt`, so de-duplication does not
/// need a parallel list of them.
fn start_of(d: &WsprDecode) -> usize {
    (((d.dt_sec + 1.0) * 12_000.0).max(0.0)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The synthesised round trip. Weak evidence on its own — a decoder and a
    /// transmitter that share an author agree with each other by construction —
    /// which is why the real proof is `tests/wspr_corpus.rs` against the WSJT-X
    /// reference recording. This one is here to catch the plumbing: wiring the
    /// wrong buffer in, or losing the pad offset.
    #[test]
    fn a_synthesised_beacon_decodes_back_with_its_message_and_frequency() {
        let audio = crate::wspr::tx::synthesize("K1ABC", "FN42", 37, 12_000, 1500.0, 0.5)
            .expect("K1ABC FN42 37 is a valid type 1 message");
        // Place it where a real one sits: one second into a 120 s slot.
        let mut slot = vec![0f32; 120 * 12_000];
        let start = 12_000;
        slot[start..start + audio.len()].copy_from_slice(&audio);

        let got = decode_slot(&slot, 12_000);
        let hit = got
            .iter()
            .find(|d| matches!(&d.message, WsprMessage::Type1 { callsign, .. } if callsign == "K1ABC"))
            .unwrap_or_else(|| panic!("K1ABC not among {got:?}"));
        assert_eq!(hit.message.to_string(), "K1ABC FN42 37");
        // 1500 Hz is where `synthesize` was asked to put tone 0, and the
        // carrier sits 1.5 spacings above it.
        assert!((hit.freq_hz - 1500.0).abs() < 2.0, "tone 0 at {}", hit.freq_hz);
        assert!(hit.dt_sec.abs() < 0.5, "dt {} is not near zero", hit.dt_sec);
    }

    #[test]
    fn a_slot_of_silence_decodes_to_nothing() {
        assert!(decode_slot(&vec![0f32; 120 * 12_000], 12_000).is_empty());
    }

    /// A deterministic Gaussian source. No `rand` in this crate's tree, and a
    /// test that decoded a different signal on every run would be worse than no
    /// test at all.
    struct Noise(u64);

    impl Noise {
        fn next_u32(&mut self) -> u32 {
            // xorshift64*
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
        }
        fn unit(&mut self) -> f64 {
            (self.next_u32() as f64 + 0.5) / 4_294_967_296.0
        }
        /// Box–Muller, one value per call (the second is discarded; this is a
        /// test, not a hot loop).
        fn gaussian(&mut self) -> f64 {
            let (u1, u2) = (self.unit(), self.unit());
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    /// Build a slot holding one beacon at a known SNR, referenced the way WSPR
    /// reports it — in a 2500 Hz noise bandwidth.
    ///
    /// The audio is real and spans 0–6000 Hz at 12 kHz, so noise of variance σ²
    /// has power `σ²·2500/6000` inside the reference bandwidth. That factor is
    /// the whole reason this helper exists rather than a bare `signal + noise`.
    fn slot_at_snr(target_db: f64, seed: u64) -> Vec<f32> {
        let amp = 0.05f32;
        let sig_power = (amp as f64 * amp as f64) / 2.0;
        let sigma2 = sig_power / 10f64.powf(target_db / 10.0) * (6000.0 / 2500.0);
        let sigma = sigma2.sqrt();

        let burst = crate::wspr::tx::synthesize("K1ABC", "FN42", 37, 12_000, 1500.0, amp)
            .expect("valid message");
        let mut slot = vec![0f32; 120 * 12_000];
        let start = 12_000;
        slot[start..start + burst.len()].copy_from_slice(&burst);
        let mut n = Noise(seed);
        for s in slot.iter_mut() {
            *s += (n.gaussian() * sigma) as f32;
        }
        slot
    }

    /// A busy window: beacons across the band, strong down to under the floor.
    ///
    /// Returns the slot and the callsigns actually transmitted, so a test can
    /// ask not only how many came back but whether anything came back that was
    /// never sent.
    fn crowded_slot(seed: u64) -> (Vec<f32>, Vec<String>) {
        const GRIDS: &[&str] = &["FN42", "IO91", "QE38", "PM95", "JO70", "FM19", "JN48", "IM98"];
        let sigma = 0.02f64;
        let mut n = Noise(seed);
        let mut slot: Vec<f32> = (0..120 * 12_000).map(|_| (n.gaussian() * sigma) as f32).collect();

        let mut calls = Vec::new();
        let count = 16;
        for i in 0..count {
            let call = format!("K{}A{}A", i % 10, (b'A' + i as u8) as char);
            let f0 = 1408.0 + 184.0 * (i as f64 / (count - 1) as f64);
            // −6 dB down to −30: the whole range an operator cares about.
            let snr = -6.0 - 24.0 * (i as f64 / (count - 1) as f64);
            let n_ref = sigma * sigma * 2500.0 / 6000.0;
            let amp = (2.0 * n_ref * 10f64.powf(snr / 10.0)).sqrt() as f32;
            let burst =
                crate::wspr::tx::synthesize(&call, GRIDS[i % GRIDS.len()], 23, 12_000, f0, amp)
                    .expect("a K-digit-three-letter call packs");
            let start = 12_000 + (i % 5) * 1_200;
            for (k, &v) in burst.iter().enumerate() {
                if start + k < slot.len() {
                    slot[start + k] += v;
                }
            }
            calls.push(call);
        }
        (slot, calls)
    }

    /// The one that matters most, and the reason the decode path was rebuilt:
    /// **nothing may come back that was never transmitted.**
    ///
    /// WSPR's code carries no CRC, and underneath the Fano decoder sits an
    /// ordered-statistics fallback that answers with the nearest valid codeword
    /// to whatever it is given — so pointed at noise it does not fail, it
    /// invents a callsign. Before the fit gate this slot came back with seven
    /// stations nobody sent, mixed in among the real ones and indistinguishable
    /// from them on screen: readable callsigns, plausible grids, sensible power
    /// levels, sitting at the frequencies of the weak signals they were built
    /// out of.
    ///
    /// A spot is not a private observation. It goes to WSPRnet and is pooled
    /// into the path statistics everyone else reads, so an invented one is not
    /// merely this operator's problem — which is why this asserts zero, not
    /// few, and why it is worth failing the build over.
    #[test]
    fn a_busy_band_yields_no_station_that_was_never_transmitted() {
        for seed in [0xB0B_0001u64, 0xB0B_0002, 0xB0B_0003] {
            let (slot, sent) = crowded_slot(seed);
            let got = decode_slot(&slot, 12_000);
            let invented: Vec<String> = got
                .iter()
                .map(|d| d.message.to_string())
                .filter(|m| {
                    let call = m.split_whitespace().next().unwrap_or("");
                    !sent.iter().any(|s| s == call)
                })
                .collect();
            assert!(invented.is_empty(), "seed {seed:x} invented {invented:?}");
        }
    }

    /// And it still has to hear the band. A decoder that reports nothing also
    /// reports nothing false, so the guarantee above is only worth having
    /// beside a floor on how much actually comes back.
    #[test]
    fn a_busy_band_is_mostly_decoded_including_signals_under_twenty_five_dB_down() {
        let (slot, sent) = crowded_slot(0xB0B_0001);
        let got = decode_slot(&slot, 12_000);
        let heard: Vec<&String> = sent
            .iter()
            .filter(|c| {
                got.iter().any(|d| d.message.to_string().split_whitespace().next() == Some(c))
            })
            .collect();
        assert!(heard.len() >= 12, "only {} of {} came back: {heard:?}", heard.len(), sent.len());
        // The bottom of the list is the whole point of the mode: the last four
        // stations sit between −25 and −30 dB.
        let weak = sent[sent.len() - 4..].iter().filter(|c| heard.iter().any(|h| h == c)).count();
        assert!(weak >= 1, "nothing below −25 dB decoded");
    }

    /// The weak end, which is the whole reason this mode exists.
    ///
    /// A lone beacon at −29 dB used to be missed outright: the coarse search
    /// hands over a frequency good to about a third of a hertz, and the decoder
    /// needs it to a fifth of one, so the alignment it was given was never quite
    /// the alignment it could decode at. Nothing about the demodulator or the
    /// FEC had to change to fix it — only looking in the right place.
    #[test]
    fn a_lone_beacon_near_the_floor_is_heard() {
        for (target, want) in [(-26.0f64, 3), (-29.0, 2)] {
            let heard = [0xF100_0001u64, 0xF100_0002, 0xF100_0003]
                .iter()
                .filter(|&&seed| {
                    decode_slot(&slot_at_snr(target, seed), 12_000).iter().any(|d| {
                        matches!(&d.message, WsprMessage::Type1 { callsign, .. } if callsign == "K1ABC")
                    })
                })
                .count();
            assert!(heard >= want, "at {target} dB only {heard} of 3 slots decoded");
        }
    }

    /// Candidates are attempted in parallel, so the same recording has to come
    /// back the same way every time — a decoder whose spot list depended on
    /// which thread finished first would be impossible to trust or to test.
    #[test]
    fn the_same_slot_decodes_the_same_way_every_time() {
        let (slot, _) = crowded_slot(0xD37E_0001);
        let first = decode_slot(&slot, 12_000);
        assert!(first.len() > 8, "the fixture decoded almost nothing: {}", first.len());
        for _ in 0..2 {
            assert_eq!(decode_slot(&slot, 12_000), first, "decode order is not deterministic");
        }
    }

    /// The frequency on a spot is a measurement other people use, and the
    /// coarse search only ever knew it to the nearest 0.73 Hz bin. Interpolating
    /// the spectral peak is what makes the reported figure finer than the grid
    /// it was found on — so this puts a beacon deliberately between two bins and
    /// checks the number that would go to WSPRnet.
    #[test]
    fn the_reported_frequency_is_finer_than_the_bin_it_was_found_in() {
        // Quarter-bin steps across a whole coarse bin, so no single sub-bin
        // phase can pass by luck.
        for k in 0..4 {
            let tone0 = 1500.0 + k as f64 * (375.0 / 512.0) / 4.0;
            let truth = tone0 as f32 + 1.5 * mfsk_core::wspr::demod::TONE_SPACING_HZ;
            let audio = crate::wspr::tx::synthesize("K1ABC", "FN42", 37, 12_000, tone0, 0.05)
                .expect("valid message");
            let mut slot = vec![0f32; 120 * 12_000];
            slot[12_000..12_000 + audio.len()].copy_from_slice(&audio);
            let mut n = Noise(0xF1E0_0000 + k);
            for s in slot.iter_mut() {
                *s += (n.gaussian() * 0.02) as f32;
            }
            let got = decode_slot(&slot, 12_000);
            let d = got
                .iter()
                .find(|d| matches!(&d.message, WsprMessage::Type1 { callsign, .. } if callsign == "K1ABC"))
                .unwrap_or_else(|| panic!("no decode at tone0 {tone0}"));
            let err = (d.carrier_hz() - truth).abs();
            assert!(err < 0.25, "reported {} for {truth}, off by {err:.3} Hz", d.carrier_hz());
        }
    }

    /// A stable carrier must report no drift. The coarse search cannot resolve
    /// a hertz — see [`reportable_drift`] — and reporting the tie made every
    /// steady beacon on a test band come back at −1 Hz.
    #[test]
    fn a_drift_the_search_cannot_resolve_is_reported_as_none() {
        assert_eq!(reportable_drift(-1.0), 0.0);
        assert_eq!(reportable_drift(1.0), 0.0);
        assert_eq!(reportable_drift(0.0), 0.0);
        // What it can resolve is passed through.
        assert_eq!(reportable_drift(-3.0), -3.0);
        assert_eq!(reportable_drift(2.0), 2.0);
    }

    /// The plausibility gate, on the shapes that actually came out of OSD.
    #[test]
    fn a_message_that_is_not_shaped_like_a_station_is_refused() {
        // Real ones.
        assert!(is_callsign("K1ABC"));
        assert!(is_callsign("OK1UNL"));
        assert!(is_callsign("G0XYZ"));
        assert!(is_callsign("VK7ZZ"));
        assert!(is_callsign("2E0ABC"));
        // Invented ones this decoder has actually reported. Note what is *not*
        // asserted here: "D31NEL" and "I1KSN" also came out of OSD and are
        // perfectly well-formed, so no grammar can refuse them. Shape is the
        // cheap half of the gate; [`fit_of`] is the half that does the work.
        assert!(!is_callsign("OZ4 YD"), "a space is the packer's padding, not a callsign");
        assert!(!is_callsign("ABCDEF"), "no digit at all");
        assert!(!is_callsign("K8P/KO0SLR"), "a compound call is not Type 1");
        assert!(is_grid4("JO70"));
        assert!(!is_grid4("ZZ99"), "the field only runs A–R");
        assert!(!is_grid4("JO7"));
    }

    /// The calibration test, and the reason this module exists at all: the SNR
    /// we hand to WSPRnet has to be the SNR that was actually there.
    ///
    /// A spot is a measurement pooled with everybody else's. Reporting a number
    /// that is systematically wrong does not merely mislead this operator — it
    /// corrupts the path statistics for every station on the other end.
    ///
    /// The tolerance is wide (±5 dB) on purpose: the estimator is a coarse
    /// spectral one over a 0.73 Hz bin against a 30th-percentile noise floor,
    /// and `wsprd` itself is not better than a few dB. What this catches is the
    /// failure that matters — a constant offset, or the number not tracking the
    /// signal at all.
    #[test]
    fn the_reported_snr_tracks_the_signal_that_was_actually_there() {
        let mut seen = Vec::new();
        for (i, &target) in [-14.0f64, -20.0, -26.0].iter().enumerate() {
            let slot = slot_at_snr(target, 0x5EED_0000 + i as u64);
            let got = decode_slot(&slot, 12_000);
            let hit = got
                .iter()
                .find(|d| matches!(&d.message, WsprMessage::Type1 { callsign, .. } if callsign == "K1ABC"))
                .unwrap_or_else(|| panic!("no decode at {target} dB: {got:?}"));
            assert!(
                (hit.snr_db as f64 - target).abs() < 5.0,
                "at {target} dB the decoder reported {:.1}",
                hit.snr_db
            );
            seen.push(hit.snr_db as f64);
        }
        // And it moves the right way, monotonically, over a 12 dB span — the
        // check a constant offset would pass but a broken estimator would not.
        assert!(
            seen[0] > seen[1] && seen[1] > seen[2],
            "SNR did not fall with the signal: {seen:?}"
        );
    }

    #[test]
    fn the_carrier_sits_above_tone_zero_by_a_symmetric_half_of_the_tone_set() {
        let d = WsprDecode {
            message: WsprMessage::Type1 {
                callsign: "K1ABC".into(),
                grid: "FN42".into(),
                power_dbm: 37,
            },
            freq_hz: 1500.0,
            dt_sec: 0.0,
            snr_db: -20.0,
            drift_hz: 0.0,
            fit: 0.5,
        };
        // Four tones 1.4648 Hz apart: the centre is 1.5 spacings up, ≈2.2 Hz.
        assert!((d.carrier_hz() - 1502.197).abs() < 0.01, "{}", d.carrier_hz());
    }
}
