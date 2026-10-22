//! Watch one frequency until a STANAG 4285 frame appears, then record it.
//!
//! ```text
//! s4285watch 6348000 62500 /tmp/opencode/s4285 [gain] [threshold] [dwell]
//! s4285watch 6348000,5215600,5155000 62500 /tmp/opencode/s4285
//! s4285watch --selftest
//! ```
//!
//! One frequency or a comma-separated list. With a list it dwells `dwell`
//! seconds (default 6) on each in turn and scores that frequency's last four
//! seconds, so a sweep takes about `dwell x frequencies`. That matters because
//! the tapes are intermittent *and* spread over the band: sigidwiki records
//! 4285 anywhere from 1.89 to 22.7 MHz, and the French Navy alone uses a dozen
//! channels.
//!
//! The French Navy test tapes are intermittent and a station can sit idle for
//! hours, so "watch it and record the tape" is the one thing a session cannot do
//! by hand. This does that: it opens the radio, keeps eight seconds in memory,
//! and scores one statistic every second --
//!
//!   **the 106.67 ms frame period** (256 symbols at 2400 baud), against the
//!   median autocorrelation of nearby lags measured on the same buffer.
//!
//! That is the invariant: the sync sequence is transmitted every frame, so a
//! real transmission repeats itself every 106.67 ms whatever its data rate,
//! whether it is coded or not, and whether it is carrying text or idling. Noise
//! does not. The threshold is a ratio against the buffer's own null because a
//! bare correlation means nothing (see `tools/s4285-probe/README.md`).
//!
//! On a trigger it writes the dwell it already holds plus thirty more seconds as
//! a 16-bit mono WAV, then moves to the next frequency. **Nothing is written
//! when there is no signal**, so a night's watching costs a few megabytes rather
//! than tens of gigabytes of noise.
//!
//! The audio is the real part of the baseband, downsampled to about 12 kHz,
//! which is what a USB demodulator produces: with the dial on the carrier, the
//! 4285 subcarrier sits at 1800 Hz exactly where the receiver wants it.
//!
//! **Ask for 62500, not 48000.** An RSP1 refuses 48 kHz and silently keeps its
//! own rate -- 2 Msps by the log, which overflows the read buffer and fills the
//! memory with samples nobody wanted. The tool prints the rate it actually got
//! and the WAV is written at a rate derived from that, so a refusal is visible
//! rather than silent, but 62.5 kHz is a rate the radio takes. `tools/s4285-probe/s4285_verdict.py`
//! reads the files back.
use num_complex::Complex32;
use soapysdr::{Device, Direction};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const FRAME_S: f64 = 256.0 / 2400.0; // 106.67 ms
const POST_S: f64 = 30.0;
const WINDOW_S: f64 = 4.0;
const ACF_MIN: f32 = 0.25;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("usage: s4285watch <hz[,hz...]> <rate> <outdir> [gain] [threshold] [dwell]");
        eprintln!("       s4285watch --selftest");
        std::process::exit(2);
    }
    if a[1] == "--selftest" {
        std::process::exit(selftest());
    }

    let freqs: Vec<f64> = a[1].split(',').map(|f| f.trim().parse().expect("hz")).collect();
    let rate: f64 = a[2].parse().expect("rate");
    let outdir = std::path::PathBuf::from(a.get(3).cloned().unwrap_or_else(|| ".".into()));
    let gain: f64 = a.get(4).and_then(|g| g.parse().ok()).unwrap_or(40.0);
    let threshold: f32 = a.get(5).and_then(|g| g.parse().ok()).unwrap_or(3.0);

    std::fs::create_dir_all(&outdir).expect("make the output directory");

    let dev = Device::new("driver=sdrplay").expect("open the radio");
    dev.set_sample_rate(Direction::Rx, 0, rate).expect("sample rate");
    dev.set_frequency(Direction::Rx, 0, freqs[0], ()).expect("frequency");
    if dev.has_gain_mode(Direction::Rx, 0).unwrap_or(false) {
        let _ = dev.set_gain_mode(Direction::Rx, 0, false);
    }
    dev.set_gain(Direction::Rx, 0, gain).expect("gain");

    // What the radio actually did, not what was asked for -- a refused rate
    // silently keeps the old one and every frequency in the file would be wrong.
    let rate = dev.sample_rate(Direction::Rx, 0).expect("read the rate back");
    let got_hz = dev.frequency(Direction::Rx, 0).expect("read the frequency back");

    let mut stream = dev.rx_stream::<Complex32>(&[0]).expect("stream");
    stream.activate(None).expect("activate");
    println!(
        "watching {:.3} kHz at {rate:.0} S/s, gain {gain:.0}, threshold {threshold:.1}x, \
         writing to {}",
        got_hz / 1e3,
        outdir.display()
    );
    println!("frame {:.2} ms; a trigger writes the dwell plus {:.0}s", FRAME_S * 1e3, POST_S);

    let dwell: f64 = a.get(6).and_then(|g| g.parse().ok()).unwrap_or(6.0);
    let window_s = WINDOW_S.min(dwell);
    let window_len = (window_s * rate) as usize;
    let ring_cap = (dwell * rate) as usize;
    let post_len = (POST_S * rate) as usize;

    println!(
        "{} frequenc{}, {:.1}s each, frame {:.2} ms, threshold {threshold:.1}x",
        freqs.len(),
        if freqs.len() == 1 { "y" } else { "ies" },
        dwell,
        FRAME_S * 1e3
    );
    if freqs.len() > 1 {
        println!("a sweep takes about {:.0}s; a trigger records {:.0}s at that frequency",
                 dwell * freqs.len() as f64, dwell + POST_S);
    }

    let mut chunk = vec![Complex32::new(0.0, 0.0); 32768];
    let mut drops = 0u64;
    let mut fi = 0usize;
    let mut ring: Vec<Complex32> = Vec::with_capacity(ring_cap);
    let mut capture: Option<Vec<Complex32>> = None;
    let mut capture_left = 0usize;
    let mut window: Vec<Complex32> = Vec::with_capacity(window_len);
    let mut trigger: (f32, f32) = (0.0, 0.0);

    loop {
        if capture.is_none() && ring.is_empty() {
            // New frequency: retune and start a clean window. Nothing from the
            // old one may leak into the measurement of the new one.
            let f = freqs[fi];
            if let Err(e) = dev.set_frequency(Direction::Rx, 0, f, ()) {
                eprintln!("{} retune to {f:.0} failed: {e}", utc_stamp());
            }
        }
        let mut bufs: Vec<&mut [Complex32]> = vec![&mut chunk];
        let n = match stream.read(&mut bufs, 5_000_000) {
            Ok(n) => n,
            Err(e) => {
                // An overflow is the radio's buffer not being collected in time.
                // Dropping a chunk costs a second of the watch and nothing else,
                // so it must never end the night.
                drops += 1;
                if drops == 1 || drops % 100 == 0 {
                    eprintln!("{} read error: {e} ({drops} dropped so far)", utc_stamp());
                }
                continue;
            }
        };
        if n == 0 {
            continue;
        }

        if let Some(c) = capture.as_mut() {
            c.extend_from_slice(&chunk[..n]);
            capture_left = capture_left.saturating_sub(n);
            if capture_left == 0 {
                let (acf, null) = trigger;
                let name = outdir.join(format!(
                    "{:.3}kHz-{}.wav",
                    freqs[fi] / 1e3,
                    utc_stamp()
                ));
                match write_wav(&name, c, rate) {
                    Ok((secs, out_rate)) => println!(
                        "{} saved {} ({secs:.1}s at {out_rate} Hz, triggered at acf {acf:.3} null {null:.3})",
                        utc_stamp(),
                        name.display()
                    ),
                    Err(e) => eprintln!("{} could not write {}: {e}", utc_stamp(), name.display()),
                }
                capture = None;
                fi = (fi + 1) % freqs.len();
                ring.clear();
            }
            continue;
        }

        ring.extend_from_slice(&chunk[..n]);
        if ring.len() < ring_cap {
            continue;
        }
        window.clear();
        window.extend_from_slice(&ring[ring.len() - window_len..]);

        let (acf, null) = score_window(&window, rate);
        let ratio = acf / null.max(1e-6);
        let pass = acf > ACF_MIN && ratio > threshold;
        println!(
            "{} {:9.3} kHz  acf {acf:5.3}  null {null:5.3}  {ratio:5.1}x  {}",
            utc_stamp(),
            freqs[fi] / 1e3,
            if pass { "MATCH" } else { "" }
        );

        if pass {
            println!(
                "{} frame present -- recording {:.0}s",
                utc_stamp(),
                (ring.len() + post_len) as f64 / rate
            );
            trigger = (acf, null);
            capture = Some(std::mem::take(&mut ring));
            capture_left = post_len;
        } else {
            fi = (fi + 1) % freqs.len();
            ring.clear();
        }
    }
}

