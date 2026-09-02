//! A driveable RSP for bench work: holds one session open and takes commands
//! on stdin, so a script can step an external generator between measurements
//! without paying for a device open each time.
//!
//! ```sh
//! cargo run --release -p sdroxide-sdrplay --example hdr_probe -- <centre_hz>
//! ```
//!
//! Commands, one per line:
//!
//! ```text
//! hdr on|off        the RSPdx's high-dynamic-range path
//! hdrbw 0|1|2|3     its analog filter: 200 kHz, 500 kHz, 1.2 MHz, 1.7 MHz
//! lna <n>           LNA state; prints what the band clamp kept
//! ifgr <n>          IF gain reduction, dB
//! extgr on|off      allow the reduction below the API's 20 dB floor
//! maxlna            the clamp in force — 18 below 2 MHz, 21 on the HDR path
//! measure           one reading: carrier, noise, gain, overload
//! quit
//! ```
//!
//! `measure` reports the level in the carrier bin **and** in a bin well away
//! from it, so one pass gives both ends of the dynamic range: the wanted
//! signal and the floor it is sitting on. Both are narrowband — a bin, not the
//! span — because a -76 dBm carrier is far below the receiver's own noise
//! taken over 2 MHz, and a wideband measurement would report only the noise.
//!
//! The carrier is expected [`CARRIER_OFFSET_HZ`] above the tuned centre. That
//! is not a convenience: HDR is built at fixed centres, so the *receiver* has
//! nowhere else to sit, and a carrier on the centre would land on the zero-IF
//! DC spike. Offsetting the generator instead keeps both happy.

use std::f64::consts::TAU;
use std::io::{BufRead, Write};
use std::time::{Duration, Instant};

use sdroxide_sdrplay::SdrPlayHandle;
use sdroxide_types::{SdrPlayAgc, SdrPlayConfig, SdrPlayHdrBw};

/// Samples per coherent block: a 488 Hz bin at 2 Msps.
const BLOCK: usize = 4096;
/// Bins either side of the expected offset, so a few ppm of reference error is
/// found rather than missed.
const SEARCH_BINS: i32 = 6;
/// Where the generator is parked relative to the tuned centre.
const CARRIER_OFFSET_HZ: f64 = 40_000.0;
/// Where the noise floor is read: far enough from the carrier that its skirts
/// and any spurious product of it do not raise the answer, still inside the
/// analog filter on every HDR setting (the narrowest is 200 kHz).
const NOISE_OFFSET_HZ: f64 = -70_000.0;

