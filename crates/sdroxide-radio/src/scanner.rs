//! The arithmetic behind a frequency scan: where to put the front end, and what
//! in the resulting spectrum counts as a busy channel.
//!
//! Kept apart from the engine's state machine because this is the part with
//! answers that can be checked. The state machine decides *when* to look; these
//! decide *where*, and whether what came back was a signal.

/// Fraction of the device span either side of the centre that a sweep trusts.
/// The outer edges are in the anti-alias filter's roll-off, where a real signal
/// reads low and might be missed.
const USABLE_HALF: f64 = 0.4;
/// How far the front end moves between slices, as a fraction of the span. Less
/// than twice [`USABLE_HALF`], so consecutive slices overlap rather than merely
/// touching — a signal exactly on a seam is then seen twice instead of not at
/// all.
const SLICE_STRIDE: f64 = 0.7;
/// Bins either side of DC ignored when hunting for peaks. A zero-IF front end
/// piles its LO leakage there, and it is not a station.
const DC_GUARD_BINS: usize = 4;

/// Where to put the hardware centre, in order, to cover `lo..hi` with a front
/// end that sees `span_hz` at a time.
///
/// One slice when the whole range already fits, which is the common case on a
/// wideband receiver: scanning 2 m at 1.536 Msps is then a single tune and one
/// look at the FFT, not a hundred and sixty visits.
pub fn slice_centers(lo: f64, hi: f64, span_hz: f64) -> Vec<f64> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo || !(span_hz > 0.0) {
        return Vec::new();
    }
    let usable = span_hz * USABLE_HALF;
    if hi - lo <= usable * 2.0 {
        return vec![(lo + hi) / 2.0];
    }
    let stride = span_hz * SLICE_STRIDE;
    let mut out = Vec::new();
    let mut c = lo + usable;
    // `<` rather than `<=` on the far edge, plus the unconditional last slice
    // below: the final step is placed to end exactly on `hi` instead of running
    // past it, so the top of the range is never searched at the roll-off.
    while c + usable < hi {
        out.push(c);
        c += stride;
    }
    out.push(hi - usable);
    out
}

/// Hardware centres that cover `freqs` with a front end that sees `span_hz` at
/// a time, each with the indices of the frequencies it covers.
///
/// The memory-scan twin of [`slice_centers`], and a different problem: a range
/// scan has to search a continuous band, so its slices tile it and overlap; a
/// memory scan has a *list*, and the whole point is to look at nothing else. So
/// the frequencies are grouped greedily in order — everything that fits inside
/// one usable window with the lowest one still uncovered — and each group's
/// centre is put in the middle of what it holds, which leaves the members as
/// far from the roll-off as they can be.
///
/// Two hundred marine, airband and repeater channels on a receiver that sees
/// 2 MHz at a time therefore come down to a handful of tunes, each answered by
/// one transform, instead of two hundred visits of a settling time each
/// (issue #228). A list spread across three bands still costs three, which is
/// what it must: the receiver cannot be in two places at once.
///
/// Frequencies that are not finite are dropped. The groups come out in
/// ascending order, and every index appears exactly once.
pub fn memory_slices(freqs: &[f64], span_hz: f64) -> Vec<(f64, Vec<usize>)> {
    if !(span_hz > 0.0) {
        return Vec::new();
    }
    let width = span_hz * USABLE_HALF * 2.0;
    let mut order: Vec<usize> = (0..freqs.len()).filter(|&i| freqs[i].is_finite()).collect();
    order.sort_by(|&a, &b| freqs[a].total_cmp(&freqs[b]));

    let mut out: Vec<(f64, Vec<usize>)> = Vec::new();
    let mut group: Vec<usize> = Vec::new();
    let mut first = 0.0f64;
    for i in order {
        // `<=` so a list whose spread is exactly one window is one slice.
        if !group.is_empty() && freqs[i] - first > width {
            let last = freqs[*group.last().expect("non-empty")];
            out.push(((first + last) / 2.0, std::mem::take(&mut group)));
        }
        if group.is_empty() {
            first = freqs[i];
        }
        group.push(i);
    }
    if !group.is_empty() {
        let last = freqs[*group.last().expect("non-empty")];
        out.push(((first + last) / 2.0, group));
    }
    out
}

