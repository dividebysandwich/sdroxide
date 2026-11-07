//! What FT8's checkpointed early decode buys at `Deep`, through the modem the
//! decode worker runs.
//!
//! Each recording is decoded two ways. *Boundary*: the whole slot at 15 s, as a
//! slot with no early decode is (the quick pass, then the subtraction pass).
//! *Staged*: the slot so far at checkpoint A (11.8 s) and at checkpoint B with
//! the quick pass (13.5 s), then the boundary decode finishing from there. For
//! each it reports the work left after the boundary — what stands between the
//! slot ending and its last decode — what had been found before the boundary,
//! and the whole, scored against WSJT-X's own decodes where a `.txt` lies beside
//! the file.
//!
//! ```text
//! cargo run --release -p sdroxide-digi --example ft8_staged_bench -- slot1.wav ...
//! ```
//!
//! Each file is one 15 s slot, 12 kHz mono 16-bit. As in `ft8_early_bench`, a
//! recording with a WSJT-X `.txt` is moved to put its median DT at 0 first, as
//! a band of stations on synced clocks sits. `RAYON_NUM_THREADS=1`
//! approximates a slow machine.

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Instant;

use sdroxide_digi::modem::{ApHints, Ft8Modem};
use sdroxide_types::{Decode, Mode};

const RATE: f64 = 12_000.0;
const SLOT: usize = 180_000;

#[derive(Default)]
struct Tally {
    /// Seconds of decoding before the boundary, and after it.
    before_s: f64,
    after_s: f64,
    /// The longest after-boundary time of any one slot.
    worst_after_s: f64,
    early: usize,
    early_hits: usize,
    all: usize,
    hits: usize,
}

impl Tally {
    fn add(&mut self, early: &BTreeSet<String>, all: &BTreeSet<String>, refs: &BTreeSet<String>) {
        self.early += early.len();
        self.early_hits += early.intersection(refs).count();
        self.all += all.len();
        self.hits += all.intersection(refs).count();
    }
}

fn main() {
    let ap = ApHints::default();
    let (mut boundary, mut staged) = (Tally::default(), Tally::default());
    let (mut slots, mut refs_n) = (0, 0);
    for (i, path) in std::env::args().skip(1).enumerate() {
        let mut r = hound::WavReader::open(&path).expect("open wav");
        let spec = r.spec();
        if spec.sample_rate != 12_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
            eprintln!("skip {path}: not 12 kHz mono 16-bit");
            continue;
        }
        let mut audio: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
        let txt = std::fs::read_to_string(path.replace(".wav", ".txt")).unwrap_or_default();
        let mut dts: Vec<f64> =
            txt.lines().filter_map(|l| l.split_whitespace().nth(2)?.parse().ok()).collect();
        dts.sort_by(f64::total_cmp);
        let shift = dts.get(dts.len() / 2).copied().unwrap_or(0.0);
        let skip = ((shift * RATE).round().max(0.0) as usize).min(audio.len());
        audio.drain(..skip);
        audio.resize(SLOT, 0);
        // WSJT-X decode lines: "hhmmss  snr  dt  freq ~  MESSAGE".
        let refs: BTreeSet<String> = txt
            .lines()
            .filter_map(|l| l.split_once('~'))
            .map(|(_, m)| m.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|m| !m.is_empty())
            .collect();
        refs_n += refs.len();
        slots += 1;

        let got = Mutex::new(BTreeSet::new());
        let emit = |d: Vec<Decode>| got.lock().unwrap().extend(d.into_iter().map(|d| d.message));
        let slot = i as i64;

        let mut m = Ft8Modem::new(Mode::Ft8);
        let t = Instant::now();
        m.decode_slot_streaming(&audio, slot, 0, &ap, 1500.0, &|| false, &emit);
        let after = t.elapsed().as_secs_f64();
        boundary.after_s += after;
        boundary.worst_after_s = boundary.worst_after_s.max(after);
        let all = std::mem::take(&mut *got.lock().unwrap());
        boundary.add(&BTreeSet::new(), &all, &refs);

        let mut m = Ft8Modem::new(Mode::Ft8);
        let t = Instant::now();
        for &point in m.early_points() {
            m.decode_slot_early(&audio[..point], slot, 0, &ap, 1500.0, &emit);
        }
        staged.before_s += t.elapsed().as_secs_f64();
        let early = got.lock().unwrap().clone();
        let t = Instant::now();
        m.decode_slot_streaming(&audio, slot, 0, &ap, 1500.0, &|| false, &emit);
        let after = t.elapsed().as_secs_f64();
        staged.after_s += after;
        staged.worst_after_s = staged.worst_after_s.max(after);
        let all = got.into_inner().unwrap();
        staged.add(&early, &all, &refs);
    }
    let n = slots.max(1) as f64;
    println!("{slots} slots, {refs_n} WSJT-X decodes; per-slot means, worst in brackets\n");
    for (name, t) in [("boundary only", &boundary), ("staged", &staged)] {
        println!(
            "{name:<14} before 15 s: {:>4.0} ms, {:>4} found ({:>4} WSJT-X) | after 15 s: {:>4.0} ms [{:>4.0}] | all: {:>4} ({:>4} WSJT-X)",
            t.before_s / n * 1e3,
            t.early,
            t.early_hits,
            t.after_s / n * 1e3,
            t.worst_after_s * 1e3,
            t.all,
            t.hits
        );
    }
}
