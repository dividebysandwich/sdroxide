//! Whether a level referred back to the antenna holds still while the front
//! end's gain moves, measured against an injected carrier of known level.
//!
//! ```sh
//! # defaults match the bench setup: -73 dBm at 7295 kHz on Antenna A
//! cargo run --release -p sdroxide-sdrplay --example gain_probe
//! cargo run --release -p sdroxide-sdrplay --example gain_probe -- 7295000 -73 "Antenna A"
//! ```
//!
//! Measures the **carrier bin**, not the whole span. That distinction is the
//! whole point: a −73 dBm carrier is far below the receiver's own noise taken
//! over 2 MHz, so a wideband power measurement reports the noise floor and
//! says nothing about the signal. The S-meter measures a demodulated passband
//! a few kHz wide, and this measures narrower still, so what comes out is the
//! carrier and not the band it sits in.
//!
//! For each LNA state it prints the carrier level in dBFS, the system gain the
//! API reports, the difference (what an antenna-referred S-meter would read in
//! dBFS-at-the-antenna), and the error against the generator's known level.
//! The `referred` column is supposed to stay put; the `error` column is what
//! `cal_offset_db` would have to be to make the reading absolute.

use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use sdroxide_sdrplay::SdrPlayHandle;
use sdroxide_types::{SdrPlayAgc, SdrPlayConfig};

/// Samples per coherent block. At 2 Msps this is a 488 Hz bin — narrow enough
/// to put the carrier well clear of the noise, wide enough that a few ppm of
/// reference error does not rotate the accumulation apart inside one block.
const BLOCK: usize = 4096;
/// How far either side of the expected offset to look for the carrier, in
/// bins, so a reference error of a few ppm is found rather than missed.
const SEARCH_BINS: i32 = 6;
/// Where the carrier is parked relative to the tuned centre. Off DC on
/// purpose: a zero-IF front end piles LO leakage and flicker noise there.
const CARRIER_OFFSET_HZ: f64 = 40_000.0;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sdroxide_sdrplay=warn".into()),
        )
        .init();

    let mut args = std::env::args().skip(1);
    let carrier_hz: f64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(7_295_000.0);
    let known_dbm: f64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(-73.0);
    let antenna = args.next().unwrap_or_else(|| "Antenna A".into());

    let devices = sdroxide_sdrplay::list();
    let Some(dev) = devices.first() else {
        eprintln!("no SDRplay receiver found");
        return;
    };
    println!("using {}", dev.label());
    println!("reference: {:.3} kHz carrier at {known_dbm} dBm on {antenna}", carrier_hz / 1000.0);

    // Tune below the carrier so it lands at a known offset, clear of DC.
    let center = carrier_hz - CARRIER_OFFSET_HZ;
    let cfg = SdrPlayConfig {
        serial: dev.serial.clone(),
        sample_rate_hz: 2_000_000.0,
        agc: SdrPlayAgc::Off,
        if_gr_db: 40,
        lna_state: 0,
        antenna: antenna.clone(),
        ..SdrPlayConfig::default()
    };

    let mut h = match sdroxide_sdrplay::spawn(&cfg, center) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("could not open the receiver: {e}");
            return;
        }
    };
    // The port is a control like any other, and the config above only reaches
    // the session that opened the device.
    h.set_antenna(&antenna);

    println!("\n=== AGC off (manual IF gain, 40 dB) ===\n");
    h.set_agc(SdrPlayAgc::Off.code() as i32);
    h.set_if_gr_db(40);
    sweep(&mut h, known_dbm);

    println!("\n=== extended IF range: gRdB below the API's 20 dB floor ===\n");
    h.set_agc(SdrPlayAgc::Off.code() as i32);
    h.set_lna_state(9);
    println!("{:>5}  {:>11}  {:>9}  {:>12}", "IFGR", "carrier dBFS", "currGain", "referred");
    println!("{}", "-".repeat(42));
    h.set_extended_if_gr(true);
    for gr in [0i32, 5, 10, 15, 20, 25] {
        h.set_if_gr_db(gr);
        if let Some((dbfs, curr)) = measure(&mut h) {
            println!("{gr:>5}  {dbfs:>11.2}  {curr:>9.2}  {:>12.2}", dbfs - curr);
        }
    }
    h.set_extended_if_gr(false);
    h.set_if_gr_db(40);
}