/// Which of `freqs` the spectrum says are busy, as indices into it.
///
/// The memory-scan twin of [`busy_channels`], and the simpler half of the pair:
/// there is nothing to search for and nothing to snap to a grid, because the
/// channels are already known. Each one is measured the way `busy_channels`
/// measures a candidate — the total power across a channel's worth of bins,
/// which is what the receiver's own squelch measures — so one threshold means
/// the same thing to the sweep, to the dwell that confirms it and to the audio
/// gate.
///
/// `bandwidths_hz` is per frequency: a memory carries the passband it was
/// stored with, and 16 kHz of NFM and 300 Hz of CW are not the same channel.
///
/// Two frequencies are reported busy without being measured at all. One outside
/// the usable window is in the anti-alias roll-off, where a real signal reads
/// low — but unlike a range scan, which has a neighbouring slice to catch it,
/// this is a channel somebody asked for by name, so it is passed to the dwell
/// rather than dropped. One sitting on the span's centre is on a zero-IF front
/// end's own LO leakage, which is loud and is not a station: there the
/// measurement is worthless in the other direction, and the dwell — which
/// listens through the receiver, DC block and all — is the honest answer.
///
/// The cost of either is one settling time on a channel that turns out to be
/// quiet. The cost of guessing wrong the other way is a channel the operator
/// stored and the scanner never stops on.
pub fn busy_memories(
    bins: &[f32],
    center_hz: f64,
    span_hz: f64,
    freqs: &[f64],
    bandwidths_hz: &[f64],
    threshold_db: f32,
) -> Vec<usize> {
    if bins.is_empty() || !(span_hz > 0.0) {
        return (0..freqs.len()).collect();
    }
    let usable = span_hz * USABLE_HALF;
    let bin_hz = span_hz / bins.len() as f64;
    let mut out = Vec::new();
    for (i, &f) in freqs.iter().enumerate() {
        if !f.is_finite() {
            continue;
        }
        let off = f - center_hz;
        let bw = bandwidths_hz.get(i).copied().unwrap_or(0.0).abs().max(bin_hz);
        // Either guard: listen rather than measure.
        if off.abs() > usable || off.abs() <= (DC_GUARD_BINS as f64 + bw / bin_hz / 2.0) * bin_hz {
            out.push(i);
            continue;
        }
        if channel_power_db(bins, center_hz, span_hz, f, bw).is_some_and(|db| db >= threshold_db) {
            out.push(i);
        }
    }
    out
}