/// |ACF| at the frame period and the median of nearby lags, which is the null.
fn score_window(buf: &[Complex32], rate: f64) -> (f32, f32) {
    let frame = (FRAME_S * rate).round() as usize;
    let acf = acf_at(buf, frame);
    let mut nulls = Vec::new();
    for ms in 60..=160 {
        if (ms as f64 - FRAME_S * 1e3).abs() < 4.0 {
            continue;
        }
        let lag = (ms as f64 * 1e-3 * rate).round() as usize;
        if lag < 1 || lag >= buf.len() {
            continue;
        }
        nulls.push(acf_at(buf, lag));
    }
    nulls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let null = nulls.get(nulls.len() / 2).copied().unwrap_or(0.0);
    (acf, null)
}

fn acf_at(buf: &[Complex32], lag: usize) -> f32 {
    if lag == 0 || lag >= buf.len() {
        return 0.0;
    }
    let n = buf.len() - lag;
    let mut num = Complex32::new(0.0, 0.0);
    let mut da = 0.0f32;
    let mut db = 0.0f32;
    for i in 0..n {
        let a = buf[i];
        let b = buf[i + lag];
        num += a * b.conj();
        da += a.norm_sqr();
        db += b.norm_sqr();
    }
    num.norm() / (da * db).sqrt().max(1e-20)
}

