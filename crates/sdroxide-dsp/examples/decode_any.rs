//! Decode a WAV with whichever of our text demodulators fits, and print what came
//! out. The bench sweep uses this to turn "recorded" into a pass or a fail for
//! the modes we already claim to read.
//!
//!     cargo run --release -p sdroxide-dsp --example decode_any -- \
//!         BPSK31.wav psk 1700 31.25 0
//!     cargo run --release -p sdroxide-dsp --example decode_any -- \
//!         RTTY.wav rtty 1700 45.45 170 1
//!
//! Arguments: `<wav> <kind> <centre_hz> [<baud>] [<shift_hz>] [<qpsk|reverse>]`
//!
//! `kind` is `psk` or `rtty`. Olivia is decoded by `sdroxide-olivia`'s own
//! `decode_file`, because that is the engine the program ships.
//!
//! Hell is deliberately not here: it delivers a picture, not text, so there is
//! nothing for a script to compare against.

use sdroxide_dsp::{PskRx, RttyRx};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 3 {
        eprintln!("usage: decode_any <wav> <psk|rtty> <centre_hz> [baud] [shift] [qpsk|reverse]");
        std::process::exit(2);
    }
    let mut reader = hound::WavReader::open(&a[0]).expect("open the wav");
    let spec = reader.spec();
    let audio: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
        }
    };
    let rate = spec.sample_rate as f64;
    let centre: f64 = a[2].parse().expect("centre");

    let out = match a[1].as_str() {
        "psk" => {
            let baud: f64 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(31.25);
            let mut rx = PskRx::new_baud(rate, centre, baud);
            if a.get(5).map(String::as_str) == Some("qpsk") {
                rx.set_qpsk(true);
            }
            let mut out = String::new();
            for chunk in audio.chunks(512) {
                out.push_str(&rx.process(chunk));
            }
            out
        }
        "rtty" => {
            let baud: f64 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(45.45);
            let shift: f64 = a.get(4).and_then(|v| v.parse().ok()).unwrap_or(170.0);
            let mut rx = RttyRx::new(rate, centre, baud, shift);
            if a.get(5).map(String::as_str) == Some("reverse") {
                rx.set_reverse(true);
            }
            let mut out = String::new();
            for chunk in audio.chunks(512) {
                out.push_str(&rx.process(chunk));
            }
            out
        }
        other => {
            eprintln!("unknown kind {other:?}");
            std::process::exit(2);
        }
    };
    println!("{out}");
}
