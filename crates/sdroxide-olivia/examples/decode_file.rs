//! Decode a WAV with the vendored Olivia engine and print the text.
//!
//! The bench script (`tools/bench/bench.py`) calls this after recording a
//! transmission off the air, so that what is measured is the same engine the
//! band menu uses — not a mock or a second implementation.
//!
//!     cargo run --release -p sdroxide-olivia --example decode_file -- \
//!         capture.wav 16 500 1705 1
//!
//! Arguments: `<wav> <tones> <bandwidth> <centre_hz> <reverse 0|1> [text...]`
//! With a text, the exit status says whether it was recovered.

use sdroxide_olivia::OliviaDecoder;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        eprintln!("usage: decode_file <wav> <tones> <bandwidth> <centre_hz> <reverse> [text...]");
        std::process::exit(2);
    }
    let path = &args[0];
    let tones: usize = args[1].parse().expect("tones");
    let bandwidth: u32 = args[2].parse().expect("bandwidth");
    let centre: f64 = args[3].parse().expect("centre");
    let reverse = args[4] != "0";
    let want = args[5..].join(" ");

    let mut reader = hound::WavReader::open(path).expect("open the wav");
    let spec = reader.spec();
    let audio: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
        }
    };

    let Some(mut rx) =
        OliviaDecoder::new(spec.sample_rate as f64, centre, tones, bandwidth, reverse)
    else {
        eprintln!("the engine could not allocate");
        std::process::exit(2);
    };
    // The engine's pipeline runs about a second behind the audio, so a capture
    // that ends on the last character loses that character's block. Silence
    // flushes it — the same tail the tests feed.
    let pad = vec![0.0f32; (spec.sample_rate as usize).max(8000) * 2];
    let mut out = String::new();
    for chunk in audio.iter().chain(pad.iter()).copied().collect::<Vec<_>>().chunks(1024) {
        out.push_str(&rx.process(chunk));
    }
    println!("{out}");

    if !want.is_empty() {
        let missing: Vec<&str> = want.split_whitespace().filter(|w| !out.contains(w)).collect();
        if missing.is_empty() {
            eprintln!("ok: every word of {want:?} was recovered");
        } else {
            eprintln!("MISSING {missing:?} from {want:?}");
            std::process::exit(1);
        }
    }
}
