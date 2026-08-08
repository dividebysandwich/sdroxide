//! End-to-end DSP chain tests: DDC frequency placement, SSB demod purity,
//! filter stopband rejection, AGC convergence, resampler ratio.

use num_complex::Complex;
use sdroxide_dsp::{
    Agc, ComplexDcBlock, ComplexFir, Ddc, Duc, MonoResampler, bandpass_taps, channel_target,
    make_demod, make_modulator,
};
use sdroxide_types::{AgcMode, Mode};

type C32 = Complex<f32>;

fn tone(rate: f64, freq: f64, amp: f32, n: usize) -> Vec<C32> {
    (0..n)
        .map(|i| {
            let ph = std::f64::consts::TAU * freq * i as f64 / rate;
            C32::new((ph.cos() * amp as f64) as f32, (ph.sin() * amp as f64) as f32)
        })
        .collect()
}

/// Mean instantaneous frequency from the phase slope.
fn mean_freq(x: &[C32], rate: f64) -> f64 {
    let sum: f64 = x.windows(2).map(|w| (w[1] * w[0].conj()).arg() as f64).sum();
    sum / (x.len() - 1) as f64 * rate / std::f64::consts::TAU
}

/// Goertzel power of a real signal at one frequency.
fn goertzel(x: &[f32], freq: f64, rate: f64) -> f64 {
    let w = std::f64::consts::TAU * freq / rate;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v as f64 + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2) / (x.len() as f64 * x.len() as f64 / 4.0)
}

#[test]
fn ddc_exact_48k_and_tone_placement() {
    let in_rate = 1_536_000.0;
    let mut ddc = Ddc::new(in_rate, 48_000.0);
    assert!((ddc.out_rate() - 48_000.0).abs() < 1e-9, "out_rate = {}", ddc.out_rate());

    // Signal 100 kHz above hardware center; VFO tuned 95 kHz above → 5 kHz IF.
    ddc.set_offset_hz(95_000.0);
    let input = tone(in_rate, 100_000.0, 0.5, (in_rate * 0.4) as usize);
    let mut out = Vec::new();
    for chunk in input.chunks(16_384) {
        ddc.process(chunk, &mut out);
    }
    assert!(out.len() > 10_000);
    let tail = &out[out.len() / 2..];
    let f = mean_freq(tail, ddc.out_rate());
    assert!((f - 5_000.0).abs() < 5.0, "measured {f} Hz");
}

#[test]
fn ddc_odd_factor_rates() {
    // 2 Msps (HackRF style): halfbands to 250 kHz, then /5 → 50 kHz.
    let ddc = Ddc::new(2_000_000.0, 48_000.0);
    assert!((ddc.out_rate() - 50_000.0).abs() < 1e-6, "out_rate = {}", ddc.out_rate());
}

#[test]
fn ssb_demod_recovers_clean_tone() {
    let rate = 48_000.0;
    let mut demod = make_demod(Mode::Usb, rate).unwrap();
    // Carrier at DC, audio tone at +1.5 kHz (inside the 150–2850 passband).
    let input = tone(rate, 1_500.0, 0.5, 48_000);
    let mut audio = Vec::new();
    for chunk in input.chunks(1024) {
        demod.process(chunk, &mut audio);
    }
    let tail = &audio[audio.len() / 2..];
    let wanted = goertzel(tail, 1_500.0, rate);
    let total: f64 = tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
    // Nearly all energy at the wanted tone (tone power = A²/2 ≈ goertzel amp²).
    assert!(wanted > 0.5 * total, "wanted {wanted}, total {total}");
    assert!(total > 0.1, "audio too quiet: {total}");
}