/// Frequencies inside `lo..hi` where the spectrum says something is on the air.
///
/// `bins` are dBFS, frequency-ascending, covering `center_hz ± span_hz/2`. The
/// test is *channel* power — the total across a channel's worth of bins, not the
/// height of one — because that is the same quantity the receiver's own squelch
/// measures, so one threshold in one unit serves the search, the confirmation
/// and the audio gate. A single-bin test would instead make wide signals look
/// weak and narrow ones look strong, and the threshold would mean something
/// different for every mode.
///
/// Results are snapped to the `step_hz` channel grid and deduplicated, so a
/// signal spread across several bins yields one channel rather than a cluster.
pub fn busy_channels(
    bins: &[f32],
    center_hz: f64,
    span_hz: f64,
    lo: f64,
    hi: f64,
    bandwidth_hz: f64,
    step_hz: f64,
    threshold_db: f32,
) -> Vec<f64> {
    let n = bins.len();
    if n == 0 || !(span_hz > 0.0) || !(step_hz > 0.0) {
        return Vec::new();
    }
    let bin_hz = span_hz / n as f64;
    let width = ((bandwidth_hz / bin_hz).ceil() as usize).clamp(1, n);
    let base = center_hz - span_hz / 2.0;

    // Channel power at each window position, as dB again.
    let linear: Vec<f64> = bins.iter().map(|&d| 10f64.powf(d as f64 / 10.0)).collect();
    let mut running: f64 = linear[..width].iter().sum();
    let mut channel = Vec::with_capacity(n - width + 1);
    channel.push(running);
    for i in width..n {
        running += linear[i] - linear[i - width];
        channel.push(running);
    }

    let dc = n / 2;
    let usable = span_hz * USABLE_HALF;
    let mut out: Vec<f64> = Vec::new();
    let mut run_start: Option<usize> = None;
    for i in 0..=channel.len() {
        // The window's centre is what its power belongs to.
        let over = i < channel.len() && 10.0 * channel[i].log10() >= threshold_db as f64;
        match (over, run_start) {
            (true, None) => run_start = Some(i),
            (false, Some(start)) => {
                // A window narrower than the signal reads the same total at
                // every position that covers all of it, so the run has a
                // plateau rather than a peak; its middle is the channel. Taking
                // either end instead would bias the answer by half a window,
                // which at a 12.5 kHz grid is enough to snap to the wrong
                // channel.
                let best = (start..i).fold(0.0f64, |m, k| m.max(channel[k]));
                let plateau = (start..i).filter(|&k| channel[k] >= best * 0.9);
                let (first, last) = (
                    plateau.clone().next().expect("a non-empty run"),
                    plateau.last().expect("a non-empty run"),
                );
                let mid = (first + last) / 2 + width / 2;
                let f = base + mid as f64 * bin_hz;
                let near_dc = mid.abs_diff(dc) <= DC_GUARD_BINS + width / 2;
                if !near_dc && (f - center_hz).abs() <= usable && f >= lo && f <= hi {
                    let snapped = (f / step_hz).round() * step_hz;
                    if snapped >= lo
                        && snapped <= hi
                        && out.last().is_none_or(|&p| (snapped - p).abs() > step_hz / 2.0)
                    {
                        out.push(snapped);
                    }
                }
                run_start = None;
            }
            _ => {}
        }
    }
    out
}

/// Channel power (dBFS) around `at_hz`, from the same spectrum
/// [`busy_channels`] reads.
///
/// The fallback level reading for an engine with no audio chain at all — a
/// headless one with no sound device, which still has a panadapter and so still
/// has evidence. Returns `None` when the frequency is outside the span.
pub fn channel_power_db(
    bins: &[f32],
    center_hz: f64,
    span_hz: f64,
    at_hz: f64,
    bandwidth_hz: f64,
) -> Option<f32> {
    let n = bins.len();
    if n == 0 || !(span_hz > 0.0) {
        return None;
    }
    let bin_hz = span_hz / n as f64;
    let width = ((bandwidth_hz / bin_hz).ceil() as usize).clamp(1, n);
    let mid = ((at_hz - (center_hz - span_hz / 2.0)) / bin_hz).round();
    if !(0.0..n as f64).contains(&mid) {
        return None;
    }
    let start = (mid as usize).saturating_sub(width / 2).min(n - width);
    let total: f64 = bins[start..start + width].iter().map(|&d| 10f64.powf(d as f64 / 10.0)).sum();
    Some((10.0 * (total + 1e-30).log10()) as f32)
}

/// The most bins a panorama keeps across its range. The full-band strip pools
/// to far fewer than this, so more would only be memory; a narrow range keeps
/// the front end's own resolution instead (see [`panorama_bins`]).
pub const PANORAMA_MAX_BINS: usize = 8192;

/// How many bins a panorama of `lo..hi` gets from a front end that sees
/// `span_hz` in `fft_bins` bins: its own resolution, capped at
/// [`PANORAMA_MAX_BINS`].
pub fn panorama_bins(lo: f64, hi: f64, span_hz: f64, fft_bins: usize) -> usize {
    if !(hi > lo && span_hz > 0.0) || fft_bins == 0 {
        return 0;
    }
    let bin_hz = span_hz / fft_bins as f64;
    (((hi - lo) / bin_hz).ceil() as usize).clamp(1, PANORAMA_MAX_BINS)
}

