//! Run the STANAG 4285 receiver over a WAV and print what it found.
//!
//!     cargo run --release -p sdroxide-dsp --example s4285_probe -- \
//!         ~/s4285-captures/8467.800kHz-20261021-225334Z.wav 1800
//!
//! Arguments: `<wav> [<carrier_hz>] [<text_carrier>]`. The first carrier is the
//! audio frequency of the suppressed carrier (1800 Hz when the dial is on it,
//! which is the standard's own figure). A fixed-frequency log entry can be a
//! kilohertz out, so it is a parameter rather than an assumption.
//!
//! The point of this probe is the **report**, not the text: a lock, a frame
//! phase, the three reference concentrations and a rate verdict are the
//! difference between "a 4285 is on this frequency" and a guess. Text is printed
//! when it comes, and the raw payload bits are always available.

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
    for chunk in audio.chunks(4096) {
        for ev in rx.process(chunk) {
            match ev {
                sdroxide_dsp::stanag4285::S4285Event::Frame(f) => frames.push(f),
                sdroxide_dsp::stanag4285::S4285Event::Text(t) => {
                    text.push_str(&t);
                    for ch in t.chars() {
                        print!("{ch}");
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
        "locked {} | sync {:.3} | floor {:.3} | frames {} | rate {} | offset {:+.1} Hz",
        r.locked,
        r.sync_quality,
        r.correlation_floor,
        r.frames,
        r.rate.label(),
        r.carrier_offset_hz
    );
    println!(
        "frame ACF {:.3} (a correct 4285 repeats 80 of its 256 symbols, so ~0.31; another mode ~0.03)",
        r.frame_acf
    );
    println!(
        "last reference blocks: {:.3} {:.3} {:.3} (1.0 collapses, 0.25 is sixteen symbols of noise)",
        r.reference[0], r.reference[1], r.reference[2]
    );
    if let Some(f) = frames.last() {
        println!(
            "last frame: {} phases used {}, scrambler {}, {} bits",
            f.rate.label(),
            f.phases_used,
            if f.reversed { "reversed" } else { "direct" },
            f.bits.len()
        );
    }
    println!("payload bits so far: {}", rx.bits().len());
    if !frames.is_empty() {
        // The first 64 bits, so a frame that is not ITA2 is visible as bits.
        let head: String = frames
            .last()
            .map(|f| f.bits.iter().take(64).map(|b| if *b == 1 { '1' } else { '0' }).collect())
            .unwrap_or_default();
        println!("first 64 bits of the last frame: {head}");
    }
}