#[test]
fn ssb_filter_rejects_opposite_sideband() {
    let rate = 48_000.0;
    let taps = bandpass_taps(331, 150.0, 2850.0, rate);
    let mut fir = ComplexFir::new(taps);

    let inband = tone(rate, 1_500.0, 1.0, 24_000);
    let mut out = Vec::new();
    fir.process(&inband, &mut out);
    let p_pass: f32 =
        out[out.len() / 2..].iter().map(|z| z.norm_sqr()).sum::<f32>() / (out.len() / 2) as f32;

    let mut fir2 = ComplexFir::new(bandpass_taps(331, 150.0, 2850.0, rate));
    let stop = tone(rate, -1_500.0, 1.0, 24_000);
    let mut out2 = Vec::new();
    fir2.process(&stop, &mut out2);
    let p_stop: f32 =
        out2[out2.len() / 2..].iter().map(|z| z.norm_sqr()).sum::<f32>() / (out2.len() / 2) as f32;

    let rejection_db = 10.0 * (p_stop / p_pass).log10();
    assert!(rejection_db < -60.0, "opposite sideband only {rejection_db:.1} dB down");
}

#[test]
fn agc_levels_weak_and_strong_signals() {
    let rate = 48_000.0;
    let mut agc = Agc::new(rate);
    agc.set_mode(AgcMode::Med);
    agc.set_max_gain_db(90.0);

    let run = |agc: &mut Agc, amp: f32, secs: f64| -> f32 {
        let n = (rate * secs) as usize;
        let mut peak_tail = 0.0f32;
        let tail_start = n * 3 / 4;
        for start in (0..n).step_by(512) {
            let mut block: Vec<f32> = (start..(start + 512).min(n))
                .map(|i| amp * (std::f64::consts::TAU * 1000.0 * i as f64 / rate).sin() as f32)
                .collect();
            agc.process(&mut block);
            if start >= tail_start {
                for &v in &block {
                    peak_tail = peak_tail.max(v.abs());
                }
            }
        }
        peak_tail
    };

    // Weak signal amplified toward the target…
    let weak = run(&mut agc, 0.001, 3.0);
    assert!(weak > 0.1 && weak < 0.8, "weak → {weak}");
    // …strong signal held near the same level (not clipping).
    let strong = run(&mut agc, 0.8, 3.0);
    assert!(strong > 0.1 && strong < 0.8, "strong → {strong}");
}

#[test]
fn wfm_demod_recovers_tone_without_dc() {
    let rate = 256_000.0;
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    assert!((demod.audio_rate() - 64_000.0).abs() < 1e-6);

    // FM: 1 kHz tone at ±60 kHz deviation, carrier off-tuned by +10 kHz —
    // the DC blocker must absorb the off-tune offset.
    let n = (rate * 1.5) as usize;
    let mut phase = 0.0f64;
    let input: Vec<C32> = (0..n)
        .map(|i| {
            let t = i as f64 / rate;
            let inst = 10_000.0 + 60_000.0 * (std::f64::consts::TAU * 1_000.0 * t).cos();
            phase += std::f64::consts::TAU * inst / rate;
            C32::new(phase.cos() as f32, phase.sin() as f32)
        })
        .collect();

    let mut audio = Vec::new();
    for chunk in input.chunks(8_192) {
        demod.process(chunk, &mut audio);
    }
    let tail = &audio[audio.len() / 2..];
    let mean: f64 = tail.iter().map(|&v| v as f64).sum::<f64>() / tail.len() as f64;
    assert!(mean.abs() < 0.02, "residual DC {mean}");

    let wanted = goertzel(tail, 1_000.0, demod.audio_rate());
    let total: f64 = tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
    assert!(wanted > 0.5 * total, "wanted {wanted}, total {total}");
    // Never anywhere near clipping despite 80% deviation.
    let peak = tail.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    assert!(peak < 0.9, "peak {peak}");
    assert!(total > 0.01, "audio too quiet: {total}");
}

