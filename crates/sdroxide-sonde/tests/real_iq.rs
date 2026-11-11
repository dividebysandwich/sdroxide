//! Real-recording check. Ignored by default: set `SONDE_IQ_WAV` to a 16-bit
//! stereo (I/Q) WAV and run `cargo test --release -p sdroxide-sonde --test real_iq -- --ignored --nocapture`.
//! Sweeps the carrier offset and prints what decodes.

use sdroxide_dsp::Complex32 as Complex;
use sdroxide_sonde::{SondeModel, decoder_for};

fn read_wav(path: &str, max_secs: f64) -> (f64, Vec<Complex>) {
    let b = std::fs::read(path).expect("read wav");
    let rate = u32::from_le_bytes(b[24..28].try_into().unwrap()) as f64;
    let mut p = 12;
    while &b[p..p + 4] != b"data" {
        p += 8 + u32::from_le_bytes(b[p + 4..p + 8].try_into().unwrap()) as usize;
    }
    let len = u32::from_le_bytes(b[p + 4..p + 8].try_into().unwrap()) as usize;
    let data = &b[p + 8..(p + 8 + len).min(b.len())];
    let n = ((max_secs * rate) as usize).min(data.len() / 4);
    let iq = (0..n)
        .map(|i| {
            let re = i16::from_le_bytes([data[4 * i], data[4 * i + 1]]) as f32 / 32768.0;
            let im = i16::from_le_bytes([data[4 * i + 2], data[4 * i + 3]]) as f32 / 32768.0;
            Complex::new(re, im)
        })
        .collect();
    (rate, iq)
}

/// Raw interleaved float32 I/Q (the projecthorus sample set: 96 kS/s, centred).
fn read_f32(path: &str, rate: f64, max_secs: f64) -> (f64, Vec<Complex>) {
    let b = std::fs::read(path).expect("read bin");
    let n = ((max_secs * rate) as usize).min(b.len() / 8);
    let iq = (0..n)
        .map(|i| {
            let f = |o: usize| f32::from_le_bytes(b[8 * i + o..8 * i + o + 4].try_into().unwrap());
            Complex::new(f(0), f(4))
        })
        .collect();
    (rate, iq)
}

/// A projecthorus-style sample (raw float32 I/Q, 96 kS/s, centred), decoded by
/// the model named in `SONDE_MODEL` (`RS41` by default, or `DFM`).
#[test]
#[ignore]
fn real_f32_sample_decodes() {
    let path = std::env::var("SONDE_IQ_F32").expect("SONDE_IQ_F32");
    let model = match std::env::var("SONDE_MODEL").as_deref() {
        Ok("DFM") => SondeModel::Dfm,
        Ok("M10") => SondeModel::M10,
        Ok("M20") => SondeModel::M20,
        Ok("IMET4") => SondeModel::Imet4,
        Ok("MRZ") => SondeModel::Mrz,
        Ok("LMS6") => SondeModel::Lms6,
        Ok("MTS01") => SondeModel::Mts01,
        Ok("IMET54") => SondeModel::Imet54,
        Ok("CF06") => SondeModel::Cf06,
        Ok("WEATHEX") => SondeModel::Weathex,
        Ok("C34") => SondeModel::C34,
        Ok("MEISEI") => SondeModel::Meisei,
        Ok("MK2A") => SondeModel::Mk2a,
        _ => SondeModel::Rs41,
    };
    let rate0 = std::env::var("SONDE_RATE").ok().and_then(|v| v.parse().ok()).unwrap_or(96_000.0);
    let (rate, iq) = read_f32(&path, rate0, 120.0);
    let offset = std::env::var("SONDE_OFFSET_HZ").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let mut dec = decoder_for(model, rate, offset).unwrap();
    let mut all = Vec::new();
    for chunk in iq.chunks(4096) {
        all.extend(dec.process(chunk));
    }
    let mut serials: Vec<_> = all.iter().map(|f| f.serial.clone()).collect();
    serials.sort();
    serials.dedup();
    println!("{} fixes, serials {serials:?}", all.len());
    if std::env::var("SONDE_VERBOSE").is_ok() {
        for f in &all {
            if let Some(p) = f.pos.as_ref() {
                println!(
                    "[{:3}] lat {:.6} lon {:.6} alt {:.1} vH {:.2} D {:.1} vV {:.2} sv {}",
                    f.frame_num,
                    p.lat,
                    p.lon,
                    p.alt_m,
                    p.speed_ms,
                    p.course_deg,
                    p.climb_ms,
                    p.num_sv
                );
            }
        }
    }
    if let Some(p) = all.first().and_then(|f| f.pos.as_ref()) {
        println!("first: lat {:.5} lon {:.5} alt {:.1} m", p.lat, p.lon, p.alt_m);
    }
    if let Some(p) = all.last().and_then(|f| f.pos.as_ref()) {
        println!("last:  lat {:.5} lon {:.5} alt {:.1} m", p.lat, p.lon, p.alt_m);
    }
    if std::env::var("SONDE_VERBOSE").is_ok() {
        for f in &all {
            let m = f.meteo;
            println!(
                "[{:3}] T {:?} RH {:?} P {:?}",
                f.frame_num, m.temp_c, m.humidity_pct, m.pressure_hpa
            );
        }
    }
    println!(
        "{} with a position, {} with a temperature, {} with a humidity",
        all.iter().filter(|f| f.pos.is_some()).count(),
        all.iter().filter(|f| f.meteo.temp_c.is_some()).count(),
        all.iter().filter(|f| f.meteo.humidity_pct.is_some()).count()
    );
    println!("{} with a position", all.iter().filter(|f| f.pos.is_some()).count());
    assert!(!all.is_empty());
}

