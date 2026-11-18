//! The main panadapter must go back to the I/Q once the dial has moved.
//!
//! A front end that publishes a band-wide overview alongside its I/Q — an
//! RX-888's whole 64.8 MHz of HF, a KiwiSDR's 0–30 MHz — draws the main
//! panadapter from that overview only while the viewport reaches outside what
//! the I/Q covers. The test is containment, so it needs the centre the samples
//! in hand were taken at.
//!
//! On a front end that declares a stream delay the engine tracks that down a
//! trail of retunes. On every other one there is no trail to track: the samples
//! in hand are at the commanded centre. Reading the tracked field regardless
//! left it on the centre the engine opened at, so every later tune looked like
//! a viewport panned clean off the passband and the whole of HF was drawn from
//! the overview lane at some 16 kHz a bin — except in the one window the engine
//! had started in.

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioEvent, SpectrumConfig, SpectrumFrame, Vfo};

/// An RX-888 at its 129.6 MHz clock, 1/32 of it downconverted.
const RATE: f64 = 4_050_000.0;
/// The whole real half-spectrum the overview lane covers, and its bins — a
/// 8192-point real transform, so 15.8 kHz apiece.
const WIDE_SPAN: f64 = 64_800_000.0;
const WIDE_BINS: usize = 4097;

/// Which lane drew a frame cannot be read off its span: the overview lane
/// renders the client's viewport too. So each lane carries its own marker at a
/// frequency the other one has nothing at, and the frame's peak names the lane
/// that built it.
///
/// The I/Q's marker is a tone at a fixed offset from the stream centre, so it
/// follows the dial and stays inside any window fitted to the passband.
const IQ_TONE_OFFSET: f64 = 500_000.0;
/// The overview's marker is a fixed RF: inside the wide zoom-out, outside the
/// narrow windows the I/Q is supposed to own.
const WIDE_MARKER_HZ: f64 = 20_400_000.0;

/// Where the engine opens, and where it is then tuned. Two megahertz apart, so
/// the second sits outside the passband the first one had.
const START: f64 = 16_200_000.0;
const MOVED: f64 = 21_000_000.0;

struct Rx888ish {
    center_hz: f64,
    seed: u32,
    phase: f32,
}

impl Rx888ish {
    fn next(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1 << 23) as f32 - 1.0
    }
}

impl IqSource for Rx888ish {
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
        std::thread::sleep(Duration::from_millis(4));
        let n = buf.len().min(8192);
        let step = std::f32::consts::TAU * (IQ_TONE_OFFSET / RATE) as f32;
        for s in buf[..n].iter_mut() {
            let tone = Complex32::new(self.phase.cos(), self.phase.sin()) * 0.5;
            self.phase = (self.phase + step) % std::f32::consts::TAU;
            *s = tone + Complex32::new(self.next() * 0.01, self.next() * 0.01);
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock RX-888".into()
    }
    /// The full-band overview, always the same window: DC to Nyquist, flat
    /// apart from its own marker.
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        out.clear();
        out.resize(WIDE_BINS, -110.0);
        let hot = (WIDE_MARKER_HZ / WIDE_SPAN * WIDE_BINS as f64).round() as usize;
        out[hot] = -20.0;
        Some((WIDE_SPAN / 2.0, WIDE_SPAN))
    }
    fn wide_span_hz(&self) -> f64 {
        WIDE_SPAN
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "rx888".into(),
        label: "mock RX-888".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, WIDE_SPAN / 2.0)],
        ..DeviceCaps::default()
    }
}

/// Tune to `dial`, ask for a window `span` wide around it, and return the last
/// main-lane frame published after that.
fn frame_at(dial: f64, span: f64) -> SpectrumFrame {
    let mut h = start_engine(
        Box::new(Rx888ish { center_hz: START, seed: 12345, phase: 0.0 }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(250));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: dial }).unwrap();
    h.cmd_tx
        .send(Command::SetSpectrumCfg(SpectrumConfig {
            viewport: Some((dial - span / 2.0, dial + span / 2.0)),
            smooth: false,
            reassign: false,
            reassign_floor_db: 6.0,
            reassign_dim_db: 0.0,
            ..SpectrumConfig::default()
        }))
        .unwrap();

    let mut frame = None;
    let deadline = Instant::now() + Duration::from_millis(900);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            let _: RadioEvent = ev;
        }
        if h.spectrum_out.update() {
            frame = Some(h.spectrum_out.output_buffer().clone());
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    frame.expect("the engine published no spectrum")
}

/// The RF the frame's strongest column sits on.
fn peak_hz(f: &SpectrumFrame) -> f64 {
    let n = f.bins.len();
    assert!(n > 0, "an empty frame");
    let i = f.bins.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i).unwrap();
    f.center_hz - f.span_hz / 2.0 + f.span_hz * (i as f64 + 0.5) / n as f64
}

/// Whether the frame is drawn on `want`, within a couple of columns of it.
#[track_caller]
fn drew_on(f: &SpectrumFrame, want: f64, lane: &str) {
    let tol = f.span_hz / f.bins.len() as f64 * 3.0;
    let got = peak_hz(f);
    assert!(
        (got - want).abs() <= tol.max(1.0),
        "expected the {lane} lane (marker at {want:.0} Hz), but the peak is at {got:.0} Hz \
         in a {:.0} Hz window centred on {:.0} Hz",
        f.span_hz,
        f.center_hz,
    );
}

/// The regression: a window well inside the passband, at a dial the engine did
/// not open on. It has to be drawn from the I/Q.
#[test]
fn a_window_inside_the_passband_keeps_the_iq_after_a_retune() {
    let f = frame_at(MOVED, 2_000_000.0);
    drew_on(&f, MOVED + IQ_TONE_OFFSET, "I/Q");
}

/// The same window at the dial the engine opened on, which worked all along —
/// so a failure above is about the retune and not about the lane choice.
#[test]
fn it_worked_at_the_centre_the_engine_opened_on() {
    let f = frame_at(START, 2_000_000.0);
    drew_on(&f, START + IQ_TONE_OFFSET, "I/Q");
}

/// And the lane the fallback exists for is still reached: zoom out past the
/// I/Q and the overview draws.
#[test]
fn zooming_out_past_the_iq_still_reaches_the_overview() {
    let f = frame_at(MOVED, 30_000_000.0);
    drew_on(&f, WIDE_MARKER_HZ, "overview");
}