#[test]
fn ssb_modulator_places_single_sideband() {
    let rate = 48_000.0;
    let mut m = make_modulator(Mode::Usb, rate, Mode::Usb.default_filter()).unwrap();
    let audio: Vec<f32> = (0..48_000)
        .map(|i| (std::f64::consts::TAU * 1_000.0 * i as f64 / rate).sin() as f32)
        .collect();
    let mut out = Vec::new();
    for chunk in audio.chunks(1024) {
        m.process(chunk, &mut out);
    }
    // USB: the complex baseband tone must sit at +1 kHz.
    let tail = &out[out.len() / 2..];
    let f = mean_freq(tail, rate);
    assert!((f - 1_000.0).abs() < 5.0, "USB tone at {f} Hz");

    let mut m = make_modulator(Mode::Lsb, rate, Mode::Lsb.default_filter()).unwrap();
    let mut out = Vec::new();
    for chunk in audio.chunks(1024) {
        m.process(chunk, &mut out);
    }
    let tail = &out[out.len() / 2..];
    let f = mean_freq(tail, rate);
    assert!((f + 1_000.0).abs() < 5.0, "LSB tone at {f} Hz");
}

#[test]
fn ssb_tx_rx_loopback() {
    let rate = 48_000.0;
    let mut m = make_modulator(Mode::Usb, rate, Mode::Usb.default_filter()).unwrap();
    let mut d = make_demod(Mode::Usb, rate).unwrap();
    let audio: Vec<f32> = (0..48_000)
        .map(|i| 0.5 * (std::f64::consts::TAU * 1_500.0 * i as f64 / rate).sin() as f32)
        .collect();
    let mut baseband = Vec::new();
    for chunk in audio.chunks(1024) {
        m.process(chunk, &mut baseband);
    }
    let mut rx_audio = Vec::new();
    for chunk in baseband.chunks(1024) {
        d.process(chunk, &mut rx_audio);
    }
    let tail = &rx_audio[rx_audio.len() / 2..];
    let wanted = goertzel(tail, 1_500.0, rate);
    let total: f64 = tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
    assert!(wanted > 0.5 * total, "loopback wanted {wanted}, total {total}");
    assert!(total > 0.01, "loopback too quiet: {total}");
}

#[test]
fn duc_interpolates_and_preserves_frequency() {
    let mut duc = Duc::new(48_000.0, 1_536_000.0);
    // 5 kHz baseband tone.
    let input = tone(48_000.0, 5_000.0, 0.5, 48_000);
    let mut out = Vec::new();
    for chunk in input.chunks(1024) {
        duc.process(chunk, &mut out);
    }
    let ratio = out.len() as f64 / input.len() as f64;
    assert!((ratio - 32.0).abs() < 0.5, "interpolation ratio {ratio}");
    let tail = &out[out.len() / 2..];
    let f = mean_freq(tail, 1_536_000.0);
    assert!((f - 5_000.0).abs() < 20.0, "tone moved to {f} Hz");
    // No significant images: peak magnitude close to input amplitude.
    let peak = tail.iter().fold(0.0f32, |a, z| a.max(z.norm()));
    assert!(peak < 0.7, "interpolation overshoot: {peak}");
}

#[test]
fn resampler_ratio_direction() {
    let mut rs = MonoResampler::new(50_000.0, 48_000.0).unwrap();
    let input: Vec<f32> = (0..50_000)
        .map(|i| (std::f64::consts::TAU * 1000.0 * i as f64 / 50_000.0).sin() as f32)
        .collect();
    let mut out = Vec::new();
    rs.push(&input, &mut out);
    let ratio = out.len() as f64 / input.len() as f64;
    assert!(
        (0.90..=0.99).contains(&ratio),
        "1 s at 50 kHz should give ~0.96 s at 48 kHz, got ratio {ratio}"
    );
    assert!(MonoResampler::new(48_000.0, 48_000.0).is_none());
}

