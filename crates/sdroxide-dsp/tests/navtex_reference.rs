//! NAVTEX against real SITOR-B recordings.
//!
//! Two fixtures:
//!
//! 1. `SITOR-B.wav` — the SITOR-B sample from sigidwiki (MONDOLFO RADIO
//!    broadcasting a coastal warning on 518 kHz), 8 kHz mono, the original
//!    fixture. Kept as a regression test: the decoder must still read it.
//!
//! 2. `Bermuda-48k.wav` — a 48 kHz mono recording of Bermuda Radio on 518 kHz
//!    (operator F4NRM, 2026-10-22, 180 s, 17 MB). This is the production
//!    rate the engine actually feeds the decoder at; it is the fixture that
//!    exposed the ratio-discriminator bug, which only manifested at 48 kHz.
//!
//! The reference text `Bermuda-48k.txt` is itself a lossy decode (it carries
//! ~15 character-level errors and a figures/letters garbled run), so the test
//! asserts message content — anchors and decoding metrics — not byte equality.
//! See AGENTS.md: "A passing test is evidence about the test, not about the
//! program."

use sdroxide_dsp::NavtexRx;

const RATE_8K: f64 = 8_000.0;
const RATE_48K: f64 = 48_000.0;

fn load_wav(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "{path}");
    let x: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
    x
}

#[test]
#[ignore]
fn a_reference_recording_decodes_sitor_b() {
    let path = format!("{}/tests/testdata/navtex/SITOR-B.wav", env!("CARGO_MANIFEST_DIR"));
    let audio = load_wav(&path);
    let mut rx = NavtexRx::new(RATE_8K);
    rx.set_center_hz(2000.0);
    let mut got = String::new();
    for chunk in audio.chunks(512) {
        got.push_str(&rx.process(chunk));
    }
    println!("SITOR-B DECODED: {got:?}");
    println!("SITOR-B SYNC: {}", rx.in_sync());
    println!("SITOR-B COUNTS: {:?}", rx.counts());
    println!("SITOR-B AFC: {}", rx.afc_offset_hz());
    assert!(got.contains("ZCZC UA69"), "decoded {got:?}");
    assert!(got.contains("COASTAL WARNING"), "decoded {got:?}");
    assert!(got.contains("NNNN"), "decoded {got:?}");
}

#[test]
#[ignore]
fn a_reference_recording_decodes_bermuda_48k() {
    let path = format!("{}/tests/testdata/navtex/Bermuda-48k.wav", env!("CARGO_MANIFEST_DIR"));
    let audio = load_wav(&path);
    let mut rx = NavtexRx::new(RATE_48K);
    rx.set_center_hz(1700.0);
    let mut got = String::new();
    for chunk in audio.chunks(512) {
        got.push_str(&rx.process(chunk));
    }
    println!("BERMUDA DECODED: {got:?}");
    println!("BERMUDA SYNC: {}", rx.in_sync());
    println!("BERMUDA COUNTS: {:?}", rx.counts());
    println!("BERMUDA AFC: {}", rx.afc_offset_hz());
    // Structural anchors — the message, not byte equality.
    assert!(got.contains("ZCZC BE22"), "decoded {got:?}");
    assert!(got.contains("BERMUDA"), "decoded {got:?}");
    assert!(got.contains("261021"), "decoded {got:?}");
    assert!(got.contains("NNNN"), "decoded {got:?}");
    // Lost characters should be a small fraction, not most of the message.
    let lost = got.matches('*').count();
    assert!(lost < 200, "too many lost chars ({lost}): {got:?}");
    // The synopsis and the forecast headings must be readable.
    assert!(got.contains("SYNOPSIS"), "decoded {got:?}");
    assert!(got.contains("TONIGHT"), "decoded {got:?}");
    assert!(got.contains("HURSDAY") || got.contains("THURSDAY"), "decoded {got:?}");
    assert!(got.contains("FRIDAY"), "decoded {got:?}");
}

#[test]
fn the_decoder_is_rate_independent() {
    // The same NAVTEX message, synthesised at 8 kHz and at 48 kHz, must
    // decode to the same anchors. This is the guard that would have caught
    // the ratio-discriminator bug, which only manifested at 48 kHz (the
    // production rate) because the per-sample constants were tuned at 8 kHz.
    use sdroxide_dsp::navtex_test::{encode_bits, synth};
    let text = "ZCZC BE22 BERMUDA RADIO NNNN";
    let bits = encode_bits(text);
    for &rate in &[RATE_8K, RATE_48K] {
        let audio = synth(&bits, rate, 1700.0, 0.5);
        let mut rx = NavtexRx::new(rate);
        rx.set_center_hz(1700.0);
        let mut got = String::new();
        for chunk in audio.chunks(512) {
            got.push_str(&rx.process(chunk));
        }
        assert!(got.contains("ZCZC"), "rate {rate}: {got:?}");
        assert!(got.contains("BERMUDA"), "rate {rate}: {got:?}");
        assert!(got.contains("NNNN"), "rate {rate}: {got:?}");
    }
}
