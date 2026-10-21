//! NAVTEX against a real SITOR-B recording.
//!
//! The recording is the SITOR-B sample from sigidwiki: a coast station
//! (MONDOLFO RADIO) broadcasting a coastal warning on 518 kHz. It is 48 kHz
//! mono MP3, converted to 8 kHz WAV for the decoder.
//!
//! Expected text (from sigidwiki):
//!   ZCZC UA69 132320 UTC JAN 12 MONDOLFO RADIO
//!   COASTAL WARNING 012/12 JAN 12 SOUTHERN ADRIATIC SEA - LECCE.
//!   ...
//!   SAILING, ANCHORAGE AND FISHING ARE PROHIBITED. NNNN

use sdroxide_dsp::NavtexRx;

const RATE: f64 = 8_000.0;

fn load_wav(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "{path}");
    let x: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap()) / 32768.0).collect();
    assert_eq!(spec.sample_rate, 8_000, "{path}: expected 8000 Hz, got {}", spec.sample_rate);
    x
}

#[test]
#[ignore]
fn a_reference_recording_decodes() {
    let path = format!("{}/tests/testdata/navtex/SITOR-B.wav", env!("CARGO_MANIFEST_DIR"));
    let audio = load_wav(&path);
    let mut rx = NavtexRx::new(RATE);
    rx.set_center_hz(2000.0);
    let mut got = String::new();
    for chunk in audio.chunks(512) {
        got.push_str(&rx.process(chunk));
    }
    println!("DECODED: {got:?}");
    println!("SYNC: {}", rx.in_sync());
    println!("COUNTS: {:?}", rx.counts());
    println!("AFC: {}", rx.afc_offset_hz());
    assert!(got.contains("ZCZC UA69"), "decoded {got:?}");
    assert!(got.contains("COASTAL WARNING"), "decoded {got:?}");
    assert!(got.contains("NNNN"), "decoded {got:?}");
}
