//! Run the STANAG 4285 receiver over a WAV and print what it found.
//!
//!     cargo run --release -p sdroxide-dsp --example s4285_probe -- \
//!         ~/s4285-captures/6348.000kHz-20261022-045545Z.wav 1800
//!
//! Arguments: `<wav> [<carrier_hz>]`. The carrier is the audio frequency of the
//! suppressed carrier (1800 Hz when the dial is on it, which is the standard's
//! own figure). A fixed-frequency log entry can be a kilohertz off, so it is a
//! parameter rather than an assumption.
//!
//! The point of this probe is the **report**, not the text: a lock, the folded
//! sync correlation and its null, the frame ACF, the three reference
//! concentrations and a rate verdict are the difference between "a 4285 is on
//! this frequency" and a guess. Text is printed when it comes, and the raw
//! payload bits are always available.

fn read_wav(path: &str) -> (Vec<f32>, f64) {
    let mut reader = hound::WavReader::open(path).expect("open the wav");
    let spec = reader.spec();
    let audio = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.expect("s")).collect(),
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => reader.samples::<i16>().map(|s| s.expect("s") as f32 / 32_768.0).collect(),
            _ => reader.samples::<i32>().map(|s| s.expect("s") as f32 / 2_147_483_648.0).collect(),
        },
    };
    (audio, spec.sample_rate as f64)
}

fn hex(bits: &[u8], n: usize) -> String {
    let take = bits.len().min(n * 8);
    let mut s = String::new();
    for byte in bits[..take].chunks(8) {
        let mut v = 0u8;
        for (i, b) in byte.iter().enumerate() {
            v |= b << (7 - i);
        }
        s.push_str(&format!("{v:02x}"));
    }
    if bits.len() > take {
        s.push('…');
    }
    s
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!("usage: s4285_probe <wav> [carrier_hz]");
        std::process::exit(2);
    }
    let (audio, rate) = read_wav(&a[0]);
    let carrier: f64 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(1800.0);
    println!(
        "{}: {:.1} s at {:.0} Hz, carrier {:.0} Hz",
        a[0],
        audio.len() as f64 / rate,
        rate,
        carrier
    );

    let mut rx = sdroxide_dsp::stanag4285::S4285Receiver::new(rate, carrier);
    let mut frames = Vec::new();
    let mut text = String::new();
    let mut messages = 0usize;
    for chunk in audio.chunks(4096) {
        for ev in rx.process(chunk) {
            match ev {
                sdroxide_dsp::stanag4285::S4285Event::Frame(f) => {
                    if let Ok(path) = std::env::var("S4285_SOFT_OUT") {
                        use std::io::Write;
                        let mut fh = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&path)
                            .expect("open the soft dump");
                        for v in &f.soft {
                            write!(fh, "{v:.4} ").expect("write");
                        }
                    }
                    frames.push(f)
                }
                sdroxide_dsp::stanag4285::S4285Event::Text(t) => {
                    text.push_str(&t);
                    for ch in t.chars() {
                        print!("{ch}");
                    }
                }
                sdroxide_dsp::stanag4285::S4285Event::Message(m) => {
                    messages += 1;
                    if messages <= 6 {
                        println!(
                            "\nmessage via {} ({} bits, {}): {}",
                            m.rate.label(),
                            m.bits.len(),
                            if m.closed { "SOM..EOM" } else { "no EOM" },
                            hex(&m.bits, 24)
                        );
                    }
                }
            }
        }
    }
    if !text.is_empty() {
        println!();
    }

    let r = rx.report();
    println!(
        "locked {} | sync {:.3} (null {:.3}) | frame ACF {:.3} | frames {} | rate {}",
        r.locked,
        r.sync_quality,
        r.correlation_floor,
        r.frame_acf,
        r.frames,
        r.rate.label()
    );
    println!(
        "last reference blocks: {:.3} {:.3} {:.3} (1.0 collapses, 0.25 is sixteen symbols of noise)",
        r.reference[0], r.reference[1], r.reference[2]
    );
    println!(
        "constellation 2/4/8-fold: {:.3} {:.3} {:.3} | offset {:+.1} Hz | timing {:.2}",
        r.constellation[0], r.constellation[1], r.constellation[2], r.carrier_offset_hz, r.timing
    );
    if !frames.is_empty() {
        let mean = |i: usize| {
            frames
                .iter()
                .map(|f: &sdroxide_dsp::stanag4285::S4285Frame| f.reference[i])
                .sum::<f32>()
                / frames.len() as f32
        };
        println!(
            "mean reference over {} frames: {:.3} {:.3} {:.3}",
            frames.len(),
            mean(0),
            mean(1),
            mean(2)
        );
        let f = frames.last().unwrap();
        println!(
            "last frame: {} phases used {}, scrambler {}, {} bits",
            f.rate.label(),
            f.phases_used,
            if f.reversed { "reversed" } else { "direct" },
            f.bits.len()
        );
    }
    println!(
        "coded: rate {} | {} messages | best SOM {}/32",
        r.coded.map(|c| c.label()).unwrap_or("none seen"),
        r.messages,
        r.best_som
    );
    println!("payload bits so far: {}", rx.bits().len());
    if let Some(path) = std::env::var("S4285_BITS_OUT").ok() {
        let s: String = rx.bits().iter().map(|b| if *b == 1 { '1' } else { '0' }).collect();
        std::fs::write(path, s).expect("write bits");
    }
    if let Some(f) = frames.last() {
        let head: String =
            f.bits.iter().take(64).map(|b| if *b == 1 { '1' } else { '0' }).collect();
        println!("first 64 bits of the last frame: {head}");
    }
}
