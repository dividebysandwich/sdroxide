//! What one panadapter window costs the DSP thread — the measurement behind
//! issue #195, where zooming into the waterfall on a 2 Msps HackRF started an
//! RX overflow that never let go.
//!
//! The source is free-running rather than paced, so the engine consumes as fast
//! as it can and the achieved rate *is* the throughput of the whole receive
//! path at that setting. Anything at or below the front end's real rate is a
//! receiver that cannot keep up, which the driver reports as dropped samples.
//!
//! `cargo run --release --example panadapter_cost -- [device Msps] [seconds]`

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig};

const CENTER: f64 = 89_900_000.0;

/// Noise at the device rate, as fast as it is asked for.
struct FreeRunning {
    rate: f64,
    delivered: Arc<AtomicU64>,
    phase: f32,
}

impl IqSource for FreeRunning {
    fn sample_rate(&self) -> f64 {
        self.rate
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(16_384);
        for s in buf[..n].iter_mut() {
            self.phase = (self.phase + 0.017_3) % std::f32::consts::TAU;
            *s = Complex32::new(self.phase.cos() * 0.2, self.phase.sin() * 0.2);
        }
        self.delivered.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
    fn describe(&self) -> String {
        "free-running noise".into()
    }
}

fn caps(rate: f64) -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench".into(),
        rx_channels: 1,
        sample_rates: vec![rate],
        freq_ranges_rx: vec![(1_000_000.0, 2_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The config a client sends for a window `span_hz` wide on the centre. `None`
/// is the fully zoomed-out view. `fft` is what the client asks the device-wide
/// analyser for, which is what grows with zoom today.
fn cfg(span_hz: Option<f64>, fft: u32) -> Command {
    cfg_rows(span_hz, fft, 100)
}

fn cfg_rows(span_hz: Option<f64>, fft: u32, rows: u16) -> Command {
    Command::SetSpectrumCfg(SpectrumConfig {
        fft_size: fft,
        display_bins: 2048,
        rows_per_sec: rows,
        db_floor: -140.0,
        db_ceil: 0.0,
        viewport: span_hz.map(|s| (CENTER - s / 2.0, CENTER + s / 2.0)),
        fps: 30,
        avg_tc: 0.0,
        ..Default::default()
    })
}

fn measure(rate: f64, secs: f64, label: &str, command: Command) {
    let delivered = Arc::new(AtomicU64::new(0));
    let src = FreeRunning { rate, delivered: Arc::clone(&delivered), phase: 0.0 };
    let mut h = start_engine(Box::new(src), caps(rate), EngineConfig::default());
    let thread = h.thread.take();
    h.cmd_tx.send(command).unwrap();
    // Let the lanes settle on the new config before the clock starts.
    std::thread::sleep(Duration::from_secs(1));
    let (at, from) = (Instant::now(), delivered.load(Ordering::Relaxed));
    std::thread::sleep(Duration::from_secs_f64(secs));
    let got = delivered.load(Ordering::Relaxed) - from;
    let msps = got as f64 / at.elapsed().as_secs_f64() / 1e6;
    println!(
        "{label:<28} {msps:8.2} Msps  ({:.1}× a {:.1} Msps front end)",
        msps / (rate / 1e6),
        rate / 1e6
    );
    drop(h);
    let _ = thread.map(|t| t.join());
}

fn main() {
    if let Ok(only) = std::env::var("PANA_ONLY") {
        // One case, for a profiler: the sweep's other settings would be in the
        // same profile and there would be no telling them apart.
        let mut it = only.split(',');
        let rate = it.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(2.4) * 1e6;
        let secs = it.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(10.0);
        let fft = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(4096);
        let rows = it.next().and_then(|s| s.parse::<u16>().ok()).unwrap_or(224);
        let zoom = it.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();
        let span = (zoom > 0.0).then(|| rate / zoom);
        measure(rate, secs, &format!("fft {fft}, {rows} rows/s"), cfg_rows(span, fft, rows));
        return;
    }
    let mut args = std::env::args().skip(1);
    let rate = args.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(2.0) * 1e6;
    let secs = args.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(5.0);
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
    println!("free-running throughput of the whole receive path, {secs:.0} s a setting\n");
    measure(rate, secs, "zoomed out, 4096", cfg(None, 4096));
    measure(rate, secs, "8× zoom, 4096", cfg(Some(rate / 8.0), 4096));
    measure(rate, secs, "8× zoom, 32768", cfg(Some(rate / 8.0), 32_768));
    measure(rate, secs, "64× zoom, 32768", cfg(Some(rate / 64.0), 32_768));
    measure(rate, secs, "64× zoom, 4096", cfg(Some(rate / 64.0), 4096));
    measure(rate, secs, "2× zoom, 8192", cfg(Some(rate / 2.0), 8192));
    println!();
    for rows in [1u16, 28, 56, 112, 224] {
        measure(rate, secs, &format!("zoomed out 4096, {rows} rows/s"), cfg_rows(None, 4096, rows));
    }
    for rows in [1u16, 28, 100] {
        measure(
            rate,
            secs,
            &format!("8x zoom 32768, {rows} rows/s"),
            cfg_rows(Some(rate / 8.0), 32_768, rows),
        );
    }
}
