//! Does handing FT8's subtraction pass the quick pass's results pay?
//!
//! The decode worker runs a plain single pass first and then a subtraction
//! pass over the same audio. Without `.known(..)` the slow pass starts from
//! scratch and finds the strong signals all over again; with it, they are
//! subtracted up front. This decodes each recording both ways at each depth
//! and reports how long the slow pass took and what each way found that the
//! other did not, so the choice rests on real recordings rather than a guess.
//!
//! ```text
//! cargo run --release -p sdroxide-digi --example ft8_known_bench -- slot1.wav slot2.wav ...
//! ```
//!
//! Each file is one 15 s slot, 12 kHz mono 16-bit (ft8_lib's test corpus is
//! this shape). A `.txt` beside it in WSJT-X's decode-line format is read as
//! the reference: a decode it holds counts as a hit, one it lacks as an
//! extra — mostly false decodes, sometimes a real signal WSJT-X missed. `FT8_REPS` sets how many times each pass is timed; the fastest
//! is kept (default 3). `RAYON_NUM_THREADS=1` approximates a slow machine.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use mfsk_core::msg::decode_request::DecodeRequest;
use sdroxide_digi::params::{AUDIO_MAX_HZ, AUDIO_MIN_HZ, MAX_CAND, SYNC_MIN};

type Results = Vec<mfsk_core::ft8::decode::DecodeResult>;

fn base(audio: &[i16]) -> DecodeRequest<'_, mfsk_core::Ft8> {
    DecodeRequest::<mfsk_core::Ft8>::new(audio, AUDIO_MIN_HZ, AUDIO_MAX_HZ, SYNC_MIN, MAX_CAND)
        .osd(true)
}

/// Run `f` `reps` times; the fastest time and the last result.
fn timed(reps: usize, f: impl Fn() -> Results) -> (Duration, Results) {
    let mut best = Duration::MAX;
    let mut out = Vec::new();
    for _ in 0..reps {
        let t = Instant::now();
        out = f();
        best = best.min(t.elapsed());
    }
    (best, out)
}

fn texts(r: &Results) -> BTreeSet<String> {
    r.iter()
        .map(|d| mfsk_core::msg::wsjt77::unpack77(&d.message77()).unwrap_or_else(|| "?".into()))
        .collect()
}

fn main() {
    let reps: usize = std::env::var("FT8_REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    // depth → slow pass ms, reference hits and extras; [without, with] known.
    let mut totals: Vec<(&str, [f64; 2], [usize; 2], [usize; 2])> =
        vec![("Normal", [0.0; 2], [0; 2], [0; 2]), ("Deep", [0.0; 2], [0; 2], [0; 2])];
    let mut ref_n = 0;
    let mut quick_ms = 0.0;
    let mut quick_n = 0;
    for path in std::env::args().skip(1) {
        let mut r = hound::WavReader::open(&path).expect("open wav");
        let spec = r.spec();
        if spec.sample_rate != 12_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
            eprintln!("skip {path}: not 12 kHz mono 16-bit");
            continue;
        }
        let mut audio: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
        audio.resize(180_000, 0);

        // WSJT-X decode lines: "hhmmss  snr  dt  freq ~  MESSAGE".
        let reference: Option<BTreeSet<String>> =
            std::fs::read_to_string(path.replace(".wav", ".txt")).ok().map(|t| {
                t.lines()
                    .filter_map(|l| l.split_once('~').map(|(_, m)| m.trim().to_string()))
                    .filter(|m| !m.is_empty())
                    .collect()
            });
        ref_n += reference.as_ref().map_or(0, BTreeSet::len);
        let score = |seen: &BTreeSet<String>| match &reference {
            Some(r) => (seen.intersection(r).count(), seen.difference(r).count()),
            None => (0, 0),
        };
        let (tq, quick) = timed(reps, || base(&audio).decode().results);
        quick_ms += tq.as_secs_f64() * 1e3;
        quick_n += quick.len();
        let quick_set = texts(&quick);
        let name = path.rsplit('/').next().unwrap_or(&path);
        print!("{name:<22} quick {:>3} in {:>6.1} ms", quick.len(), tq.as_secs_f64() * 1e3);

        for (depth, ms, hits, extras) in totals.iter_mut() {
            let req = |known: &[_]| {
                let r = base(&audio).known(known);
                let r = if *depth == "Deep" { r.sic_early() } else { r.sic_rounds(2) };
                r.decode().results
            };
            let (t0, without) = timed(reps, || req(&[]));
            let (t1, with) = timed(reps, || req(&quick));
            // What the operator sees: the quick pass plus whatever the slow
            // one adds on top of it.
            let seen0: BTreeSet<String> = quick_set.union(&texts(&without)).cloned().collect();
            let seen1: BTreeSet<String> = quick_set.union(&texts(&with)).cloned().collect();
            ms[0] += t0.as_secs_f64() * 1e3;
            ms[1] += t1.as_secs_f64() * 1e3;
            let (s0, s1) = (score(&seen0), score(&seen1));
            hits[0] += s0.0;
            hits[1] += s1.0;
            extras[0] += s0.1;
            extras[1] += s1.1;
            print!(
                " | {depth} {:>6.1} → {:>6.1} ms, {:>3} → {:>3}",
                t0.as_secs_f64() * 1e3,
                t1.as_secs_f64() * 1e3,
                seen0.len(),
                seen1.len()
            );
            let lost: Vec<_> = seen0.difference(&seen1).collect();
            let gained: Vec<_> = seen1.difference(&seen0).collect();
            if !lost.is_empty() || !gained.is_empty() {
                print!(" [{depth} lost {lost:?} gained {gained:?}]");
            }
        }
        println!();
    }
    println!("\nquick pass: {quick_n} decodes, {quick_ms:.0} ms total; reference: {ref_n}");
    for (depth, ms, hits, extras) in &totals {
        println!(
            "{depth:<6} slow pass {:>7.0} → {:>7.0} ms ({:+.0}%), reference hits {} → {}, \
             extras {} → {}",
            ms[0],
            ms[1],
            (ms[1] / ms[0] - 1.0) * 100.0,
            hits[0],
            hits[1],
            extras[0],
            extras[1]
        );
    }
}
