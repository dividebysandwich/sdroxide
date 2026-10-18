//! **Decode a WEFAX recording and say what the decoder made of it.**
//!
//! Written for issue #91 — *"WEEFAX broken in v2.0.13"* — where the only
//! artefact was a screenshot of a picture 322 lines long. A screenshot cannot
//! say *why* a picture stopped, so this reads the audio and prints the
//! decoder's own account of it: whether the start tone was found, where the
//! phasing pulse landed, how many lines arrived, and whether the transmission
//! ended with a stop tone or just stopped.
//!
//! ```sh
//! cargo run -p sdroxide-dsp --release --example wefax_probe -- \
//!     recording.wav out.pgm [lpm] [ioc]
//! ```
//!
//! `lpm` defaults to 120 and `ioc` to 576 — what every weather service uses,
//! and what `WEFAX_STATIONS` says about all of them. The picture comes out as a
//! **PGM** (P5, 8-bit), which every image tool reads and which needs no encoder
//! here: `ffmpeg -i out.pgm out.png` if you want a PNG.
//!
//! The audio may be any sample rate; it is resampled to the decoder's own
//! 12 kHz first. The resampler is linear interpolation, which is right for a
//! probe and wrong for a product — the engine hands the decoder its tap rate
//! directly, so nothing in the program depends on this.

use std::io::Write;

use sdroxide_dsp::{WefaxEvent, WefaxIoc, WefaxLpm, WefaxRx};

/// What the decoder is built for: the digi tap rate the engine feeds it.
const RATE: f64 = 12_000.0;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: wefax_probe <audio.wav> <out.pgm> [lpm] [ioc]");
        eprintln!("  lpm: 60 | 90 | 120 | 180 | 240   (default 120)");
        eprintln!("  ioc: 576 | 288                   (default 576)");
        std::process::exit(2);
    }
    let audio_path = &args[1];
    let out_path = &args[2];
    let lpm = match args.get(3).map(String::as_str) {
        None | Some("120") => WefaxLpm::L120,
        Some("60") => WefaxLpm::L60,
        Some("90") => WefaxLpm::L90,
        Some("180") => WefaxLpm::L180,
        Some("240") => WefaxLpm::L240,
        Some(other) => {
            eprintln!("unknown line rate {other}");
            std::process::exit(2);
        }
    };
    let ioc = match args.get(4).map(String::as_str) {
        None | Some("576") => WefaxIoc::I576,
        Some("288") => WefaxIoc::I288,
        Some(other) => {
            eprintln!("unknown IOC {other}");
            std::process::exit(2);
        }
    };

    let (samples, file_rate) = match read_wav(audio_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let seconds = samples.len() as f64 / file_rate;
    println!("{audio_path}: {seconds:.1} s at {file_rate} Hz");

    let audio = resample(&samples, file_rate, RATE);

    // **Two passes.** The first lets the decoder find the transmission by its
    // own start tone, which is what happens on the air. A recording that begins
    // *inside* a picture has no start tone to find — most samples on the
    // internet are like that — so if the first pass received nothing, the
    // second starts the decoder by hand and lets it phase on the first pulse it
    // sees. Which pass produced the picture is printed, because "it needed a
    // manual start" is a fact about the recording, not about the decoder.
    //
    // **Every start makes its own picture.** A recording of a whole evening
    // holds transmission after transmission, and the decoder starts a new
    // picture for each — so the probe writes one file per picture rather than
    // letting the last one overwrite the rest. The line count of each is the
    // number that matters: a picture that stops after a handful of lines is a
    // decoder that lost the signal, and one that runs to a thousand is a chart.
    let (pictures, how) = match decode(&audio, lpm, ioc, false) {
        Some(v) => (v, "found by its start tone"),
        None => match decode(&audio, lpm, ioc, true) {
            Some(v) => (v, "started by hand (no start tone in the recording)"),
            None => (Vec::new(), "nothing"),
        },
    };
    println!("  pass: {how}");
    if pictures.is_empty() {
        eprintln!("nothing to write: no lines were decoded");
        std::process::exit(1);
    }
    let width =
        pictures.iter().map(|p| p.iter().map(Vec::len).max().unwrap_or(0)).max().unwrap_or(0);
    for (i, picture) in pictures.iter().enumerate() {
        let path = if pictures.len() == 1 {
            out_path.clone()
        } else {
            let stem = out_path.strip_suffix(".pgm").unwrap_or(out_path);
            format!("{stem}-{}.pgm", i + 1)
        };
        match write_pgm(&path, picture, width) {
            Ok(()) => println!("  wrote {path} ({width}x{})", picture.len()),
            Err(e) => eprintln!("{e}"),
        }
    }
}

