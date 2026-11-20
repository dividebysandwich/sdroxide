//! Push a recording through the decoder and print what comes out.
//!
//! ```text
//! cargo run --release -p sdroxide-adsb --example adsb_replay -- capture.iq [rate]
//! ```
//!
//! The file is interleaved little-endian `f32` pairs (CF32) — what
//! `sdroxide --record-iq` writes and what the `adsb_iq` example generates — at
//! 2.4 Msps unless a rate is given.
//!
//! This is the tool that answers the only question that matters about a decoder
//! like this: *does it work on your air*. It prints every message it accepted
//! and, at the end, the aircraft table it built, so the output can be put
//! beside another decoder's on the same recording. A count of preambles that
//! produced nothing is printed too, because "the band is busy with something
//! else" and "the receiver is deaf" are different problems and the ratio is what
//! tells them apart.

use std::io::Read;

use sdroxide_adsb::{Demod, Tracker, accept, frame::Rejected};
use sdroxide_dsp::Complex32;
use sdroxide_types::AdsbSettings;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: adsb_replay <capture.iq> [sample-rate-hz]");
        std::process::exit(2);
    };
    let rate: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2_400_000.0);
    if rate < sdroxide_types::ADSB_MIN_RATE_HZ {
        eprintln!(
            "warning: {:.3} Msps is below the {:.1} Msps Mode S needs; expect nothing to decode",
            rate / 1e6,
            sdroxide_types::ADSB_MIN_RATE_HZ / 1e6
        );
    }

    let mut f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("cannot open {path}: {e}"));
    let mut demod = Demod::new(rate);
    // A long window and no ceiling: a replay wants everything the recording
    // contained, not a live display's forgetting.
    let mut tracker = Tracker::new(AdsbSettings {
        drop_map_s: 600,
        drop_list_s: 3600,
        history_points: 0,
        max_aircraft: 2000,
        ..AdsbSettings::default()
    });

    let mut raw = vec![0u8; 1 << 20];
    let mut iq: Vec<Complex32> = Vec::new();
    let mut cand = Vec::new();
    let (mut frames, mut bad, mut unmatched, mut samples) = (0u64, 0u64, 0u64, 0u64);

    loop {
        let n = f.read(&mut raw).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        if n < 8 {
            break;
        }
        iq.clear();
        for c in raw[..n - n % 8].chunks_exact(8) {
            iq.push(Complex32::new(
                f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
            ));
        }
        // The stream clock, in seconds: a replay must not read the wall clock
        // for anything, or the same recording decodes differently depending on
        // how fast the disk is.
        let mono = samples as f64 / rate;
        samples += iq.len() as u64;
        let now = samples as i64 / rate as i64;

        cand.clear();
        demod.push(&iq, &mut cand);
        for c in &cand {
            match accept(&c.bytes, |icao| tracker.knows(icao, now)) {
                Ok(a) => {
                    frames += 1;
                    let hex: String = c.bytes.iter().map(|b| format!("{b:02X}")).collect();
                    println!(
                        "{:8.3}s  DF{:<2} {:06X}  {:>6.1} dBFS  conf {:.2}  {hex}",
                        mono,
                        a.message().df,
                        a.icao(),
                        c.rssi_dbfs,
                        c.confidence,
                    );
                    tracker.absorb(&a, &c.bytes, c.rssi_dbfs, None, now, mono);
                }
                Err(Rejected::BadCrc | Rejected::Malformed) => bad += 1,
                Err(Rejected::Unmatched) => unmatched += 1,
                Err(Rejected::Unsupported) => {}
            }
        }
    }

    let secs = samples as f64 / rate;
    println!();
    println!(
        "{secs:.1} s at {:.3} Msps: {} preambles, {frames} frames, {bad} failed their check, \
         {unmatched} replies from unknown addresses",
        rate / 1e6,
        demod.preambles
    );
    if demod.preambles > 0 && frames == 0 {
        println!(
            "  the correlator is firing but nothing checks out — either the band is busy with \
             something that is not Mode S, or the receiver is not on 1090 MHz"
        );
    }
    println!();

    let mut table = tracker.snapshot();
    table.sort_by_key(|a| std::cmp::Reverse(a.frames));
    println!(
        "{:<8} {:<9} {:>7} {:>5} {:>4} {:>6} {:>7}  position",
        "ICAO", "CALL", "ALT", "GS", "TRK", "FRAMES", "SIG"
    );
    for a in &table {
        println!(
            "{:<8} {:<9} {:>7} {:>5} {:>4} {:>6} {:>6.0}  {}",
            a.hex(),
            a.callsign,
            a.altitude_ft.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            a.ground_speed_kt.map(|v| format!("{v:.0}")).unwrap_or_else(|| "-".into()),
            a.track_deg.map(|v| format!("{v:.0}")).unwrap_or_else(|| "-".into()),
            a.frames,
            a.rssi_dbfs,
            match a.lat.zip(a.lon) {
                Some((lat, lon)) => format!("{lat:.4}, {lon:.4}"),
                None => "-".to_string(),
            }
        );
    }
    println!("\n{} aircraft", table.len());
}
