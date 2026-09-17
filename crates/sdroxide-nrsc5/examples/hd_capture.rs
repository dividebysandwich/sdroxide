//! Decode a recorded FM channel through `HdDemod`, off a capture made by
//! `sdroxide --record-iq` at 4x the FM rate (2,976,750 S/s).
//!
//!     cargo run --release -p sdroxide-nrsc5 --example hd_capture -- \
//!         cap.cf32 <capture centre Hz> <channel Hz> [capture rate] [programme]
//!
//! The capture rate defaults to 2,976,750 S/s — four times the decoder's own
//! rate, which is what an `sdroxide --record-iq` file of an FM band usually is.
//! Pass 744187.5 for a clip already cut to one channel at the decoder's rate,
//! and the shift and decimation fall away.
//!
//! Prints what the decoder reports so a real station can be checked without a
//! GUI: lock, per-sideband MER, CBER, the station's identity, the stereo side
//! channel's level, how much audio came out, and the backlog drop count (which
//! must be zero — a climbing one is a queue drained slower than the decoder
//! fills it).
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};

use num_complex::Complex32;
use sdroxide_dsp::Demodulator;
use sdroxide_nrsc5::HdDemod;
use sdroxide_nrsc5::demod::FM_RATE_HZ;

const TAPS: usize = 255;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sdroxide_nrsc5=debug".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let a: Vec<String> = env::args().skip(1).collect();
    let (path, centre, chan) = (&a[0], a[1].parse::<f64>().unwrap(), a[2].parse::<f64>().unwrap());
    // The capture rate is not fixed: `--record-iq` files come at whatever the
    // front end was running, and a clip cut for the test is already at the
    // decoder's own rate. Whole multiples only — that is what a capture made
    // for this is, and a fractional one belongs in the engine's resampler
    // rather than in a bench tool.
    let cap_rate = a.get(3).and_then(|s| s.parse::<f64>().ok()).unwrap_or(4.0 * FM_RATE_HZ);
    let decim = (cap_rate / FM_RATE_HZ).round().max(1.0) as u64;
    let program: u8 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut demod = HdDemod::new(FM_RATE_HZ);
    demod.select_program(program);

    // Windowed-sinc low-pass, flat to 200 kHz, down by the 372 kHz where the
    // decimated band folds.
    let mut h = [0f32; TAPS];
    let (mut sum, fc) = (0f64, 286e3 / cap_rate);
    for (i, t) in h.iter_mut().enumerate() {
        let k = i as f64 - (TAPS / 2) as f64;
        let sinc = if k == 0.0 {
            2.0 * fc
        } else {
            (2.0 * std::f64::consts::PI * fc * k).sin() / (std::f64::consts::PI * k)
        };
        let w = 0.42 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (TAPS - 1) as f64).cos()
            + 0.08 * (4.0 * std::f64::consts::PI * i as f64 / (TAPS - 1) as f64).cos();
        *t = (sinc * w) as f32;
        sum += (sinc * w) as f64;
    }
    for t in h.iter_mut() {
        *t /= sum as f32;
    }

    // `.cs16` is interleaved 16-bit; anything else is the CF32 `--record-iq`
    // writes.
    let cs16 = path.ends_with(".cs16");
    let mut rd = BufReader::new(File::open(path).expect("open the capture"));
    let (mut hist, mut pos) = ([Complex32::new(0.0, 0.0); TAPS], 0usize);
    let (mut phase, dphi) = (0f64, -2.0 * std::f64::consts::PI * (chan - centre) / cap_rate);
    let (mut n, mut audio_samples, mut best) =
        (0u64, 0usize, None::<sdroxide_types::HdRadioStatus>);
    let (mut raw, mut chan_iq, mut out) = (vec![0u8; 1 << 18], Vec::new(), Vec::new());
    let (mut side_energy, mut side_n) = (0f64, 0usize);
    let mut side: Vec<f32> = Vec::new();
    let dump = env::var("HD_OUT").is_ok();
    let mut dumped: Vec<f32> = Vec::new();

    while let Ok(got) = rd.read(&mut raw) {
        if got < 8 {
            break;
        }
        chan_iq.clear();
        let step = if cs16 { 4 } else { 8 };
        for s in raw[..got - got % step].chunks_exact(step) {
            // 16-bit pairs where the file says so: a clip cut for a test is
            // half the size that way, and the front end's own words are 14-bit.
            let (re, im) = if cs16 {
                (
                    f32::from(i16::from_le_bytes([s[0], s[1]])) / 32_768.0,
                    f32::from(i16::from_le_bytes([s[2], s[3]])) / 32_768.0,
                )
            } else {
                (
                    f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                    f32::from_le_bytes([s[4], s[5], s[6], s[7]]),
                )
            };
            let (sn, cs) = phase.sin_cos();
            hist[pos] = Complex32::new(re, im) * Complex32::new(cs as f32, sn as f32);
            pos = (pos + 1) % TAPS;
            phase += dphi;
            n += 1;
            if n % decim == 0 {
                let mut y = Complex32::new(0.0, 0.0);
                for (t, c) in h.iter().enumerate() {
                    y += hist[(pos + t) % TAPS] * *c;
                }
                chan_iq.push(y);
            }
        }
        out.clear();
        demod.process(&chan_iq, &mut out);
        audio_samples += out.len();
        if dump {
            dumped.extend_from_slice(&out);
        }
        side.clear();
        if demod.take_side(&mut side) {
            side_energy += side.iter().map(|v| (v * v) as f64).sum::<f64>();
            side_n += side.len();
        }
        if let Some(s) = demod.take_hd_radio() {
            if s.locked {
                best = Some(s);
            }
        }
    }

    println!("\n{:.1} MHz, programme HD{}", chan / 1e6, program + 1);
    match best {
        None => println!("  never locked"),
        Some(s) => {
            println!("  locked, audio {}", if s.audio { "yes" } else { "no" });
            println!("  MER {:.1} / {:.1} dB, CBER {:.4}", s.mer_lower_db, s.mer_upper_db, s.cber);
            println!("  carrier offset {:.1} Hz, PSMI {}", s.freq_offset_hz, s.psmi);
            println!("  station {:?} / {:?}", s.station_name, s.station_slogan);
            println!(
                "  services: {:?}",
                s.audio_services.iter().map(|a| a.program).collect::<Vec<_>>()
            );
        }
    }
    if side_n > 0 {
        println!(
            "  stereo side channel: RMS {:.4} over {} samples",
            (side_energy / side_n as f64).sqrt(),
            side_n
        );
    } else {
        println!("  stereo side channel: nothing offered");
    }
    println!(
        "  {} values out = {:.1} s if mono at 44.1 kHz, {:.1} s if stereo",
        audio_samples,
        audio_samples as f64 / 44_100.0,
        audio_samples as f64 / 2.0 / 44_100.0
    );
    // Zero on a healthy decode. A climbing count is a queue drained slower than
    // the decoder fills it — the shape the stereo-pair pacing bug took, and the
    // assertion the SDROXIDE_HD_SAMPLE test makes.
    println!("  backlog drops: {}", demod.backlog_drops());
    if let Ok(path) = env::var("HD_OUT") {
        let pcm: Vec<u8> =
            dumped.iter().flat_map(|v| ((v * 32_767.0) as i16).to_le_bytes()).collect();
        let n = pcm.len();
        std::fs::write(&path, pcm).expect("write the raw audio");
        println!("  wrote {n} bytes of raw i16 to {path}");
    }
}
