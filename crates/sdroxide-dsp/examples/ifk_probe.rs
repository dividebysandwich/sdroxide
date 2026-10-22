//! Run the IFK (THOR / DominoEX) receiver over a WAV and print what it decided.
//!
//!     cargo run --release -p sdroxide-dsp --example ifk_probe -- \
//!         samples/fldigi/DOMEX8.wav DOMEX8 [centre_hz]
//!
//! The point of this probe is the **decision sequence**, not the text: a mode that
//! reads nothing can be wrong in its front end (which tone did it pick, and when)
//! or in its character layer (what the steps mean), and those two look identical
//! from the text alone. The steps and the bank positions are printed so the first
//! one can be compared with an independent measurement of the same sample.

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
    if a.len() < 2 {
        eprintln!("usage: ifk_probe <wav> <variant> [centre_hz]");
        std::process::exit(2);
    }
    let (audio, rate) = read_wav(&a[0]);
    let variant = sdroxide_dsp::ifk::IfkVariant::named(&a[1]).expect("a variant fldigi knows");
    let centre: f64 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(1500.0);
    println!(
        "{}: {:.1} s at {rate:.0} Hz, variant {}, centre {centre:.0} Hz",
        a[0],
        audio.len() as f64 / rate,
        variant.name
    );
    let mut rx = sdroxide_dsp::ifk::IfkRx::new(rate, centre, variant);
    for chunk in audio.chunks(512) {
        rx.feed(chunk);
    }
    rx.feed(&vec![0.0f32; 16_000]);
    let steps = rx.steps();
    let pos = rx.positions();
    let refused = steps.iter().filter(|s| **s == 0xFF).count();
    let over = steps.iter().filter(|s| **s != 0xFF && **s > 15).count();
    println!(
        "{} steps decided, {refused} refused, {over} outside the 16-symbol alphabet",
        steps.len()
    );
    println!("steps[:80] {:?}", &steps[..steps.len().min(80)]);
    println!("positions[:80] {:?}", &pos[..pos.len().min(80)]);
    if let Ok(path) = std::env::var("IFK_STEPS_OUT") {
        let t: String = steps.iter().map(|s| format!("{s},")).collect();
        std::fs::write(path, t).expect("write the steps");
    }
    println!("text: {:?}", rx.text());
}
