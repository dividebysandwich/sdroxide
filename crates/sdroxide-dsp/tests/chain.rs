//! End-to-end DSP chain tests: DDC frequency placement, SSB demod purity,
//! filter stopband rejection, AGC convergence, resampler ratio.

use num_complex::Complex;
use sdroxide_dsp::{
    Agc, ComplexFir, Ddc, Duc, MonoResampler, bandpass_taps, make_demod, make_modulator,
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
    let sum: f64 = x
        .windows(2)
        .map(|w| (w[1] * w[0].conj()).arg() as f64)
        .sum();
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
                .map(|i| {
                    amp * (std::f64::consts::TAU * 1000.0 * i as f64 / rate).sin() as f32
                })
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
    let total: f64 =
        tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
    assert!(wanted > 0.5 * total, "wanted {wanted}, total {total}");
    // Never anywhere near clipping despite 80% deviation.
    let peak = tail.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    assert!(peak < 0.9, "peak {peak}");
    assert!(total > 0.01, "audio too quiet: {total}");
}

#[test]
fn ssb_modulator_places_single_sideband() {
    let rate = 48_000.0;
    let mut m = make_modulator(Mode::Usb, rate).unwrap();
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

    let mut m = make_modulator(Mode::Lsb, rate).unwrap();
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
    let mut m = make_modulator(Mode::Usb, rate).unwrap();
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
    let total: f64 =
        tail.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / tail.len() as f64;
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