/// 16-bit mono PCM at about 12 kHz, properly filtered.
///
/// The first version averaged groups of samples, which is a box filter: its
/// stopband is only about -13 dB, so anything strong in the rest of the 62.5 kHz
/// band folded straight into the audio. A capture of 6348 kHz came back as a flat
/// mess across the whole band because of it -- the signal is 3 kHz wide and the
/// file was 6 kHz of aliased noise. A windowed-sinc low-pass first, then take
/// every nth sample.
fn write_wav(path: &std::path::Path, iq: &[Complex32], rate: f64) -> std::io::Result<(f64, u32)> {
    let factor = ((rate / 12_000.0).round() as usize).max(1);
    let out_rate = (rate / factor as f64).round() as u32;
    let filtered = lowpass(iq, out_rate as f64);
    let n = filtered.len() / factor;
    let data_len = (n * 2) as u32;
    let mut f = std::fs::File::create(path)?;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&out_rate.to_le_bytes())?;
    f.write_all(&(out_rate * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    let mut bytes: Vec<u8> = Vec::with_capacity(n * 2);
    for group in filtered.chunks_exact(factor) {
        let v = group[0].re.clamp(-1.0, 1.0);
        bytes.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
    }
    f.write_all(&bytes)?;
    Ok((n as f64 / out_rate as f64, out_rate))
}

/// A 127-tap windowed-sinc low-pass with its corner just under the new Nyquist.
fn lowpass(iq: &[Complex32], out_rate: f64) -> Vec<Complex32> {
    let taps = 127usize;
    let half = (taps / 2) as isize;
    let fc = 0.45; // fraction of the output Nyquist
    let mut h = Vec::with_capacity(taps);
    let mut sum = 0.0f32;
    for i in 0..taps {
        let x = i as f32 - half as f32;
        let sinc = if x == 0.0 {
            fc
        } else {
            (std::f32::consts::PI * fc * x).sin() / (std::f32::consts::PI * x)
        };
        let w = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (taps - 1) as f32).cos();
        h.push(sinc * w);
        sum += sinc * w;
    }
    for v in h.iter_mut() {
        *v /= sum;
    }
    let mut out = Vec::with_capacity(iq.len());
    for (i, s) in iq.iter().enumerate() {
        let mut acc = Complex32::new(0.0, 0.0);
        for (k, w) in h.iter().enumerate() {
            let j = i as isize + k as isize - half;
            if j >= 0 && (j as usize) < iq.len() {
                acc += iq[j as usize] * *w;
            }
        }
        let _ = s;
        out.push(acc);
    }
    out
}

fn utc_stamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-from-civil, inverted; no chrono for one timestamp.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Does the detector fire on a framed signal and stay quiet on noise? Runs with
/// no radio, so it can be checked on any machine.
fn selftest() -> i32 {
    let rate = 48_000.0;
    let n = (8.0 * rate) as usize;
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        Complex32::new(
            (seed & 0xffff) as f32 / 32768.0 - 1.0,
            ((seed >> 16) & 0xffff) as f32 / 32768.0 - 1.0,
        )
    };
    let noise: Vec<Complex32> = (0..n).map(|_| rnd()).collect();
    let (acf, null) = score_window(&noise, rate);
    let noise_ratio = acf / null.max(1e-6);

    // A block repeated at the frame period: the statistic's subject, without
    // pretending to be a transmitter.
    let frame = (FRAME_S * rate).round() as usize;
    let block: Vec<Complex32> = (0..frame).map(|_| rnd()).collect();
    let framed: Vec<Complex32> = (0..n)
        .map(|i| block[i % frame] + rnd() * 0.5)
        .collect();
    let (facf, fnull) = score_window(&framed, rate);
    let framed_ratio = facf / fnull.max(1e-6);

    // Write one, so the WAV path is exercised too -- a writer that only runs on
    // a trigger would otherwise first be tested on a real capture, after a night
    // of watching.
    let path = std::env::temp_dir().join("s4285-selftest.wav");
    match write_wav(&path, &framed, rate) {
        Ok((secs, out)) => println!(
            "wrote {} ({secs:.1}s at {out} Hz) -- read it back with s4285_verdict.py",
            path.display()
        ),
        Err(e) => {
            eprintln!("could not write the selftest wav: {e}");
            return 1;
        }
    }

    println!("noise   acf {acf:.3}  null {null:.3}  {noise_ratio:.1}x");
    println!("framed  acf {facf:.3}  null {fnull:.3}  {framed_ratio:.1}x");
    let ok = noise_ratio < 2.0 && facf > ACF_MIN && framed_ratio > 3.0;
    println!("{}", if ok { "selftest ok" } else { "selftest FAILED" });
    if ok { 0 } else { 1 }
}
