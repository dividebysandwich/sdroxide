//! The waterfall must never go blank because a lane was rebuilt.
//!
//! A pan or a zoom replaces the panadapter's analyser — the zoom lane when the
//! window moves, the device-wide one when the client resizes its FFT — and a
//! fresh analyser holds no spectrum at all until its first transform lands.
//! Drawn from anyway, it answers the display floor for every column, so the
//! operator gets a black band across the whole width of the waterfall lasting
//! however long that analyser takes to fill: a hundred and thirty milliseconds
//! for a zoom lane at 125 kHz through a 16384-point window, and much longer on
//! a narrow front end.

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, SpectrumConfig, SpectrumFrame};

const RATE: f64 = 2_000_000.0;
const CENTER: f64 = 14_200_000.0;
/// The window the operator has open, well inside the stream.
const VIEW_HZ: f64 = 100_000.0;

/// Broadband noise at a level nothing in the display range can mistake for the
/// floor: every column of every honest frame is far above zero, so a column
/// that reads zero can only be an analyser with nothing in it.
struct Noise {
    center_hz: f64,
    seed: u32,
}

impl Noise {
    fn next(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1 << 23) as f32 - 1.0
    }
}

impl IqSource for Noise {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Paced so the engine loop runs at a realistic rate without the test
        // waiting real time for 2 Msps.
        std::thread::sleep(Duration::from_millis(4));
        let n = buf.len().min(8192);
        for s in buf[..n].iter_mut() {
            *s = Complex32::new(self.next() * 0.3, self.next() * 0.3);
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "noise generator".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn engine() -> EngineHandles {
    start_engine(
        Box::new(Noise { center_hz: CENTER, seed: 12345 }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    )
}

fn cfg(center: f64, span: f64, fft: u32) -> Command {
    Command::SetSpectrumCfg(SpectrumConfig {
        fft_size: fft,
        display_bins: 2048,
        rows_per_sec: 60,
        db_floor: -140.0,
        db_ceil: 0.0,
        viewport: Some((center - span / 2.0, center + span / 2.0)),
        smooth: false,
        reassign: false,
        reassign_floor_db: 6.0,
        reassign_dim_db: 0.0,
        fps: 60,
        avg_tc: 0.0,
    })
}

/// The worst column of the worst waterfall row (and of the trace) in a frame.
/// Zero means a whole picture at the display floor — the black band.
fn floor_run(f: &SpectrumFrame) -> Option<String> {
    let cols = f.bins.len();
    if cols == 0 {
        return None;
    }
    if f.bins.iter().all(|&b| b == 0) {
        return Some(format!("trace at {:.0} Hz span {:.0}", f.center_hz, f.span_hz));
    }
    for (i, row) in f.rows.chunks_exact(cols).enumerate() {
        if row.iter().all(|&b| b == 0) {
            return Some(format!("row {i} of {}", f.rows.len() / cols));
        }
    }
    None
}

/// Watch for `secs`, reporting every blank picture seen.
fn watch(h: &mut EngineHandles, secs: f64) -> Vec<String> {
    let mut bad = Vec::new();
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < deadline {
        if h.spectrum_out.update() {
            if let Some(why) = floor_run(h.spectrum_out.output_buffer()) {
                bad.push(why);
            }
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }
    bad
}

/// Wait until the picture is honest, so the settling every fresh engine goes
/// through cannot be mistaken for the fault under test.
fn settle(h: &mut EngineHandles, secs: f64) {
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < deadline {
        if h.spectrum_out.update() && floor_run(h.spectrum_out.output_buffer()).is_none() {
            std::thread::sleep(Duration::from_millis(50));
            return;
        }
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the picture never settled");
}

#[test]
fn panning_a_zoomed_window_never_blanks_the_waterfall() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg(CENTER, VIEW_HZ, 4096)).unwrap();
    settle(&mut h, 5.0);

    // A drag: the window walks a tenth of its width every frame for half a
    // second, exactly as a client sends it.
    let mut bad = Vec::new();
    for step in 1..=30 {
        h.cmd_tx.send(cfg(CENTER + step as f64 * VIEW_HZ / 10.0, VIEW_HZ, 4096)).unwrap();
        bad.extend(watch(&mut h, 0.016));
    }
    bad.extend(watch(&mut h, 0.5));
    assert!(bad.is_empty(), "the waterfall went blank while panning: {bad:?}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn zooming_never_blanks_the_waterfall() {
    let mut h = engine();
    let thread = h.thread.take();

    h.cmd_tx.send(cfg(CENTER, VIEW_HZ, 4096)).unwrap();
    settle(&mut h, 5.0);

    // A zoom in and back out, the client growing its FFT with the zoom as the
    // real one does.
    let mut bad = Vec::new();
    for (span, fft) in [(50_000.0, 8192), (25_000.0, 16384), (50_000.0, 8192), (VIEW_HZ, 4096)] {
        h.cmd_tx.send(cfg(CENTER, span, fft)).unwrap();
        bad.extend(watch(&mut h, 0.6));
    }
    assert!(bad.is_empty(), "the waterfall went blank while zooming: {bad:?}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