/// FM-modulate a stereo multiplex carrying `left` and `right` (both 1 kHz-family
/// tones given as closures of time) and return channel-rate IQ.
///
/// `mpx = 0.9·[M + S·cos ω₃₈t] + 0.1·cos ω₁₉t`, with `M = (L+R)/2`,
/// `S = (L−R)/2` — the broadcast standard's deviation split, so the recovered
/// levels come out where a real station's would.
fn stereo_mpx_iq(
    rate: f64,
    n: usize,
    dev_hz: f64,
    pilot: bool,
    left: impl Fn(f64) -> f64,
    right: impl Fn(f64) -> f64,
) -> Vec<C32> {
    let mut phase = 0.0f64;
    (0..n)
        .map(|i| {
            let t = i as f64 / rate;
            let (l, r) = (left(t), right(t));
            let (m, s) = ((l + r) / 2.0, (l - r) / 2.0);
            // Sine phase throughout, as the broadcast standard specifies —
            // the subcarrier is the pilot's second harmonic. Writing both as
            // cosine is self-consistent and silently validates a decoder whose
            // subcarrier is a quarter turn out.
            let mpx = if pilot {
                0.9 * (m + s * (std::f64::consts::TAU * 38_000.0 * t).sin())
                    + 0.1 * (std::f64::consts::TAU * 19_000.0 * t).sin()
            } else {
                m
            };
            phase += std::f64::consts::TAU * dev_hz * mpx / rate;
            C32::new(phase.cos() as f32, phase.sin() as f32)
        })
        .collect()
}

/// Run a WFM demod over `iq` and return the matrixed (left, right) audio.
fn wfm_stereo(demod: &mut Box<dyn sdroxide_dsp::Demodulator>, iq: &[C32]) -> (Vec<f32>, Vec<f32>) {
    let (mut m, mut s) = (Vec::new(), Vec::new());
    for chunk in iq.chunks(8_192) {
        let before = m.len();
        demod.process(chunk, &mut m);
        let produced = m.len() - before;
        let had = s.len();
        if !demod.take_side(&mut s) {
            // Mono block: the difference is zero for exactly as many samples.
            s.resize(had + produced, 0.0);
        }
        assert_eq!(m.len(), s.len(), "sum and difference must stay sample-aligned");
    }
    let l = m.iter().zip(&s).map(|(a, b)| a + b).collect();
    let r = m.iter().zip(&s).map(|(a, b)| a - b).collect();
    (l, r)
}

/// Channel separation (dB) for a hard-left 1 kHz tone at `rate`.
fn wfm_separation(rate: f64) -> f64 {
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    // Long enough that the PLL has acquired *and* the 200 ms blend smoother has
    // converged; measuring across the ramp reads far lower than the steady state.
    let n = (rate * 6.0) as usize;
    let iq = stereo_mpx_iq(
        rate,
        n,
        75_000.0,
        true,
        |t| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
        |_| 0.0,
    );
    let (l, r) = wfm_stereo(&mut demod, &iq);
    assert!(demod.stereo_locked(), "pilot never locked at {rate} Hz");
    assert!(demod.stereo_blend() > 0.99, "blend only {} at {rate} Hz", demod.stereo_blend());

    let tail = l.len() * 3 / 4;
    let ar = demod.audio_rate();
    let pl = goertzel(&l[tail..], 1_000.0, ar);
    let pr = goertzel(&r[tail..], 1_000.0, ar);
    10.0 * (pl / pr.max(1e-30)).log10()
}

#[test]
fn wfm_stereo_separates_hard_panned_channels() {
    // Measured ~51 dB at 256/240 kHz; asserted well below that so ordinary
    // retuning of the filters can't trip it, but far above the ~12 dB a
    // one-sample subcarrier phase slip would produce.
    for rate in [256_000.0, 240_000.0] {
        let sep = wfm_separation(rate);
        assert!(sep >= 40.0, "separation only {sep:.1} dB at {rate} Hz");
    }
    // 192 kHz (HPSDR/TCI) is floored by Carson truncation, not by the filters:
    // a 2·(75+53) = 256 kHz signal does not fit, and no amount of taps helps.
    let sep = wfm_separation(192_000.0);
    assert!(sep >= 30.0, "separation only {sep:.1} dB at 192 kHz");
}