fn main() {
    // stdout is the command protocol; anything the driver has to say goes to
    // stderr or it desynchronises the caller.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sdroxide_sdrplay=warn".into()),
        )
        .init();

    let centre: f64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(875_000.0);
    // Which port to open on. Part of the experiment rather than a setting: HDR
    // was at one point suspected of listening on a fixed port whatever the
    // selector said, which measurement disproved — it is deaf on A and C alike.
    let antenna = std::env::args().nth(2).unwrap_or_else(|| "Antenna A".into());
    // Whether HDR is asked for at Init rather than switched on afterwards: the
    // API takes it either way, and whether the hardware agrees is the question.
    let hdr_at_init = std::env::args().nth(3).is_some_and(|a| a == "hdr");
    let rate: f64 = std::env::args().nth(4).and_then(|a| a.parse().ok()).unwrap_or(2_000_000.0);
    // The *tuner's* analog bandwidth, which is a different control from the
    // HDR filter and has never been varied in these experiments. 0 = auto.
    let bw_khz: u32 = std::env::args().nth(5).and_then(|a| a.parse().ok()).unwrap_or(0);

    let devices = sdroxide_sdrplay::list();
    let Some(dev) = devices.first() else {
        eprintln!("no SDRplay receiver found");
        return;
    };

    let cfg = SdrPlayConfig {
        serial: dev.serial.clone(),
        sample_rate_hz: rate,
        bw_khz,
        // Manual throughout: an AGC would absorb the compression this exists
        // to find.
        agc: SdrPlayAgc::Off,
        if_gr_db: 40,
        lna_state: 4,
        antenna: antenna.clone(),
        hdr: hdr_at_init,
        ..SdrPlayConfig::default()
    };

    let mut h = match sdroxide_sdrplay::spawn(&cfg, centre) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("could not open the receiver: {e}");
            return;
        }
    };
    h.set_antenna(&antenna);
    h.set_agc(SdrPlayAgc::Off.code() as i32);
    h.set_if_gr_db(40);

    println!(
        "# {} on {:.3} kHz via {antenna}, hdr_at_init={hdr_at_init}, bw={bw_khz} kHz, \
              carrier expected at +{:.0} kHz",
        dev.label(),
        centre / 1e3,
        CARRIER_OFFSET_HZ / 1e3
    );
    println!("ready");
    let _ = std::io::stdout().flush();

    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(std::result::Result::ok) {
        let mut it = line.split_whitespace();
        let Some(verb) = it.next() else { continue };
        let arg = it.next().unwrap_or("");
        match verb {
            "hdr" => {
                h.set_hdr(arg == "on");
                println!("ok hdr {arg}");
            }
            "hdrbw" => {
                let bw = SdrPlayHdrBw::from_code(arg.parse().unwrap_or(3.0));
                h.set_hdr_bw(bw.code());
                println!("ok hdrbw {} ({})", bw.code(), bw.label());
            }
            "lna" => {
                h.set_lna_state(arg.parse().unwrap_or(0));
                settle(&mut h, 400);
                println!("ok lna {} gain={:.2}", h.lna_state(), h.system_gain_db().unwrap_or(0.0));
            }
            "ifgr" => {
                h.set_if_gr_db(arg.parse().unwrap_or(40));
                settle(&mut h, 400);
                println!(
                    "ok ifgr {} gain={:.2}",
                    h.effective_if_gr_db().unwrap_or(0),
                    h.system_gain_db().unwrap_or(0.0)
                );
            }
            "extgr" => {
                h.set_extended_if_gr(arg == "on");
                println!("ok extgr {arg}");
            }
            "maxlna" => {
                // Ask for a state past the end of any table and read back what
                // the driver kept: the clamp *is* the answer, and asking it
                // this way needs no copy of the table on this side.
                let was = h.lna_state();
                h.set_lna_state(u8::MAX);
                settle(&mut h, 300);
                let top = h.lna_state();
                h.set_lna_state(was);
                settle(&mut h, 300);
                println!("maxlna {top}");
            }
            "measure" => {
                // Optional offset, so a caller can walk the generator across
                // the span and find where the path actually passes.
                let off = arg.parse().unwrap_or(CARRIER_OFFSET_HZ);
                let (carrier, noise, gain, over) = measure(&mut h, off);
                // Everything referred to the antenna, which is the only scale
                // on which the two paths can be compared: their gains differ.
                println!(
                    "meas carrier_dbfs={carrier:.2} noise_dbfs={noise:.2} gain_db={gain:.2} \
                     carrier_ref={:.2} noise_ref={:.2} snr={:.2} overload={}",
                    carrier - gain,
                    noise - gain,
                    carrier - noise,
                    u8::from(over)
                );
            }
            "quit" => break,
            _ => println!("? {verb}"),
        }
        let _ = std::io::stdout().flush();
    }
}

fn settle(h: &mut SdrPlayHandle, ms: u64) {
    let mut buf = vec![0f32; BLOCK * 2];
    let until = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < until {
        h.read(&mut buf);
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Carrier level, noise level, the system gain both were taken with, and
/// whether the converter is overloaded — all in one pass over the same
/// samples, so the two levels cannot drift apart between readings.
fn measure(h: &mut SdrPlayHandle, carrier_offset_hz: f64) -> (f64, f64, f64, bool) {
    let rate = h.out_rate_hz();
    let mut buf = vec![0f32; BLOCK * 2];
    settle(h, 500);

    let bin_hz = rate / BLOCK as f64;
    let mut carrier_acc = vec![0f64; (2 * SEARCH_BINS + 1) as usize];
    let mut noise_acc = 0f64;
    let mut blocks = 0usize;
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline && blocks < 64 {
        let got = h.read(&mut buf);
        if got < BLOCK * 2 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        for (k, slot) in carrier_acc.iter_mut().enumerate() {
            let off = carrier_offset_hz + f64::from(k as i32 - SEARCH_BINS) * bin_hz;
            *slot += bin_power(&buf, off, rate);
        }
        noise_acc += bin_power(&buf, NOISE_OFFSET_HZ, rate);
        blocks += 1;
    }
    let n = blocks.max(1) as f64;
    // The peak of the search, because the carrier is somewhere in it; the
    // noise bin is a single bin and needs no search.
    let carrier = carrier_acc.iter().copied().fold(f64::MIN, f64::max) / n;
    let noise = noise_acc / n;
    (
        10.0 * (carrier + 1e-30).log10(),
        10.0 * (noise + 1e-30).log10(),
        h.system_gain_db().unwrap_or(0.0),
        h.overloaded(),
    )
}

/// Power in one bin, by mixing it to DC and averaging over the block.
fn bin_power(buf: &[f32], offset_hz: f64, rate: f64) -> f64 {
    let w = TAU * offset_hz / rate;
    let (mut re, mut im) = (0f64, 0f64);
    for n in 0..BLOCK {
        let (i, q) = (f64::from(buf[2 * n]), f64::from(buf[2 * n + 1]));
        let (s, c) = (-w * n as f64).sin_cos();
        re += i * c - q * s;
        im += i * s + q * c;
    }
    let n = BLOCK as f64;
    (re * re + im * im) / (n * n)
}
