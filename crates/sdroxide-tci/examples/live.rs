//! Live check against a TCI server: test the connection, then stream ~2 s of IQ
//! and report sample count + RMS. Usage: cargo run -p sdroxide-tci --example live

use std::time::{Duration, Instant};

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:50001".to_string());

    println!("== test_connection ==");
    match sdroxide_tci::test_connection(&addr, Duration::from_secs(3)) {
        Ok(s) => println!("  OK: {s}"),
        Err(e) => {
            println!("  ERR: {e}");
            return;
        }
    }
    // A deliberately-wrong port should fail fast.
    match sdroxide_tci::test_connection("127.0.0.1:59999", Duration::from_secs(2)) {
        Ok(s) => println!("  (bad port unexpectedly OK: {s})"),
        Err(e) => println!("  bad-port correctly errors: {e}"),
    }

    println!("== IQ stream (192 kHz, centered 14.100 MHz) ==");
    let mut h = match sdroxide_tci::TciHandle::connect(&addr, 192_000.0) {
        Ok(h) => h,
        Err(e) => {
            println!("  connect failed: {e}");
            return;
        }
    };
    println!("  device: {}", h.device);
    h.set_center(14_100_000.0);

    let mut buf = vec![0f32; 32_768];
    let mut total = 0usize;
    let mut sumsq = 0f64;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let n = h.rx_read(&mut buf);
        if n == 0 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        total += n;
        for &v in &buf[..n] {
            sumsq += (v as f64) * (v as f64);
        }
    }
    let rms = if total > 0 { (sumsq / total as f64).sqrt() } else { 0.0 };
    println!("  received {} IQ floats ({} pairs) in 2 s, RMS={rms:.5}", total, total / 2);
    for u in h.poll_updates() {
        println!("  update: {u:?}");
    }
}