#[test]
fn wfm_never_locks_on_a_dead_frequency() {
    // Pure noise, no station at all. `|i_lp|` alone hovers around 0.015 here —
    // above any threshold low enough to catch an under-injected pilot — so this
    // is what forces the lock detector onto a *signed* average.
    let rate = 256_000.0;
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    let mut seed = 0x5EEDu64;
    let mut rnd = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as f32 / (1u32 << 31) as f32) - 1.0
    };
    let iq: Vec<C32> = (0..(rate * 3.0) as usize).map(|_| C32::new(rnd(), rnd())).collect();
    let (mut m, mut s) = (Vec::new(), Vec::new());
    for chunk in iq.chunks(8_192) {
        demod.process(chunk, &mut m);
        assert!(!demod.take_side(&mut s), "emitted a difference channel from noise");
    }
    assert!(!demod.stereo_locked(), "declared stereo lock on a dead frequency");
}

#[test]
fn wfm_sum_and_side_stay_sample_aligned() {
    // The whole matrix rests on `lpf_m` and `lpf_s` keeping identical
    // decimation phase. Ragged, co-prime block sizes are what would expose drift.
    let rate = 256_000.0;
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    let iq = stereo_mpx_iq(
        rate,
        (rate * 4.0) as usize,
        75_000.0,
        true,
        |t| 0.6 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
        |t| 0.6 * (std::f64::consts::TAU * 3_000.0 * t).sin(),
    );
    let (mut m, mut s) = (Vec::new(), Vec::new());
    let mut at = 0usize;
    for step in [997usize, 1013, 8191, 64, 4099].iter().cycle() {
        if at >= iq.len() {
            break;
        }
        let end = (at + step).min(iq.len());
        let before = m.len();
        demod.process(&iq[at..end], &mut m);
        let produced = m.len() - before;
        let had = s.len();
        if demod.take_side(&mut s) {
            assert_eq!(
                s.len() - had,
                produced,
                "side produced {} for {produced} sum",
                s.len() - had
            );
        }
        at = end;
    }
    assert!(!m.is_empty());
}

#[test]
fn wfm_stereo_level_matches_mono() {
    let rate = 256_000.0;
    let n = (rate * 2.0) as usize;
    let sig = |t: f64| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin();

    // Identical L and R: the difference channel is zero, so both ears must come
    // out at exactly the level the mono path produces for the same programme.
    let mut st = make_demod(Mode::Wfm, rate).unwrap();
    let (l, r) = wfm_stereo(&mut st, &stereo_mpx_iq(rate, n, 75_000.0, true, sig, sig));
    assert!(st.stereo_locked());

    // Same broadcast, decoded with stereo forced off — the honest comparison.
    // (A genuinely mono station is a further 0.9 dB louder, because it spends
    // the pilot's and subcarrier's share of the deviation on programme instead.)
    let mut mono = make_demod(Mode::Wfm, rate).unwrap();
    mono.set_stereo_enabled(false);
    let (ml, mr) = wfm_stereo(&mut mono, &stereo_mpx_iq(rate, n, 75_000.0, true, sig, sig));
    assert!(ml.iter().zip(&mr).all(|(a, b)| a == b), "forced mono produced a difference");

    let ar = st.audio_rate();
    let tail = l.len() / 2;
    let (pl, pr) = (goertzel(&l[tail..], 1_000.0, ar), goertzel(&r[tail..], 1_000.0, ar));
    let pm = goertzel(&ml[ml.len() / 2..], 1_000.0, ar);

    // Stereo L/R vs the same signal decoded mono, in dB — no level jump when
    // the pilot locks, which is what makes the blend inaudible.
    for (name, p) in [("L", pl), ("R", pr)] {
        let d = 10.0 * (p / pm).log10();
        assert!(d.abs() < 0.5, "{name} is {d:.2} dB off the mono path");
    }
}

#[test]
fn wfm_mono_signal_never_fakes_stereo() {
    let rate = 256_000.0;
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    let n = (rate * 2.0) as usize;
    let iq = stereo_mpx_iq(
        rate,
        n,
        75_000.0,
        false,
        |t| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
        |t| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
    );
    let (l, r) = wfm_stereo(&mut demod, &iq);
    assert!(!demod.stereo_locked(), "declared lock with no pilot present");
    assert!(l.iter().zip(&r).all(|(a, b)| a == b), "channels differ with no pilot");
}

