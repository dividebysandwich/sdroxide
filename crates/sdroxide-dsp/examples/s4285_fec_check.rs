//! Is a recording carrying STANAG 4285 with Annex E coding, or without it?
//!
//!     cargo run --release -p sdroxide-dsp --example s4285_fec_check -- \
//!         ~/s4285-captures/6348.000kHz-20261022-045545Z.wav [carrier_hz]
//!
//! The question a constellation cannot answer. Every uncoded rate shares its
//! modulation with a coded one — 1200 bps BPSK is also 600 at rate 1/2, 300 at
//! 1/4, 150 at 1/8 and 75 at 1/16 — so the only evidence is whether the stream
//! satisfies the code's trellis after deinterleaving.
//!
//! The test is the decoder's own metric against **the same values shuffled**.
//! Shuffling destroys the trellis and nothing else, so it is the null for exactly
//! this question: a stream that carries the code scores far above it, and one that
//! does not scores the same. Both are printed, beside each other, because a
//! statistic without its own null is how this mode's earlier work lost a day.

use sdroxide_dsp::stanag4285::{S4285Event, S4285Receiver};
use sdroxide_dsp::stanag4285_fec as fec;

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

/// A xorshift, so the null is reproducible and is not the statistics of `rand`.
fn shuffled(input: &[f32], seed: u32) -> Vec<f32> {
    let mut out = input.to_vec();
    let mut s = seed;
    for i in (1..out.len()).rev() {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let j = (s as usize) % (i + 1);
        out.swap(i, j);
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!("usage: s4285_fec_check <wav> [carrier_hz]");
        std::process::exit(2);
    }
    let (audio, rate) = read_wav(&a[0]);
    let carrier: f64 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(1800.0);
    let mut rx = S4285Receiver::new(rate, carrier);
    let mut soft: Vec<f32> = Vec::new();
    let mut frames = 0;
    for chunk in audio.chunks(4096) {
        for ev in rx.process(chunk) {
            if let S4285Event::Frame(f) = ev {
                soft.extend_from_slice(&f.soft);
                frames += 1;
            }
        }
    }
    println!("{}: {} frames, {} soft decisions at {}", a[0], frames, soft.len(), carrier);
    if soft.len() < 5_000 {
        println!("  too little to judge");
        return;
    }
    // The stream has to be deinterleaved before the decode means anything, so
    // every rate's `j` is tried — and both signs, since the frame phase is only
    // known up to pi.
    println!("  {:>26}  {:>8}  {:>8}  {:>6}", "branch", "metric", "null", "ratio");
    for rate in fec::CodedRate::BPSK {
        for sign in [1.0f32, -1.0] {
            let mut di = fec::ConvInterleaver::deinterleaver(rate.j());
            let stream: Vec<f32> = soft.iter().map(|v| di.push(v * sign)).collect();
            let pairs = fec::average_pairs(&stream, rate.repeat());
            let (_, metric) = fec::viterbi_decode_with_metric(&pairs);
            // The null: the same values, shuffled. Two shuffles, and the larger
            // is reported, because the null is what the answer has to beat.
            let null = [shuffled(&pairs, 0x1234_5678), shuffled(&pairs, 0x9E37_79B9)]
                .iter()
                .map(|s| fec::viterbi_decode_with_metric(s).1)
                .fold(f32::NEG_INFINITY, f32::max);
            let ratio = if null > 0.0 { metric / null } else { 0.0 };
            println!(
                "  {:>26}  {:>8.0}  {:>8.0}  {:>5.2}x",
                format!("{} sign {sign:+.0}", rate.label()),
                metric,
                null,
                ratio
            );
        }
    }
    println!(
        "  (a ratio near 1 is no coding; the code is there when a branch stands well above its null)"
    );
}
