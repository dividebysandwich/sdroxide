//! Runs a synthetic station through the decoder and prints what it found.
//! `cargo run -p sdroxide-timesig --release --example timesig_probe -- dcf77 -15 187 40 [seed] [seconds]`
use sdroxide_timesig::{Decoder, Event, synth::Synth};
use sdroxide_types::TimeStation;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let station = match args.get(1).map(String::as_str).unwrap_or("dcf77") {
        "msf" => TimeStation::Msf,
        "wwvb" => TimeStation::Wwvb,
        "jjy" => TimeStation::Jjy60,
        "jjy40" => TimeStation::Jjy40,
        "wwv" => TimeStation::Wwv,
        "wwvh" => TimeStation::Wwvh,
        _ => TimeStation::Dcf77,
    };
    let snr: Option<f64> = args.get(2).and_then(|s| s.parse().ok());
    let off: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let ppm: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let rate = 12_000.0;
    let mut s = Synth::new(station, rate);
    s.snr_db = snr;
    s.tone_hz += off;
    s.rate_error_ppm = ppm;
    s.seed = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(3);
    let start = sdroxide_types::ymd_hms_to_unix(2026, 10, 18, 12, 36, 23) as f64 + 0.4;
    let secs: f64 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(200.0);
    let audio = s.render(start, secs);
    let mut dec = Decoder::new(station, rate);
    let mut ev = Vec::new();
    for (i, block) in audio.chunks(240).enumerate() {
        dec.process(block, &mut ev);
        for e in ev.drain(..) {
            let t = start + i as f64 * 0.02;
            match e {
                Event::Second(sec) => {
                    let st = start + sec.pos / rate;
                    print!(
                        "{}",
                        match sec.sym {
                            None => "?".to_string(),
                            Some(s) if s.marker => "M".to_string(),
                            Some(s) => format!("{}", s.code()),
                        }
                    );
                    if (st.rem_euclid(60.0)) > 58.5 {
                        println!("  [{:.3}]", st.rem_euclid(60.0));
                    }
                }
                Event::Frame { pos, frame, code } => println!(
                    "\nFRAME {code:?} utc={} err={:.3}ms",
                    frame.utc,
                    (start + pos / rate - frame.utc as f64) * 1e3
                ),
                Event::BadFrame { error, .. } => println!("\nBAD {error:?}"),
                Event::Locked => {
                    let sn = dec.snapshot();
                    println!(
                        "\nLOCKED t={:.1} z={:.2} carrier={:?}",
                        t - start,
                        dec.edge_z(),
                        sn.carrier_hz
                    )
                }
                Event::LockLost => println!("\nLOST t={:.1}", t - start),
            }
        }
    }
    let sn = dec.snapshot();
    println!(
        "\nend: locked={} z={:.2} carrier={:?} snr={:.1} phase snr={:?} sibling z={:?}",
        sn.locked,
        dec.edge_z(),
        sn.carrier_hz,
        sn.snr_db,
        sn.phase_snr_db,
        sn.sibling_z
    );
}