#[test]
fn wfm_stereo_honours_the_mono_override() {
    let rate = 256_000.0;
    let n = (rate * 2.0) as usize;
    let iq = stereo_mpx_iq(
        rate,
        n,
        75_000.0,
        true,
        |t| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
        |_| 0.0,
    );
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    demod.set_stereo_enabled(false);
    let (l, r) = wfm_stereo(&mut demod, &iq);
    // The pilot is still tracked (so the indicator is honest) but no difference
    // signal reaches the output.
    assert!(demod.stereo_locked(), "pilot tracking should continue while forced to mono");
    assert!(l.iter().zip(&r).all(|(a, b)| a == b), "forced mono still produced a difference");
}

#[test]
fn wfm_stereo_needs_enough_channel_rate() {
    // 96 kHz cannot carry a 53 kHz composite; stereo must not be attempted.
    let mut demod = make_demod(Mode::Wfm, 96_000.0).unwrap();
    let iq = stereo_mpx_iq(96_000.0, 96_000, 75_000.0, true, |_| 0.0, |_| 0.0);
    let (mut m, mut s) = (Vec::new(), Vec::new());
    demod.process(&iq, &mut m);
    assert!(!demod.take_side(&mut s));
    assert!(!demod.stereo_locked());
}

#[test]
fn real_fir_decim_matches_filter_then_stride() {
    use sdroxide_dsp::{RealFir, RealFirDecim};
    let (rate, taps, factor) = (256_000.0, 383, 4);
    let mut a = RealFirDecim::new(taps, 15_000.0, rate, factor);
    let mut b = RealFir::lowpass(taps, 15_000.0, rate);

    // Deterministic pseudo-random input, fed in ragged blocks so the streaming
    // state (not just a single pass) is what's under test.
    let x: Vec<f32> =
        (0..20_000).map(|i| ((i as f32 * 12.9898).sin() * 43758.547).fract() * 2.0 - 1.0).collect();
    let (mut da, mut full) = (Vec::new(), Vec::new());
    for chunk in x.chunks(1_777) {
        a.process(chunk, &mut da);
        b.process(chunk, &mut full);
    }
    let strided: Vec<f32> = full.iter().step_by(factor).copied().collect();
    assert_eq!(da.len(), strided.len());
    for (i, (p, q)) in da.iter().zip(&strided).enumerate() {
        assert!((p - q).abs() < 1e-5, "sample {i}: {p} vs {q}");
    }
}

#[test]
fn agc_pair_preserves_the_channel_ratio() {
    let mut agc = Agc::new(48_000.0);
    agc.set_mode(AgcMode::Med);

    // Side is a fixed fraction of main; that ratio is the stereo image, and the
    // AGC must not touch it however hard the gain moves.
    let mut main: Vec<f32> = (0..48_000)
        .map(|i| {
            let env = if i < 24_000 { 0.02 } else { 0.9 }; // big level step mid-way
            env * (std::f32::consts::TAU * 1_000.0 * i as f32 / 48_000.0).sin()
        })
        .collect();
    let mut side: Vec<f32> = main.iter().map(|s| s * 0.25).collect();
    agc.process_pair(&mut main, &mut side);

    for (i, (m, s)) in main.iter().zip(&side).enumerate() {
        if m.abs() < 1e-6 {
            continue;
        }
        let ratio = s / m;
        assert!((ratio - 0.25).abs() < 1e-4, "sample {i}: ratio drifted to {ratio}");
    }

    // AGC off is a fixed manual gain, not a bypass: one gain on both channels
    // (so the image survives there too) and the same lookahead delay, which is
    // what keeps switching the AGC on and off free of jumps.
    let mut agc = Agc::new(48_000.0);
    agc.set_manual_gain_db(20.0);
    agc.set_mode(AgcMode::Off);
    let (mut a, mut b) = (vec![0.03f32; 4_800], vec![0.01f32; 4_800]);
    agc.process_pair(&mut a, &mut b);
    for (i, (m, s)) in a.iter().zip(&b).enumerate().skip(2_000) {
        assert!((m - 0.3).abs() < 1e-5, "sample {i}: main {m}, wanted 0.3");
        assert!((s - 0.1).abs() < 1e-5, "sample {i}: side {s}, wanted 0.1");
    }
}