/// Write one sweep slice into a panorama: the part of `bins` (a spectrum in
/// dB across `center_hz ± span_hz / 2`) a sweep trusts — inside the usable
/// half-width, clear of DC — lands on the panorama bins of `lo..hi` it
/// covers. A panorama bin wider than the front end's bins gets their mean
/// power; one narrower gets the bin under its centre. Bins this slice does
/// not cover are left as they were.
pub fn stitch_slice(
    pano: &mut [f32],
    lo: f64,
    hi: f64,
    bins: &[f32],
    center_hz: f64,
    span_hz: f64,
) {
    let (n, m) = (bins.len(), pano.len());
    if n == 0 || m == 0 || !(span_hz > 0.0) || !(hi > lo) {
        return;
    }
    let bin_hz = span_hz / n as f64;
    let base = center_hz - span_hz / 2.0;
    let usable = span_hz * USABLE_HALF;
    let dc = n / 2;
    let pano_hz = (hi - lo) / m as f64;
    for (k, out) in pano.iter_mut().enumerate() {
        let f0 = lo + k as f64 * pano_hz;
        let f1 = f0 + pano_hz;
        let mid = (f0 + f1) / 2.0;
        if (mid - center_hz).abs() > usable {
            continue;
        }
        let i0 = ((f0 - base) / bin_hz).floor().max(0.0) as usize;
        let i1 = (((f1 - base) / bin_hz).ceil() as usize).min(n);
        let (i0, i1) = if i1 <= i0 {
            let i = (((mid - base) / bin_hz) as usize).min(n - 1);
            (i, i + 1)
        } else {
            (i0, i1)
        };
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for (i, &db) in bins.iter().enumerate().take(i1).skip(i0) {
            if i.abs_diff(dc) <= DC_GUARD_BINS || !db.is_finite() {
                continue;
            }
            sum += 10f64.powf(db as f64 / 10.0);
            count += 1;
        }
        if count > 0 {
            *out = (10.0 * (sum / count as f64).log10()) as f32;
        }
    }
}