/// One pass over the audio. `manual` starts the decoder without waiting for a
/// start tone; `None` means nothing came out.
fn decode(audio: &[f32], lpm: WefaxLpm, ioc: WefaxIoc, manual: bool) -> Option<Vec<Vec<Vec<u8>>>> {
    let mut rx = WefaxRx::new(RATE);
    rx.set_lpm(lpm);
    rx.set_ioc(ioc);
    rx.set_auto_start(!manual);

    // One picture per start, in the order they arrived.
    let mut pictures: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut width = 0usize;
    let mut events = Vec::new();
    if manual {
        rx.start_manual(&mut events);
        for e in &events {
            if let WefaxEvent::Start { ioc, lpm, .. } = e {
                println!("  start: {} at {}, asked for", ioc.label(), lpm.label());
                // A start *is* a picture, however it was started — without this
                // the first line would arrive with nowhere to go.
                pictures.push(Vec::new());
            }
        }
        events.clear();
    }
    let mut fed = 0usize;
    for block in audio.chunks(4096) {
        events.clear();
        rx.process(block, &mut events);
        fed += block.len();
        let at = fed as f64 / RATE;
        for e in &events {
            match e {
                WefaxEvent::Start { ioc, lpm, auto } => {
                    println!(
                        "  {at:7.1}s start: {} at {}, {}",
                        ioc.label(),
                        lpm.label(),
                        if *auto { "found by its start tone" } else { "asked for" }
                    );
                    pictures.push(Vec::new());
                }
                WefaxEvent::Phased { skew } => {
                    println!("  {at:7.1}s phased: the pulse is {skew} samples into the line");
                }
                WefaxEvent::Line { y, gray } => {
                    width = width.max(gray.len());
                    let y = *y as usize;
                    let picture = pictures.last_mut()?;
                    if picture.len() <= y {
                        picture.resize(y + 1, vec![255u8; width]);
                    }
                    picture[y] = gray.clone();
                }
                WefaxEvent::Complete { lines, by_tone } => {
                    println!(
                        "  {at:7.1}s complete: {lines} lines, {}",
                        if *by_tone {
                            "ended by its stop tone"
                        } else {
                            "no stop tone — the signal faded, the recording ended, or it was stopped"
                        }
                    );
                }
            }
        }
    }

    let seconds = audio.len() as f64 / RATE;
    let lines: usize = pictures.iter().map(Vec::len).sum();
    if lines == 0 {
        return None;
    }
    println!(
        "  {lines} lines in {seconds:.1} s — {:.1} lines/s ({} asked for), {} picture(s)",
        lines as f64 / seconds.max(0.001),
        lpm.label(),
        pictures.len()
    );
    for (i, p) in pictures.iter().enumerate() {
        println!("    picture {}: {} lines", i + 1, p.len());
    }
    Some(pictures)
}

/// A WAV as mono `f32` and the rate it was recorded at.
fn read_wav(path: &str) -> Result<(Vec<f32>, f64), String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("opening {path}: {e}"))?;
    let spec = reader.spec();
    let rate = f64::from(spec.sample_rate);
    let channels = spec.channels.max(1) as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("reading {path}: {e}"))?,
        hound::SampleFormat::Int => {
            let peak = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("reading {path}: {e}"))?
                .into_iter()
                .map(|s| s as f32 / peak)
                .collect()
        }
    };
    // Down to one channel: WEFAX is a single subcarrier, and a stereo recording
    // of it carries the same thing twice.
    let mono = if channels > 1 {
        samples.chunks(channels).map(|c| c.iter().sum::<f32>() / channels as f32).collect()
    } else {
        samples
    };
    Ok((mono, rate))
}

/// Linear interpolation to `to` Hz. See the note at the top of the file.
fn resample(from: &[f32], from_hz: f64, to_hz: f64) -> Vec<f32> {
    if (from_hz - to_hz).abs() < 0.5 {
        return from.to_vec();
    }
    let ratio = from_hz / to_hz;
    let n = (from.len() as f64 / ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 * ratio;
            let a = x.floor() as usize;
            let b = (a + 1).min(from.len().saturating_sub(1));
            let f = (x - a as f64) as f32;
            from.get(a).copied().unwrap_or(0.0) * (1.0 - f)
                + from.get(b).copied().unwrap_or(0.0) * f
        })
        .collect()
}

/// A P5 PGM: one byte per pixel, 0 black … 255 white, which is exactly what
/// `WefaxEvent::Line` carries.
fn write_pgm(path: &str, picture: &[Vec<u8>], width: usize) -> Result<(), String> {
    if picture.is_empty() || width == 0 {
        return Err("nothing to write: no lines were decoded".to_string());
    }
    let mut f = std::fs::File::create(path).map_err(|e| format!("creating {path}: {e}"))?;
    write!(f, "P5\n{width} {}\n255\n", picture.len()).map_err(|e| e.to_string())?;
    for row in picture {
        f.write_all(row).map_err(|e| e.to_string())?;
    }
    Ok(())
}
