//! How early in a slot can FT8's quick pass run and still find the band?
//!
//! An FT8 transmission nominally ends 13.1 s into its 15 s slot, but the
//! decoder is handed nothing until the slot boundary. This cuts each recording
//! off at a range of points (zeroing the rest, as a live buffer that has not
//! filled yet would be) and runs the quick single pass on what is left,
//! reporting how much of the whole-slot quick pass each cut still finds and
//! what it finds that the whole slot did not.
//!
//! ```text
//! cargo run --release -p sdroxide-digi --example ft8_early_bench -- slot1.wav ...
//! ```
//!
//! Each file is one 15 s slot, 12 kHz mono 16-bit. Recordings off a WebSDR
//! run late — ft8_lib's corpus has a median DT near +0.9 s, where a band of
//! stations on synced clocks sits near 0 — so when a WSJT-X `.txt` lies beside
//! the file, the audio is first moved so its median DT is 0. Without that,
//! every cut would look a second worse than it is on the air.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use mfsk_core::decoder::{ApMode, DecodeParams, Decoder, Depth, Ft8Strategy, SlotInput};
use sdroxide_digi::params::{AUDIO_MAX_HZ, AUDIO_MIN_HZ, MAX_CAND, SYNC_MIN};

const RATE: f64 = 12_000.0;
const SLOT: usize = 180_000;
const CUTS: [f64; 7] = [11.8, 12.4, 12.8, 13.2, 13.5, 14.0, 14.4];

/// The quick pass as the decode worker runs it: one pass, our search.
fn quick(audio: &[i16]) -> (Duration, BTreeSet<String>) {
    let mut d = Decoder::<mfsk_core::Ft8>::new(
        DecodeParams::for_band((AUDIO_MIN_HZ, AUDIO_MAX_HZ)).depth(Depth::Deep).ap(ApMode::Off),
    );
    let t = &mut d.extras_mut().tuning;
    t.sync_min = Some(SYNC_MIN);
    t.max_cand = Some(MAX_CAND);
    t.osd = Some(true);
    t.strategy = Some(Ft8Strategy::SinglePass);
    let t = Instant::now();
    let r = d.decode(&SlotInput::i16(audio)).rows;
    let el = t.elapsed();
    let set = r
        .iter()
        .map(|row| {
            mfsk_core::msg::wsjt77::unpack77(row.native.message77()).unwrap_or_else(|| "?".into())
        })
        .collect();
    (el, set)
}

fn main() {
    let mut whole_n = 0;
    let mut found = [0usize; CUTS.len()];
    let mut new = [0usize; CUTS.len()];
    let mut ms = [0f64; CUTS.len()];
    for path in std::env::args().skip(1) {
        let mut r = hound::WavReader::open(&path).expect("open wav");
        let spec = r.spec();
        if spec.sample_rate != 12_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
            eprintln!("skip {path}: not 12 kHz mono 16-bit");
            continue;
        }
        let mut audio: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
        // WSJT-X decode lines: "hhmmss  snr  dt  freq ~  MESSAGE".
        let mut dts: Vec<f64> = std::fs::read_to_string(path.replace(".wav", ".txt"))
            .map(|t| t.lines().filter_map(|l| l.split_whitespace().nth(2)?.parse().ok()).collect())
            .unwrap_or_default();
        dts.sort_by(f64::total_cmp);
        let shift = dts.get(dts.len() / 2).copied().unwrap_or(0.0);
        let skip = ((shift * RATE).round().max(0.0) as usize).min(audio.len());
        audio.drain(..skip);
        audio.resize(SLOT, 0);

        let (_, whole) = quick(&audio);
        whole_n += whole.len();
        let name = path.rsplit('/').next().unwrap_or(&path);
        print!("{name:<22} shift {shift:+.1}s whole {:>3} |", whole.len());
        for (i, cut) in CUTS.iter().enumerate() {
            let mut a = audio.clone();
            a[(cut * RATE) as usize..].fill(0);
            let (t, got) = quick(&a);
            found[i] += got.intersection(&whole).count();
            new[i] += got.difference(&whole).count();
            ms[i] += t.as_secs_f64() * 1e3;
            print!(" {cut}s {:>3}", got.intersection(&whole).count());
            let extra: Vec<_> = got.difference(&whole).collect();
            if !extra.is_empty() {
                print!(" {extra:?}");
            }
        }
        println!();
    }
    println!("\nwhole-slot quick pass: {whole_n} decodes");
    for (i, cut) in CUTS.iter().enumerate() {
        println!(
            "cut at {cut:>4} s: {:>4} of them ({:>5.1}%), {:>3} not in the whole slot, {:>6.0} ms",
            found[i],
            100.0 * found[i] as f64 / whole_n.max(1) as f64,
            new[i],
            ms[i]
        );
    }
}