/// Switching the AGC off must not switch the receiver off with it. Unlevelled,
/// the demodulator hands over whatever the band gave it — a weak SSB signal
/// tens of dB below anything audible — so "off" applies a fixed gain instead,
/// seeded from where the AGC had settled.
#[test]
fn agc_off_holds_the_level_it_was_switched_off_at() {
    let rate = 48_000.0;
    let run = |agc: &mut Agc, amp: f32, secs: f64| -> f32 {
        let n = (rate * secs) as usize;
        let mut peak_tail = 0.0f32;
        let tail_start = n * 3 / 4;
        for start in (0..n).step_by(512) {
            let mut block: Vec<f32> = (start..(start + 512).min(n))
                .map(|i| amp * (std::f64::consts::TAU * 1000.0 * i as f64 / rate).sin() as f32)
                .collect();
            agc.process(&mut block);
            if start >= tail_start {
                for &v in &block {
                    peak_tail = peak_tail.max(v.abs());
                }
            }
        }
        peak_tail
    };

    let mut agc = Agc::new(rate);
    agc.set_mode(AgcMode::Med);
    agc.set_max_gain_db(90.0);
    // A signal 60 dB below full scale, levelled up to the AGC's target.
    let levelled = run(&mut agc, 0.001, 3.0);
    assert!(levelled > 0.1, "AGC left the weak signal at {levelled}");

    // Switch off the way the engine does: carry the tracked gain over.
    agc.set_manual_gain_db(agc.gain_db());
    agc.set_mode(AgcMode::Off);
    let held = run(&mut agc, 0.001, 1.0);
    let step_db = 20.0 * (held / levelled).log10();
    assert!(step_db.abs() < 1.0, "level jumped {step_db:.1} dB when the AGC went off");

    // And it stays fixed: 20 dB more signal now really is 20 dB more audio,
    // which is the whole point of turning the levelling off.
    let louder = run(&mut agc, 0.01, 1.0);
    let rise_db = 20.0 * (louder / held).log10();
    assert!((rise_db - 20.0).abs() < 0.5, "manual gain moved: {rise_db:.1} dB for a 20 dB step");
}

/// Broadcast-like programme: many tones across the band at full modulation,
/// which is what a compressed music station actually looks like. A single test
/// tone is far too kind to a pilot-SNR estimate — with dense programme, an
/// estimate that accidentally measures the music instead of the noise collapses
/// the blend and every real station plays mono with the pilot still locked.
fn dense_programme(t: f64, right: bool) -> f64 {
    let mut v = if right { 0.0 } else { 0.35 * (std::f64::consts::TAU * 1_000.0 * t).sin() };
    for (f, a) in [(220.0, 0.22), (740.0, 0.18), (2_600.0, 0.15), (5_100.0, 0.12), (9_300.0, 0.10)]
    {
        let f = if right { f * 1.31 } else { f };
        v += a * (std::f64::consts::TAU * f * t).sin();
    }
    v
}

#[test]
fn wfm_stereo_survives_dense_programme() {
    let rate = 256_000.0;
    let mut demod = make_demod(Mode::Wfm, rate).unwrap();
    let iq = stereo_mpx_iq(
        rate,
        (rate * 6.0) as usize,
        75_000.0,
        true,
        |t| dense_programme(t, false),
        |t| dense_programme(t, true),
    );
    let (l, r) = wfm_stereo(&mut demod, &iq);
    assert!(demod.stereo_locked(), "pilot never locked on dense programme");
    assert!(
        demod.stereo_blend() > 0.9,
        "blend collapsed to {} on a clean but busy station",
        demod.stereo_blend()
    );

    let tail = l.len() * 3 / 4;
    let ar = demod.audio_rate();
    let pl = goertzel(&l[tail..], 1_000.0, ar);
    let pr = goertzel(&r[tail..], 1_000.0, ar);
    let sep = 10.0 * (pl / pr.max(1e-30)).log10();
    assert!(sep >= 25.0, "separation only {sep:.1} dB on dense programme");
}