#[test]
#[ignore]
fn real_recording_sweep() {
    let path = std::env::var("SONDE_IQ_WAV").expect("SONDE_IQ_WAV");
    let (rate0, iq0) = read_wav(
        &path,
        std::env::var("SONDE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(60.0),
    );
    // The decoder wants at least 48 kS/s: upsample x3 with linear interpolation.
    let rate = rate0 * 3.0;
    let mut iq = Vec::with_capacity(iq0.len() * 3);
    for w in iq0.windows(2) {
        for k in 0..3 {
            let t = k as f32 / 3.0;
            iq.push(w[0] * (1.0 - t) + w[1] * t);
        }
    }
    println!("{} samples at {rate} Hz", iq.len());
    let mut best = (0usize, 0.0f64);
    let (lo, hi) = match std::env::var("SONDE_OFFSET").ok().and_then(|v| v.parse::<f64>().ok()) {
        Some(o) => (o, o),
        None => (-14_000.0, 14_000.0),
    };
    let mut off = lo;
    while off <= hi {
        let mut dec = decoder_for(SondeModel::Rs41, rate, off).unwrap();
        let mut n = 0;
        let mut all: Vec<(String, u32, Option<f64>)> = Vec::new();
        let mut first = None;
        for chunk in iq.chunks(4096) {
            for d in dec.process(chunk) {
                n += 1;
                all.push((d.serial.clone(), d.frame_num, d.pos.as_ref().map(|p| p.alt_m)));
                if first.is_none() {
                    first = Some(format!(
                        "{} #{} {:?} pos={:?}",
                        d.serial, d.frame_num, d.battery_v, d.pos
                    ));
                }
            }
        }
        if n > 0 {
            println!("offset {off:+.0} Hz: {n} frames, first {first:?}");
        }
        if lo == hi && !all.is_empty() {
            let mut serials: Vec<_> = all.iter().map(|a| a.0.clone()).collect();
            serials.sort();
            serials.dedup();
            let f0 = all.iter().map(|a| a.1).min().unwrap();
            let f1 = all.iter().map(|a| a.1).max().unwrap();
            let alts: Vec<f64> = all.iter().filter_map(|a| a.2).collect();
            let missing = (f1 - f0 + 1) as usize - all.len().min((f1 - f0 + 1) as usize);
            println!(
                "serials {serials:?}; {} frames, counter {f0}..{f1} ({missing} missing); alt {:.0}..{:.0} m ({} with a fix)",
                all.len(),
                alts.iter().cloned().fold(f64::MAX, f64::min),
                alts.iter().cloned().fold(f64::MIN, f64::max),
                alts.len()
            );
        }
        if n > best.0 {
            best = (n, off);
        }
        off += 500.0;
    }
    println!("best: {} frames at {:+.0} Hz", best.0, best.1);
}