/// Fill the bins of a panorama no slice has written yet (NaN) — the guard
/// around each slice's DC, a slice the front end could not tune — from their
/// nearest written neighbours, so the strip draws a floor there rather than a
/// hole. All-unwritten stays as it is.
pub fn fill_panorama_gaps(pano: &mut [f32]) {
    let written: Vec<usize> = (0..pano.len()).filter(|&i| pano[i].is_finite()).collect();
    let (Some(&first), Some(&last)) = (written.first(), written.last()) else {
        return;
    };
    for i in 0..first {
        pano[i] = pano[first];
    }
    for i in last + 1..pano.len() {
        pano[i] = pano[last];
    }
    for w in written.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b > a + 1 {
            let (va, vb) = (pano[a], pano[b]);
            for i in a + 1..b {
                let t = (i - a) as f32 / (b - a) as f32;
                pano[i] = va + (vb - va) * t;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPAN: f64 = 1_536_000.0;

    #[test]
    fn a_range_inside_one_span_is_a_single_tune() {
        let c = slice_centers(144e6, 145e6, SPAN);
        assert_eq!(c, vec![144.5e6], "1 MHz fits inside 0.8 x 1.536 MHz");
    }

    /// Every frequency in the range has to fall inside some slice's usable
    /// window, or the scan quietly has holes in it.
    #[test]
    fn slices_cover_the_whole_range() {
        for (lo, hi) in [(144e6, 146e6), (430e6, 440e6), (88e6, 108e6), (144e6, 144.9e6)] {
            let centers = slice_centers(lo, hi, SPAN);
            assert!(!centers.is_empty(), "{lo}..{hi}");
            let usable = SPAN * USABLE_HALF;
            let mut f = lo;
            while f <= hi {
                assert!(
                    centers.iter().any(|&c| (f - c).abs() <= usable),
                    "{f} is in no slice of {lo}..{hi}"
                );
                f += 1_000.0;
            }
            // And nothing is searched outside the range's own span plus a slice.
            for &c in &centers {
                assert!(c > lo - SPAN && c < hi + SPAN, "slice {c} is nowhere near {lo}..{hi}");
            }
        }
    }

    #[test]
    fn a_backwards_or_empty_range_has_no_slices() {
        assert!(slice_centers(146e6, 144e6, SPAN).is_empty());
        assert!(slice_centers(144e6, 144e6, SPAN).is_empty());
        assert!(slice_centers(144e6, 146e6, 0.0).is_empty());
    }

    /// A spectrum with one signal in it: `n` bins of noise floor with a carrier
    /// of `width` bins at `at`.
    fn spectrum(n: usize, floor: f32, at: usize, width: usize, peak: f32) -> Vec<f32> {
        let mut v = vec![floor; n];
        for b in v.iter_mut().skip(at).take(width) {
            *b = peak;
        }
        v
    }

    #[test]
    fn a_carrier_is_found_on_the_channel_grid() {
        let n = 4096;
        // 12.5 kHz above the centre, at a 1.536 MHz span: 375 Hz per bin.
        let at = n / 2 + 33;
        let bins = spectrum(n, -110.0, at, 20, -60.0);
        let found =
            busy_channels(&bins, 145_000_000.0, SPAN, 144e6, 146e6, 12_500.0, 12_500.0, -70.0);
        assert_eq!(found.len(), 1, "one signal, one channel: {found:?}");
        assert!(
            (found[0] - 145_012_500.0).abs() < 1.0,
            "snapped to {} rather than 145.0125 MHz",
            found[0]
        );
    }

    #[test]
    fn a_quiet_band_yields_nothing() {
        let bins = vec![-110.0f32; 4096];
        assert!(
            busy_channels(&bins, 145e6, SPAN, 144e6, 146e6, 12_500.0, 12_500.0, -70.0).is_empty()
        );
    }

    /// The point of measuring channel power rather than peak bin height: a wide
    /// signal and a narrow one of the same total power have to read the same, or
    /// one threshold cannot serve every mode.
    #[test]
    fn wide_and_narrow_signals_of_equal_power_read_alike() {
        let n = 4096;
        let narrow = spectrum(n, -140.0, n / 2 + 200, 4, -60.0);
        // Ten times the bins at a tenth the power each: same channel total.
        let wide = spectrum(n, -140.0, n / 2 + 200, 40, -70.0);
        let args = (145e6, SPAN, 100e6, 200e6, 20_000.0, 5_000.0);
        for t in [-56.0f32, -54.0] {
            let a = busy_channels(&narrow, args.0, args.1, args.2, args.3, args.4, args.5, t);
            let b = busy_channels(&wide, args.0, args.1, args.2, args.3, args.4, args.5, t);
            assert_eq!(a.is_empty(), b.is_empty(), "threshold {t}: {a:?} vs {b:?}");
        }
    }

    /// Zero-IF front ends leave their own LO leakage at the centre of the span.
    /// It is loud, it is always there, and it is not a station.
    #[test]
    fn lo_leakage_at_dc_is_not_a_station() {
        let n = 4096;
        let bins = spectrum(n, -110.0, n / 2 - 2, 5, -30.0);
        let found = busy_channels(&bins, 145e6, SPAN, 144e6, 146e6, 12_500.0, 12_500.0, -70.0);
        assert!(found.is_empty(), "the DC spike was reported as {found:?}");
    }

    /// Anything past the usable window is in the roll-off, where levels cannot
    /// be trusted — the neighbouring slice covers it properly.
    #[test]
    fn signals_outside_the_usable_window_are_left_to_the_next_slice() {
        let n = 4096;
        // 45 % of the span out, past the 40 % the sweep trusts.
        let at = n / 2 + (n as f64 * 0.45) as usize;
        let bins = spectrum(n, -110.0, at, 20, -60.0);
        let found = busy_channels(&bins, 145e6, SPAN, 100e6, 200e6, 12_500.0, 12_500.0, -70.0);
        assert!(found.is_empty(), "{found:?}");
    }

    /// A channel list on one band is one tune and one transform, however many
    /// channels it holds — the whole point of scanning memories off the FFT
    /// (issue #228).
    #[test]
    fn a_list_inside_one_window_is_a_single_tune() {
        // The 2 m repeater outputs, 25 kHz apart: 800 kHz of list inside a
        // 1.536 MHz window.
        let freqs: Vec<f64> = (0..33).map(|i| 145_600_000.0 + i as f64 * 25_000.0).collect();
        let slices = memory_slices(&freqs, SPAN);
        assert_eq!(slices.len(), 1, "{} tunes for one band's worth", slices.len());
        assert_eq!(slices[0].1.len(), freqs.len(), "every channel is in it");
        // And centred on what it holds, so nothing sits in the roll-off.
        let (lo, hi) = (freqs[0], freqs[freqs.len() - 1]);
        assert!((slices[0].0 - (lo + hi) / 2.0).abs() < 1.0, "centre {}", slices[0].0);
    }

    /// A list spread across bands costs one tune per band, which is what it
    /// must: the receiver cannot be in two places at once. Every channel ends
    /// up in exactly one slice, and inside its usable window.
    #[test]
    fn every_channel_lands_in_one_slice_and_inside_it() {
        let freqs = vec![
            121_500_000.0, // airband
            145_500_000.0, // 2 m
            145_600_000.0,
            156_800_000.0, // marine 16
            156_825_000.0,
            433_500_000.0, // 70 cm
        ];
        let slices = memory_slices(&freqs, SPAN);
        let mut seen: Vec<usize> = slices.iter().flat_map(|(_, g)| g.iter().copied()).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..freqs.len()).collect::<Vec<_>>(), "{slices:?}");
        let usable = SPAN * USABLE_HALF;
        for (c, group) in &slices {
            for &i in group {
                assert!((freqs[i] - c).abs() <= usable, "{} is in the roll-off of {c}", freqs[i]);
            }
        }
        // Four bands, four tunes — not six visits.
        assert_eq!(slices.len(), 4, "{slices:?}");
    }

    #[test]
    fn nothing_to_cover_is_no_slices() {
        assert!(memory_slices(&[], SPAN).is_empty());
        assert!(memory_slices(&[145e6], 0.0).is_empty());
        assert!(memory_slices(&[f64::NAN], SPAN).is_empty());
    }

    /// The sweep's own test: of a list read off one transform, only the
    /// channels that are actually on the air are worth a dwell.
    #[test]
    fn only_the_busy_channels_come_back() {
        let n = 4096;
        let bin_hz = SPAN / n as f64;
        // A carrier 100 bins above the centre, 20 bins wide.
        let bins = spectrum(n, -110.0, n / 2 + 100, 20, -60.0);
        let busy_hz = 145e6 + 100.0 * bin_hz;
        let freqs = vec![busy_hz, 145e6 + 300.0 * bin_hz, 145e6 - 250.0 * bin_hz];
        let bw = vec![16_000.0; 3];
        assert_eq!(busy_memories(&bins, 145e6, SPAN, &freqs, &bw, -70.0), vec![0]);
        // Raise the bar past the carrier and nothing is worth stopping for.
        assert!(busy_memories(&bins, 145e6, SPAN, &freqs, &bw, -20.0).is_empty());
    }

    /// The two channels the transform cannot answer for — one out in the
    /// anti-alias roll-off, one sitting on a zero-IF front end's LO leakage —
    /// go to the dwell rather than being judged from a reading that means
    /// nothing. A wasted settling time is the right price for never missing a
    /// channel the operator stored by name.
    #[test]
    fn a_reading_that_means_nothing_is_left_to_the_receiver() {
        let n = 4096;
        let bin_hz = SPAN / n as f64;
        let quiet = vec![-140.0f32; n];
        // Past the 40% of the span the sweep trusts.
        let far = 145e6 + SPAN * 0.45;
        // And one right on the centre, where the LO leakage lives.
        let dc = 145e6 + bin_hz;
        let freqs = vec![far, dc, 145e6 + 300.0 * bin_hz];
        let bw = vec![16_000.0; 3];
        assert_eq!(busy_memories(&quiet, 145e6, SPAN, &freqs, &bw, -70.0), vec![0, 1]);
        // With no spectrum at all to read, every channel is still a channel.
        assert_eq!(busy_memories(&[], 145e6, SPAN, &freqs, &bw, -70.0), vec![0, 1, 2]);
    }

    #[test]
    fn channel_power_reads_a_carrier_and_a_quiet_channel_apart() {
        let n = 4096;
        let bins = spectrum(n, -120.0, n / 2 + 100, 8, -50.0);
        let on = channel_power_db(&bins, 145e6, SPAN, 145e6 + 100.0 * (SPAN / n as f64), 20_000.0)
            .expect("in span");
        let off =
            channel_power_db(&bins, 145e6, SPAN, 145e6 - 300_000.0, 20_000.0).expect("in span");
        assert!(on > off + 40.0, "carrier {on:.1} dB vs floor {off:.1} dB");
        assert_eq!(channel_power_db(&bins, 145e6, SPAN, 150e6, 20_000.0), None, "outside the span");
    }

    /// Candidates outside the operator's range are none of the scan's business,
    /// even when the front end can hear them.
    #[test]
    fn candidates_outside_the_asked_for_range_are_dropped() {
        let n = 4096;
        let bins = spectrum(n, -110.0, n / 2 + 300, 20, -60.0);
        let inside = busy_channels(&bins, 145e6, SPAN, 144e6, 146e6, 12_500.0, 12_500.0, -70.0);
        assert_eq!(inside.len(), 1);
        let outside = busy_channels(&bins, 145e6, SPAN, 144e6, 145e6, 12_500.0, 12_500.0, -70.0);
        assert!(outside.is_empty(), "{outside:?} is above the range's top");
    }

    #[test]
    fn a_panorama_is_stitched_from_its_slices() {
        // 0..4 MHz from a 2 MHz front end: three slices; a carrier at 1.3 MHz
        // in one of them, a noise floor everywhere.
        let (lo, hi) = (0.0, 4e6);
        let n = 1024;
        let m = panorama_bins(lo, hi, SPAN_PANO, n);
        assert_eq!(m, 2048, "the front end's own resolution");
        let mut pano = vec![f32::NAN; m];
        for c in slice_centers(lo, hi, SPAN_PANO) {
            let mut bins = vec![-100.0f32; n];
            let carrier = 1.3e6;
            if (carrier - c).abs() < SPAN_PANO / 2.0 {
                let i = ((carrier - (c - SPAN_PANO / 2.0)) / (SPAN_PANO / n as f64)) as usize;
                bins[i] = -40.0;
            }
            // The front end's own DC spike, which is not a station.
            bins[n / 2] = -20.0;
            stitch_slice(&mut pano, lo, hi, &bins, c, SPAN_PANO);
        }
        fill_panorama_gaps(&mut pano);
        assert!(pano.iter().all(|v| v.is_finite()), "no holes left");
        let at = |hz: f64| pano[((hz - lo) / (hi - lo) * m as f64) as usize];
        assert!(at(1.3e6) > -45.0, "the carrier is in the panorama: {}", at(1.3e6));
        assert!(pano.iter().all(|&v| v < -30.0), "no DC spike stitched in");
        assert!((at(3.5e6) - -100.0).abs() < 0.5, "the floor elsewhere");
    }

    #[test]
    fn a_wide_panorama_bin_pools_the_front_ends_bins() {
        // One panorama bin over the whole usable part: the mean power.
        let mut pano = vec![f32::NAN; 1];
        let mut bins = vec![-100.0f32; 100];
        bins[20] = -70.0; // 1000× the floor in one bin of the ~80 it pools
        stitch_slice(&mut pano, 1.1e6, 1.9e6, &bins, 1.5e6, 1e6);
        assert!(pano[0] > -90.0 && pano[0] < -80.0, "{}", pano[0]);
    }

    const SPAN_PANO: f64 = 2e6;
}
