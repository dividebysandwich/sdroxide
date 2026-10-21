//! Run whichever of our decoders fits over the whole corpus and print one line
//! per mode. This is the measurement behind `MODE-PROGRESS.md`: the sample is
//! the gate, and a mode that does not read its own text says so here.
//!
//!     cargo run --release -p sdroxide-dsp --example corpus_survey
//!
//! `SDROXIDE_CORPUS` overrides the directory. Every decodable family is tried
//! with the parameters its name implies, searching the centre both polarities
//! for the ones that need it. Picture modes are not scored here — they have
//! their own tests.

use sdroxide_dsp::{CwRx, FsqRx, PskRx, RttyRx};

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\": ");
    let start = line.find(&pat)? + pat.len();
    let rest = &line[start..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(&stripped[..end])
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

fn read_wav(path: &str) -> (Vec<f32>, f64) {
    let mut reader = hound::WavReader::open(path).expect("open the wav");
    let spec = reader.spec();
    let audio = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => {
            reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect()
        }
    };
    (audio, spec.sample_rate as f64)
}

fn words(want: &str) -> Vec<&str> {
    want.split_whitespace().filter(|w| w.len() >= 3).collect()
}

fn score(text: &str, want: &[&str]) -> usize {
    want.iter().filter(|w| text.contains(**w)).count()
}

fn main() {
    let root = std::env::var("SDROXIDE_CORPUS")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../samples/fldigi").into());
    let manifest = format!("{root}/manifest.jsonl");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        eprintln!("no manifest at {manifest}");
        return;
    };
    println!("mode|decoder|score|sample");
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(mode) = field(line, "mode") else { continue };
        let Some(wav) = field(line, "wav") else { continue };
        let want_text = field(line, "text").unwrap_or("EAGLE LANDED SDRBROWN");
        let want = words(want_text);
        let centre: f64 = field(line, "centre_hz").and_then(|v| v.parse().ok()).unwrap_or(1700.0);
        let path = format!("{root}/{wav}");
        if !std::path::Path::new(&path).exists() {
            println!("{mode}|missing|0|");
            continue;
        }
        let (audio, rate) = read_wav(&path);
        let upper = mode.to_ascii_uppercase();

        let (decoder, best): (&str, (usize, String)) = if upper.starts_with("RTTY") {
            let baud = if upper.contains("75") {
                75.0
            } else if upper.contains("50") {
                50.0
            } else {
                45.45
            };
            let shifts: &[f64] = if baud > 46.0 { &[450.0, 170.0, 850.0] } else { &[170.0, 450.0] };
            let mut best = (0usize, String::new());
            for &shift in shifts {
                for reverse in [false, true] {
                    for d in [-40.0f64, -20.0, 0.0, 20.0, 40.0] {
                        let mut rx = RttyRx::new(rate, centre + d, baud, shift);
                        rx.set_reverse(reverse);
                        let mut out = String::new();
                        for chunk in audio.chunks(512) {
                            out.push_str(&rx.process(chunk));
                        }
                        out.push_str(&rx.process(&vec![0.0f32; 8000]));
                        let s = score(&out, &want);
                        if s > best.0 {
                            best = (s, out);
                        }
                    }
                }
            }
            ("rtty", best)
        } else if upper.contains("PSK") && !upper.starts_with("8PSK") {
            let baud = if upper.contains("1000") {
                1000.0
            } else if upper.contains("500") {
                500.0
            } else if upper.contains("250") {
                250.0
            } else if upper.contains("125") {
                125.0
            } else if upper.contains("63") {
                62.5
            } else {
                31.25
            };
            let qpsk = upper.starts_with("QPSK");
            let mut best = (0usize, String::new());
            // The manifest's centre is a strongest-block estimate; a PSK
            // carrier is a single line and the two can be 30 Hz apart.
            for d in [-60.0f64, -45.0, -30.0, -15.0, 0.0, 15.0, 30.0, 45.0, 60.0] {
                let mut rx = PskRx::new_baud(rate, centre + d, baud);
                rx.set_qpsk(qpsk);
                let mut out = String::new();
                for chunk in audio.chunks(512) {
                    out.push_str(&rx.process(chunk));
                }
                out.push_str(&rx.process(&vec![0.0f32; 8000]));
                let s = score(&out, &want);
                if s > best.0 {
                    best = (s, out);
                }
            }
            ("psk", best)
        } else if upper == "CW" {
            let mut best = (0usize, String::new());
            for d in [-30.0f64, 0.0, 30.0] {
                let mut rx = CwRx::new(rate, (centre + d) as f32);
                let mut out = String::new();
                for chunk in audio.chunks(512) {
                    out.push_str(&rx.process(chunk));
                }
                out.push_str(&rx.process(&vec![0.0f32; 8000]));
                let s = score(&out, &want);
                if s > best.0 {
                    best = (s, out);
                }
            }
            ("cw", best)
        } else if upper == "FSQ" {
            let mut best = (0usize, String::new());
            for baud in [2.0f64, 4.0, 6.0, 8.0] {
                let mut rx = FsqRx::new(rate, centre, baud);
                let mut out = String::new();
                for chunk in audio.chunks(512) {
                    out.push_str(&rx.process(chunk));
                }
                out.push_str(&rx.process(&vec![0.0f32; 8000]));
                let s = score(&out, &want);
                if s > best.0 {
                    best = (s, out);
                }
            }
            ("fsq", best)
        } else {
            ("none", (0, String::new()))
        };
        let quote: String = best.1.chars().filter(|c| !c.is_control()).take(60).collect();
        println!("{mode}|{decoder}|{}/{}|{quote}", best.0, want.len());
    }
}
