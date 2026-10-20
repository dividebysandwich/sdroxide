//! Olivia's reverse switch, and the state of our transmitter.
//!
//! Two things have to hold for the switch to be worth having: a reversed
//! transmission has to decode when the receiver is told to reverse, and it has
//! to *fail* when it is not. The second half is the one that matters — without
//! it the test would pass on a decoder that ignores the setting entirely.
//!
//! The pair is proved against a real capture (`SDROXIDE_OLIVIA_SAMPLE`), because
//! our own transmitter does not yet produce audio the engine can read — see
//! `our_transmitter_is_not_yet_readable` below, which records that finding
//! rather than asserting it is fine.

use sdroxide_olivia::OliviaDecoder;

const RATE: f64 = 8000.0;

fn decode(audio: &[f32], centre: f64, tones: usize, bw: u32, reverse: bool) -> String {
    let Some(mut rx) = OliviaDecoder::new(RATE, centre, tones, bw, reverse) else {
        return String::new();
    };
    let mut out = String::new();
    for chunk in audio.chunks(1024) {
        out.push_str(&rx.process(chunk));
    }
    out
}

/// Silence must not produce text, with or without the switch — otherwise every
/// assertion below could pass on a decoder that emits characters unconditionally.
#[test]
fn silence_produces_nothing() {
    let quiet = vec![0.0f32; (RATE as usize) * 12];
    for reverse in [false, true] {
        let out = decode(&quiet, 1718.0, 16, 500, reverse);
        let letters = out.chars().filter(|c| c.is_alphabetic()).count();
        assert!(letters < 8, "silence produced {letters} letters (reverse {reverse}): {out:?}");
    }
}

/// The switch is the difference between reading a station and reading noise.
///
/// Point `SDROXIDE_OLIVIA_SAMPLE` at a mono 8 kHz WAV of a **reversed** Olivia
/// transmission — `~/Downloads/sounds/our-captures/olivia_16-500_cb_27415.wav`
/// is one — and `SDROXIDE_OLIVIA_TEXT` at words it contains.
///
/// Run it with
/// `SDROXIDE_OLIVIA_SAMPLE=... SDROXIDE_OLIVIA_TEXT="EAGLE LANDED" \
///  cargo test -p sdroxide-digi --release --test olivia_reverse -- --ignored --nocapture`.
#[test]
#[ignore]
fn the_reverse_switch_is_what_makes_this_read() {
    let Ok(path) = std::env::var("SDROXIDE_OLIVIA_SAMPLE") else {
        eprintln!("SDROXIDE_OLIVIA_SAMPLE unset; skipping");
        return;
    };
    let Ok(want) = std::env::var("SDROXIDE_OLIVIA_TEXT") else {
        eprintln!("SDROXIDE_OLIVIA_TEXT unset; skipping");
        return;
    };
    let mut reader = hound::WavReader::open(&path).expect("open the sample WAV");
    let audio: Vec<f32> = match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
        }
    };
    let centre: f64 =
        std::env::var("SDROXIDE_OLIVIA_HZ").ok().and_then(|v| v.parse().ok()).unwrap_or(1718.0);
    let tones: usize =
        std::env::var("SDROXIDE_OLIVIA_TONES").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
    let bw: u32 =
        std::env::var("SDROXIDE_OLIVIA_BW").ok().and_then(|v| v.parse().ok()).unwrap_or(500);

    let reversed = decode(&audio, centre, tones, bw, true);
    eprintln!("reverse on:  {reversed:?}");
    for w in want.split_whitespace() {
        assert!(reversed.contains(w), "with the switch on, {w:?} must be recovered");
    }

    let wrong = decode(&audio, centre, tones, bw, false);
    eprintln!("reverse off: {wrong:?}");
    assert!(
        !want.split_whitespace().all(|w| wrong.contains(w)),
        "the same audio decoded with the switch off, so the switch is not wired"
    );
}

/// Our transmitter does not yet produce audio Jalocha's engine can read.
///
/// This is a recording of a real limitation, not a passing test: our modulator
/// and our own `OliviaRx` agree with *each other* — their loopbacks pass — but
/// neither matches fldigi, so what we send is not Olivia to anyone else. The
/// note is kept as a test so the state cannot be forgotten, and it is `#[ignore]`d
/// because it asserts a fault that is known and not yet fixed.
///
/// The fix is the same shape as the receiver's: our modulator's tone assignment
/// has to be checked against the engine's, with `MFSK_Transmitter` as the oracle.
#[test]
#[ignore]
fn our_transmitter_is_not_yet_readable() {
    use sdroxide_dsp::OliviaTx;

    let mut tx = OliviaTx::new(RATE, 1500.0, 16, 500.0);
    tx.push_text("PLAIN TEXT");
    let mut sig = Vec::new();
    let mut guard = 0;
    while tx.sent_chars() < tx.total_chars() && guard < 40_000 {
        let mut b = [0.0f32; 2048];
        tx.next_block(&mut b);
        sig.extend_from_slice(&b);
        guard += 1;
    }
    sig.extend(std::iter::repeat_n(0.0f32, (RATE as usize) * 3));

    let out = decode(&sig, 1500.0, 16, 500, false);
    eprintln!("what the engine makes of our transmission: {out:?}");
    assert!(
        out.contains("PLAIN"),
        "our transmitter is not yet readable by fldigi's engine: {out:?}"
    );
}