fn sweep(h: &mut SdrPlayHandle, known_dbm: f64) {
    println!(
        "{:>3}  {:>4}  {:>11}  {:>9}  {:>10}  {:>9}  {}",
        "LNA", "IFGR", "carrier dBFS", "gain dB", "referred", "error", ""
    );
    println!("{}", "-".repeat(58));
    let mut referred = Vec::new();
    for state in [0u8, 3, 6, 9, 12, 15, 18] {
        h.set_lna_state(state);
        let Some((carrier_dbfs, gain)) = measure(h) else {
            println!("{state:>3}   ---  (no carrier found)");
            continue;
        };
        let r = carrier_dbfs - gain;
        referred.push(r);
        println!(
            "{:>3}  {:>4}  {carrier_dbfs:>11.2}  {gain:>9.2}  {r:>10.2}  {:>+9.2}  {}",
            h.lna_state(),
            h.effective_if_gr_db().unwrap_or(0),
            known_dbm - r,
            if h.overloaded() { "ADC OVERLOAD" } else { "" },
        );
    }
    if referred.len() < 2 {
        return;
    }
    let (lo, hi) = referred.iter().fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
    let mean = referred.iter().sum::<f64>() / referred.len() as f64;
    println!("\nreferred spread across the sweep: {:.2} dB  (0 is perfect)", hi - lo);
    println!("cal_offset_db that would make this absolute: {:+.2}", known_dbm - mean);
}

/// Carrier level in dBFS and the system gain that produced it, or `None` if no
/// carrier stood out of the noise.
fn measure(h: &mut SdrPlayHandle) -> Option<(f64, f64)> {
    let rate = h.out_rate_hz();
    let mut buf = vec![0f32; BLOCK * 2];
    // Settle first: a window straddling a gain change belongs to neither side
    // of it, and under AGC the loop needs a moment to find its level again.
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        h.read(&mut buf);
        std::thread::sleep(Duration::from_millis(5));
    }

    // Incoherent average of |X(f)|² over several blocks, at each of a few bins
    // around where the carrier should be. Averaging the power rather than the
    // complex value is what makes a few ppm of reference error harmless: the
    // phase differs from block to block, the magnitude does not.
    let mut acc = vec![0f64; (2 * SEARCH_BINS + 1) as usize];
    let mut blocks = 0usize;
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline && blocks < 64 {
        let got = h.read(&mut buf);
        if got < BLOCK * 2 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        let bin_hz = rate / BLOCK as f64;
        for (k, slot) in acc.iter_mut().enumerate() {
            let offset = CARRIER_OFFSET_HZ + (k as i32 - SEARCH_BINS) as f64 * bin_hz;
            let w = TAU * offset / rate;
            let (mut re, mut im) = (0f64, 0f64);
            for n in 0..BLOCK {
                let (i, q) = (f64::from(buf[2 * n]), f64::from(buf[2 * n + 1]));
                let (s, c) = (-w * n as f64).sin_cos();
                // (i + jq) · e^{-jwn}
                re += i * c - q * s;
                im += i * s + q * c;
            }
            let n = BLOCK as f64;
            *slot += (re * re + im * im) / (n * n);
        }
        blocks += 1;
    }
    if blocks == 0 {
        return None;
    }
    let peak = acc.iter().copied().fold(f64::MIN, f64::max) / blocks as f64;
    Some((10.0 * (peak + 1e-30).log10(), h.system_gain_db().unwrap_or(0.0)))
}