/// A zero-IF front end's DC offset lands exactly where the VFO sits, and an FM
/// discriminator — unlike every narrow demodulator — has no passband to keep it
/// out: it reads the phase of signal-plus-offset. Runs the real device-rate
/// path, DDC included, at the rate a HackRF actually settles on (2 Msps, the
/// nearest it supports to the 1.536 default).
///
/// Failure is a cliff, not a slope, and it sits at parity. While the offset is
/// smaller than the signal the phase error stays bounded by `arcsin(|C|/|A|)`
/// and only adds intermodulation; once it is larger the signal vector no longer
/// encircles the origin, the phase can never complete a turn, and the recovered
/// tone collapses by ~60×. Sweeping the ratio through this chain puts the knee
/// between 0.95 and 1.00.
///
/// Which is exactly where the hardware sits: measured on a HackRF One at
/// 93.2 MHz, the DC vector is 0.0196 against 0.0202 of wanted signal at the
/// discriminator input — a ratio of 0.97, with the station's instantaneous
/// envelope dipping under it constantly. Hence 1.2 here: just past the knee,
/// which is what a real capture spends much of its time being.
#[test]
fn wfm_needs_the_front_end_dc_blocker() {
    let dev_rate = 2_000_000.0;
    let n = (dev_rate * 0.4) as usize;
    let clean = stereo_mpx_iq(
        dev_rate,
        n,
        75_000.0,
        true,
        |t| 0.8 * (std::f64::consts::TAU * 1_000.0 * t).sin(),
        |_| 0.0,
    );
    // 1.2× the unit envelope, in the direction the hardware's offset points.
    let dc = C32::new(1.114, -0.448);
    let offset: Vec<C32> = clean.iter().map(|&z| z + dc).collect();

    let mut blocker = ComplexDcBlock::new(20.0, dev_rate);
    let mut blocked = offset.clone();
    // Block at a time, as the device layer feeds it.
    for chunk in blocked.chunks_mut(16_384) {
        blocker.process(chunk);
    }

    // Recovered 1 kHz tone as a fraction of total audio power. A clean tone
    // reads ≈2 (Goertzel returns amplitude², mean power is A²/2).
    let tone_purity = |iq: &[C32]| -> f64 {
        let mut ddc = Ddc::new(dev_rate, channel_target(Mode::Wfm));
        let mut demod = make_demod(Mode::Wfm, ddc.out_rate()).unwrap();
        let (mut channel, mut audio) = (Vec::new(), Vec::new());
        for chunk in iq.chunks(16_384) {
            channel.clear();
            ddc.process(chunk, &mut channel);
            demod.process(&channel, &mut audio);
        }
        let tail = &audio[audio.len() / 2..];
        let total: f64 =
            tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
        goertzel(tail, 1_000.0, demod.audio_rate()) / total.max(1e-30)
    };

    let (clean_p, offset_p, blocked_p) =
        (tone_purity(&clean), tone_purity(&offset), tone_purity(&blocked));
    assert!(clean_p > 1.5, "reference station is not clean to begin with: {clean_p:.3}");
    assert!(
        offset_p < 0.2,
        "a DC vector {:.2}× the envelope should stop the discriminator turning, \
         but purity only moved {clean_p:.3} -> {offset_p:.3}",
        dc.norm()
    );
    assert!(
        blocked_p > clean_p * 0.95,
        "the blocker should restore the station: {clean_p:.3} clean, \
         {offset_p:.3} offset, {blocked_p:.3} blocked"
    );
}
