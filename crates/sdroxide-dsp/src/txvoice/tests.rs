use sdroxide_types::{TxEqEdge, TxVoiceStage, TxVoiceState};

use super::*;

const RATE: f64 = VOICE_CHAIN_RATE;

fn tone(freq: f64, amp: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| amp * (std::f64::consts::TAU * freq * i as f64 / RATE).sin() as f32).collect()
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|&s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// Run `audio` through a chain set to `cfg`, in 10 ms blocks like the engine.
fn run(cfg: &TxVoiceState, audio: &mut [f32]) {
    let mut chain = VoiceChain::new();
    chain.configure(cfg);
    chain.reset();
    for block in audio.chunks_mut(480) {
        chain.process(block);
    }
}

/// Every stage off is the state an operator who never opened the window is
/// in, and it must be a straight wire — not "almost the same".
#[test]
fn the_default_chain_is_a_wire() {
    let src = tone(700.0, 0.4, 4800);
    let mut out = src.clone();
    run(&TxVoiceState::default(), &mut out);
    assert_eq!(src, out);
}

#[test]
fn bypass_switches_everything_out() {
    let mut cfg = TxVoiceState::default();
    cfg.gate.enabled = true;
    cfg.comp.enabled = true;
    cfg.output.limiter = true;
    cfg.bypass = true;
    let src = tone(700.0, 0.9, 4800);
    let mut out = src.clone();
    run(&cfg, &mut out);
    assert_eq!(src, out);
}

/// The curve the UI draws comes from `TxEqState::response_db`; the audio
/// has to get the same thing.
#[test]
fn the_eq_does_what_its_curve_says() {
    let mut cfg = TxVoiceState::default();
    cfg.eq.enabled = true;
    cfg.eq.hpf = TxEqEdge { enabled: true, freq_hz: 200.0, steep: true };
    cfg.eq.mid2.gain_db = 6.0;
    cfg.eq.lpf = TxEqEdge { enabled: true, freq_hz: 3000.0, steep: false };
    for f in [80.0, 400.0, 2500.0, 5000.0] {
        let n = 24_000;
        let src = tone(f, 0.1, n);
        let mut out = src.clone();
        run(&cfg, &mut out);
        let measured = db(rms(&out[n / 2..])) - db(rms(&src[n / 2..]));
        let drawn = cfg.eq.response_db(RATE, f) as f32;
        assert!(
            (measured - drawn).abs() < 0.5,
            "{f} Hz: measured {measured:.2} dB, curve {drawn:.2} dB"
        );
    }
}

#[test]
fn a_steep_high_pass_falls_at_24_db_per_octave() {
    let mut eq = TxVoiceState::default().eq;
    eq.hpf = TxEqEdge { enabled: true, freq_hz: 200.0, steep: true };
    let at = |f| eq.response_db(RATE, f);
    assert!((at(200.0) + 3.0).abs() < 0.3, "Butterworth is -3 dB at its corner: {}", at(200.0));
    let octave = at(50.0) - at(100.0);
    assert!((octave + 24.0).abs() < 2.0, "two octaves below the corner: {octave} dB/oct");
    assert!(at(1000.0).abs() < 0.1);
}

/// Room noise in a pause goes down by the floor; a voice-level tone comes
/// through untouched once the gate is open.
#[test]
fn the_gate_closes_on_noise_and_opens_on_voice() {
    let mut cfg = TxVoiceState::default();
    cfg.gate.enabled = true;
    cfg.gate.threshold_db = -40.0;
    cfg.gate.floor_db = -20.0;
    cfg.gate.ratio = 10.0;

    let quiet = tone(300.0, 0.003, 9600); // about -50 dBFS
    let mut out = quiet.clone();
    run(&cfg, &mut out);
    let cut = db(rms(&out[4800..])) - db(rms(&quiet[4800..]));
    assert!((cut + 20.0).abs() < 1.0, "noise should sit at the floor: {cut:.1} dB");

    let loud = tone(300.0, 0.3, 9600);
    let mut out = loud.clone();
    run(&cfg, &mut out);
    let through = db(rms(&out[4800..])) - db(rms(&loud[4800..]));
    assert!(through.abs() < 0.1, "voice should pass at unity: {through:.2} dB");
}

/// With the look-ahead the gate is already open when a burst arrives, so its
/// first cycle is not chopped.
#[test]
fn the_gate_look_ahead_catches_the_onset() {
    let mut cfg = TxVoiceState::default();
    cfg.gate.enabled = true;
    cfg.gate.threshold_db = -30.0;
    cfg.gate.floor_db = -40.0;
    cfg.gate.lookahead_ms = 3.0;
    let delay = (0.003 * RATE) as usize;
    let mut audio = vec![0.0; 4800];
    audio.extend(tone(1000.0, 0.5, 4800));
    let src = audio.clone();
    run(&cfg, &mut audio);
    // The first millisecond of the burst, as it leaves the delay line.
    let onset = 4800 + delay;
    let first_ms = &audio[onset + 24..onset + 48];
    let original = &src[4800 + 24..4800 + 48];
    assert!(
        peak(first_ms) > 0.7 * peak(original),
        "onset was chopped: {} vs {}",
        peak(first_ms),
        peak(original)
    );
}

