//! REASSIGN through the whole engine: a carrier in the waterfall rows is
//! drawn narrower with the reassigned rows than with WELCH's mean rows,
//! wherever a column is finer than the window's main lobe.
//!
//! Also prints the widths for a wide and a zoomed view (run with
//! `--nocapture`; `RA_DUMP=1` prints a row around the carrier, `RA_RATE` and
//! `RA_CASE=<span Hz>,<fft>` try another device and view), because the gain depends on how many transform bins one
//! column holds: where a column holds the whole main lobe already, the line
//! cannot get narrower on screen.
// Why: HDZ 2026-11-07: "ich seh bei reassign keinen Unterschied".

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig};

/// The front end's rate; `RA_RATE` overrides it to try another device's.
fn rate() -> f64 {
    std::env::var("RA_RATE").ok().and_then(|v| v.parse().ok()).unwrap_or(192_000.0)
}
const CENTER: f64 = 7_100_000.0;
/// The carrier, off any bin centre and well clear of the DC patch at the
/// middle of the device-wide lane.
const TONE_OFF: f64 = 20_123.4;

struct Tone {
    center_hz: f64,
    n: u64,
    seed: u32,
}

impl IqSource for Tone {
    fn sample_rate(&self) -> f64 {
        rate()
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min((rate() * 0.005) as usize);
        let w = std::f64::consts::TAU * (CENTER + TONE_OFF - self.center_hz) / rate();
        for s in buf[..n].iter_mut() {
            self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let r = (self.seed >> 8) as f32 / (1 << 24) as f32 - 0.5;
            let ph = w * self.n as f64;
            *s = Complex32::new(
                0.1 * ph.cos() as f32 + 0.002 * r,
                0.1 * ph.sin() as f32 - 0.002 * r,
            );
            self.n += 1;
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "tone generator".into()
    }
}

fn engine() -> EngineHandles {
    start_engine(
        Box::new(Tone { center_hz: CENTER, n: 0, seed: 99 }),
        DeviceCaps {
            driver: "test".into(),
            label: "test".into(),
            rx_channels: 1,
            sample_rates: vec![rate()],
            freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
            ..DeviceCaps::default()
        },
        EngineConfig { remember_session: false, ..Default::default() },
    )
}

/// Mean width, in columns, of the carrier in the waterfall rows: columns
/// within 10 dB of the row's top.
fn line_width(span: f64, fft: u32, smooth: bool, reassign: bool) -> f64 {
    let mut h = engine();
    let lo = CENTER + TONE_OFF - span / 2.0;
    h.cmd_tx
        .send(Command::SetSpectrumCfg(SpectrumConfig {
            fft_size: fft,
            display_bins: 1024,
            rows_per_sec: 30,
            db_floor: -140.0,
            db_ceil: 0.0,
            viewport: Some((lo, lo + span)),
            smooth,
            reassign,
            reassign_floor_db: 6.0,
            reassign_dim_db: 0.0,
            fps: 30,
            avg_tc: 0.0,
        }))
        .unwrap();
    let step = 140.0 / 255.0;
    let (mut sum, mut rows) = (0.0, 0usize);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(4) {
        if h.spectrum_out.update() {
            let f = h.spectrum_out.output_buffer();
            let cols = f.bins.len();
            // Settled frames only, over the asked view.
            if cols > 0 && start.elapsed() > Duration::from_millis(1500) {
                if rows == 20 && std::env::var("RA_DUMP").is_ok() && !f.rows.is_empty() {
                    let row = &f.rows[..cols];
                    let pk = (0..cols).max_by_key(|&i| row[i]).unwrap();
                    eprintln!(
                        "span {span} fft {fft} smooth {smooth} reassign {reassign}: cols {cols} frame span {:.1} peak {pk}: {:?}",
                        f.span_hz,
                        &row[pk.saturating_sub(12)..(pk + 13).min(cols)]
                    );
                }
                for row in f.rows.chunks_exact(cols) {
                    let top = *row.iter().max().unwrap() as f64;
                    sum += row.iter().filter(|&&b| (top - b as f64) * step <= 10.0).count() as f64;
                    rows += 1;
                }
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }
    drop(h.cmd_tx);
    if rows == 0 { f64::NAN } else { sum / rows as f64 }
}

#[test]
fn reassigned_rows_draw_a_carrier_narrower_than_welch() {
    // (span, transform): wide and zoomed, with a long and a short window.
    let cases: Vec<(f64, u32)> = match std::env::var("RA_CASE") {
        Ok(c) => {
            let (a, b) = c.split_once(',').unwrap();
            vec![(a.parse().unwrap(), b.parse().unwrap())]
        }
        Err(_) => vec![(192_000.0, 4096), (192_000.0, 512), (2_000.0, 4096), (2_000.0, 512)],
    };
    let mut short_wide = (0.0, 0.0);
    for (span, fft) in cases {
        let (w, r) = (line_width(span, fft, true, false), line_width(span, fft, false, true));
        eprintln!("span {span:>7.0} Hz, FFT {fft:>4}: WELCH {w:5.1} columns, REASSIGN {r:5.1}");
        if span > 100_000.0 && fft == 512 {
            short_wide = (w, r);
        }
    }
    // A short window across the whole band: WELCH draws the main lobe, about
    // seventeen columns wide (measured 07.11.); REASSIGN draws the carrier on
    // its own frequency, two.
    // (Only where that case ran: `RA_CASE` picks a single other one.)
    let (w, r) = short_wide;
    if w == 0.0 {
        return;
    }
    assert!(r < w * 0.3, "FFT 512, wide: REASSIGN {r:.1} vs WELCH {w:.1} columns");
}
