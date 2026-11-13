//! The Mercury engine linked in: it starts, answers its TNC port, keys for a
//! CQ frame and puts out audio in its passband. One engine per process, so
//! this file holds one test.

#![cfg(not(target_env = "msvc"))]

use std::time::{Duration, Instant};

use sdroxide_mercury::{Config, MercuryModem};
use sdroxide_tnc::TncModem;

/// Run the modem in real time on silence for `ms`, collecting what it sends.
fn run(m: &mut MercuryModem, ms: u64, tx_all: &mut Vec<f32>, lines: &mut Vec<String>) {
    let block = 160; // 20 ms
    let silence = vec![0f32; block];
    let mut tx = vec![0f32; block];
    let start = Instant::now();
    let mut fed = 0u64;
    while start.elapsed() < Duration::from_millis(ms) {
        m.process(&silence, &mut tx);
        tx_all.extend_from_slice(&tx);
        fed += block as u64;
        let (l, _) = m.take_output();
        lines.extend(l);
        // Hold to the audio clock, as a sound card would.
        let due = Duration::from_micros(fed * 125);
        if let Some(wait) = due.checked_sub(start.elapsed()) {
            std::thread::sleep(wait);
        }
    }
}

/// Power at `hz` against the rest, by a plain DFT over the burst.
fn band_share(x: &[f32], lo: f64, hi: f64) -> f64 {
    let n = x.len().min(8000 * 2);
    let x = &x[..n];
    let (mut inband, mut total) = (0.0, 0.0);
    for k in (1..n / 2).step_by(4) {
        let f = k as f64 * 8000.0 / n as f64;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, &v) in x.iter().enumerate() {
            let ph = -2.0 * std::f64::consts::PI * k as f64 * i as f64 / n as f64;
            re += f64::from(v) * ph.cos();
            im += f64::from(v) * ph.sin();
        }
        let p = re * re + im * im;
        total += p;
        if (lo..=hi).contains(&f) {
            inband += p;
        }
    }
    inband / total
}

#[test]
fn mercury_starts_answers_and_transmits_in_band() {
    let dir = std::env::temp_dir().join(format!("sdroxide-mercury-test-{}", std::process::id()));
    let engine =
        sdroxide_mercury::start(&Config { private_port: 18_390, config_dir: dir, tx_delay_ms: 50 })
            .unwrap();
    // A second start hands back the same engine.
    let again = sdroxide_mercury::start(&Config {
        private_port: 1,
        config_dir: ".".into(),
        tx_delay_ms: 0,
    })
    .unwrap();
    assert!(std::ptr::eq(engine, again));
    assert_eq!(engine.port(), 18_390);

    let mut m = MercuryModem::new(engine).unwrap();
    let (mut tx, mut lines) = (Vec::new(), Vec::new());
    m.command("MYCALL OE1AAA");
    m.command("VERSION");
    m.command("BW2300");
    run(&mut m, 500, &mut tx, &mut lines);
    // Mercury words its VERSION reply the way existing clients look for it.
    assert!(lines.iter().any(|l| l.contains("version")), "{lines:?}");
    assert!(lines.iter().any(|l| l == "REGISTERED OE1AAA"), "{lines:?}");
    assert!(lines.iter().any(|l| l == "OK"), "{lines:?}");
    assert!(tx.iter().all(|&v| v == 0.0), "nothing goes out unasked");

    // A CQ frame keys the transmitter and puts a burst on the air.
    tx.clear();
    m.command("CQFRAME OE1AAA 2300");
    let mut keyed_seen = false;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        run(&mut m, 100, &mut tx, &mut lines);
        keyed_seen |= m.keyed();
        if keyed_seen && !m.keyed() {
            break;
        }
    }
    assert!(keyed_seen, "never keyed; replies {lines:?}");
    let sound: Vec<f32> = tx.iter().copied().skip_while(|v| *v == 0.0).collect();
    assert!(sound.len() > 4000, "a burst of at least half a second, got {}", sound.len());
    let peak = sound.iter().fold(0f32, |a, v| a.max(v.abs()));
    eprintln!("CQ burst: {} samples, peak {peak:.3}", sound.len());
    assert!(peak > 0.05 && peak <= 1.0, "peak {peak}");
    let share = band_share(&sound, 300.0, 2700.0);
    assert!(share > 0.95, "{:.1}% of the power in 300-2700 Hz", share * 100.0);

    // Mercury's TCP ports are on the loopback interface only.
    let lan = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("192.0.2.1:9").map(|_| s))
        .and_then(|s| s.local_addr())
        .map(|a| a.ip());
    if let Ok(ip) = lan
        && !ip.is_loopback()
    {
        let r =
            std::net::TcpStream::connect_timeout(&(ip, 18_390).into(), Duration::from_millis(500));
        assert!(r.is_err(), "Mercury answered on {ip}");
    }
}