/// A sibilant band over the threshold is turned down by at most the amount;
/// the voice below it is left alone.
#[test]
fn the_de_esser_only_touches_its_band() {
    let mut cfg = TxVoiceState::default();
    cfg.deess.enabled = true;
    cfg.deess.freq_hz = 6000.0;
    cfg.deess.threshold_db = -30.0;
    cfg.deess.amount_db = -8.0;

    let n = 9600;
    let ess = tone(6000.0, 0.3, n);
    let mut out = ess.clone();
    run(&cfg, &mut out);
    let cut = db(rms(&out[n / 2..])) - db(rms(&ess[n / 2..]));
    assert!((cut + 8.0).abs() < 1.0, "sibilance should drop by the amount: {cut:.1} dB");

    let voice = tone(500.0, 0.3, n);
    let mut out = voice.clone();
    run(&cfg, &mut out);
    let change = db(rms(&out[n / 2..])) - db(rms(&voice[n / 2..]));
    assert!(change.abs() < 0.5, "500 Hz should be untouched: {change:.2} dB");
}

/// Below its threshold the de-esser's split puts the voice back together
/// exactly.
#[test]
fn an_idle_de_esser_is_transparent() {
    let mut cfg = TxVoiceState::default();
    cfg.deess.enabled = true;
    cfg.deess.threshold_db = 0.0;
    let src = tone(5500.0, 0.2, 4800);
    let mut out = src.clone();
    run(&cfg, &mut out);
    for (a, b) in src.iter().zip(&out) {
        assert!((a - b).abs() < 1e-5);
    }
}

#[test]
fn the_compressor_follows_its_static_curve() {
    let mut cfg = TxVoiceState::default();
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = -20.0;
    cfg.comp.ratio = 4.0;
    cfg.comp.knee_db = 0.0;
    cfg.comp.attack_ms = 1.0;
    cfg.comp.release_ms = 500.0;
    cfg.comp.makeup_db = 3.0;
    let n = 48_000;
    let src = tone(1000.0, 0.5, n); // -6 dBFS peak
    let mut out = src.clone();
    run(&cfg, &mut out);
    // 14 dB over at 4:1 leaves 3.5 dB over: -16.5 dBFS, plus 3 dB makeup.
    let got = db(peak(&out[n / 2..]));
    assert!((got + 13.5).abs() < 1.0, "expected about -13.5 dBFS, got {got:.1}");
}

/// Drive pushes a voice that sits below the threshold into it.
#[test]
fn drive_makes_the_compressor_work() {
    let mut cfg = TxVoiceState::default();
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = -20.0;
    cfg.comp.ratio = 4.0;
    let mut chain = VoiceChain::new();
    chain.configure(&cfg);
    let mut a = tone(1000.0, 0.05, 9600); // -26 dBFS: under the threshold
    chain.process(&mut a);
    assert!(chain.take_meters().unwrap().comp_gr_db > -0.01);

    cfg.comp.drive_db = 12.0;
    chain.configure(&cfg);
    let mut b = tone(1000.0, 0.05, 9600);
    chain.process(&mut b);
    assert!(chain.take_meters().unwrap().comp_gr_db < -3.0);
}

/// The phase rotator is all-pass: it moves the phase of a steady tone, never
/// its level.
#[test]
fn the_phase_rotator_keeps_the_level() {
    let mut cfg = TxVoiceState::default();
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = 0.0;
    cfg.comp.phase_stages = 6;
    for f in [120.0, 400.0, 2000.0] {
        let n = 24_000;
        let src = tone(f, 0.2, n);
        let mut out = src.clone();
        run(&cfg, &mut out);
        let change = db(rms(&out[n / 2..])) - db(rms(&src[n / 2..]));
        assert!(change.abs() < 0.05, "{f} Hz changed by {change:.3} dB");
    }
}

/// Whatever goes in, nothing leaves above the ceiling.
#[test]
fn the_limiter_holds_its_ceiling() {
    let mut cfg = TxVoiceState::default();
    cfg.output.limiter = true;
    cfg.output.ceiling_db = -3.0;
    cfg.output.trim_db = 6.0;
    let mut audio = tone(300.0, 0.9, 4800);
    audio.extend(tone(2700.0, 0.2, 4800));
    audio.extend((0..4800).map(|i| if i % 97 == 0 { 1.0 } else { 0.05 }));
    run(&cfg, &mut audio);
    let ceiling = 10f32.powf(-3.0 / 20.0);
    assert!(peak(&audio) <= ceiling * 1.0001, "peak {} over ceiling {ceiling}", peak(&audio));
}

#[test]
fn the_limiter_leaves_quiet_audio_alone() {
    let mut cfg = TxVoiceState::default();
    cfg.output.limiter = true;
    let src = tone(1000.0, 0.3, 4800);
    let mut out = src.clone();
    run(&cfg, &mut out);
    let d = (0.0015 * RATE).round() as usize;
    for (a, b) in src[..4800 - d].iter().zip(&out[d..]) {
        assert!((a - b).abs() < 1e-6);
    }
}

/// The engine's blocks are 10 ms, but the voice keyer, digital voice and the
/// test playback do not always hand over exactly that. How the audio is cut
/// up must not change what comes out.
#[test]
fn block_size_does_not_matter() {
    let mut cfg = TxVoiceState::default();
    cfg.gate.enabled = true;
    cfg.eq.enabled = true;
    cfg.eq.mid.gain_db = 4.0;
    cfg.deess.enabled = true;
    cfg.comp.enabled = true;
    cfg.comp.phase_stages = 3;
    cfg.output.limiter = true;
    cfg.order = [TxVoiceStage::Comp, TxVoiceStage::Eq, TxVoiceStage::DeEss, TxVoiceStage::Gate];
    let mut src = tone(180.0, 0.6, 9600);
    for (i, s) in src.iter_mut().enumerate() {
        *s += 0.2 * ((i as f32 * 0.37).sin() * (i as f32 * 0.011).cos());
    }

    let mut a = src.clone();
    run(&cfg, &mut a);

    let mut b = src.clone();
    let mut chain = VoiceChain::new();
    chain.configure(&cfg);
    chain.reset();
    for block in b.chunks_mut(37) {
        chain.process(block);
    }
    assert_eq!(a, b);
}

#[test]
fn order_changes_the_result() {
    let mut cfg = TxVoiceState::default();
    cfg.eq.enabled = true;
    cfg.eq.mid.gain_db = 12.0;
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = -20.0;
    cfg.comp.ratio = 10.0;
    let src = tone(1500.0, 0.2, 9600);
    let mut eq_first = src.clone();
    run(&cfg, &mut eq_first);
    cfg.order = [TxVoiceStage::Gate, TxVoiceStage::Comp, TxVoiceStage::DeEss, TxVoiceStage::Eq];
    let mut comp_first = src.clone();
    run(&cfg, &mut comp_first);
    assert!(peak(&comp_first[4800..]) > 1.5 * peak(&eq_first[4800..]));
}

#[test]
fn a_nan_does_not_get_through() {
    let mut cfg = TxVoiceState::default();
    cfg.eq.enabled = true;
    cfg.eq.mid.gain_db = 3.0;
    let mut chain = VoiceChain::new();
    chain.configure(&cfg);
    let mut audio = tone(500.0, 0.3, 480);
    audio[10] = f32::NAN;
    chain.process(&mut audio);
    assert!(audio.iter().all(|s| s.is_finite()));
}

#[test]
fn meters_report_once_per_window() {
    let mut cfg = TxVoiceState::default();
    cfg.comp.enabled = true;
    let mut chain = VoiceChain::new();
    chain.configure(&cfg);
    assert!(chain.take_meters().is_none());
    let mut audio = tone(500.0, 0.5, 480);
    chain.process(&mut audio);
    let m = chain.take_meters().unwrap();
    assert!((m.input_db - db(0.5)).abs() < 0.1);
    assert!(chain.take_meters().is_none());
}

/// The test chain meters an idle chain — the input meter is how a level is
/// set before any stage is on — and still leaves the audio alone; the air
/// chain reports nothing while idle.
#[test]
fn an_idle_chain_meters_only_when_asked() {
    let mut chain = VoiceChain::new();
    let src = tone(500.0, 0.25, 480);
    let mut audio = src.clone();
    chain.process(&mut audio);
    assert!(chain.take_meters().is_none());
    chain.set_meter_idle(true);
    chain.process(&mut audio);
    assert_eq!(audio, src);
    let m = chain.take_meters().unwrap();
    assert!((m.input_db - db(0.25)).abs() < 0.1);
    assert_eq!(m.input_db, m.output_db);
    assert_eq!(m.comp_gr_db, 0.0);
}

/// Turning a stage on mid-over starts it clean: a compressor last run minutes
/// ago must not open on whatever gain reduction it was holding then.
#[test]
fn a_stage_switched_on_starts_clean() {
    let mut cfg = TxVoiceState::default();
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = -40.0;
    cfg.comp.ratio = 20.0;
    cfg.comp.release_ms = 2000.0;
    let mut chain = VoiceChain::new();
    chain.configure(&cfg);
    let mut loud = tone(500.0, 0.9, 4800);
    chain.process(&mut loud);

    cfg.comp.enabled = false;
    chain.configure(&cfg);
    cfg.comp.enabled = true;
    cfg.comp.threshold_db = 0.0;
    chain.configure(&cfg);
    let src = tone(500.0, 0.1, 480);
    let mut quiet = src.clone();
    chain.process(&mut quiet);
    assert!((peak(&quiet) - peak(&src)).abs() < 1e-3, "{} vs {}", peak(&quiet), peak(&src));
}
