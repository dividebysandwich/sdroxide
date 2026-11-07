//! What the panadapter shows while a full-duplex receiver transmits.
//!
//! A half-duplex radio goes deaf when keyed, so the panadapter has nothing to
//! draw from the receiver and shows the transmit monitor instead. A full-duplex
//! one keeps being read through the over — the Pluto's "keep receiving while
//! transmitting", a paired receiver with blanking off — and there the receiver
//! is the picture: a QO-100 station watches its own downlink come back while it
//! transmits. The engine kept switching to the transmit monitor anyway, centred
//! on the uplink, so the spectrum went blank for the over (#652).

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId, SpectrumFrame};

const RATE: f64 = 768_000.0;
const CENTER: f64 = 14_074_000.0;
/// Where the received signal sits, well clear of the dial and its carrier.
const TONE: f64 = 150_000.0;

/// A transmit-capable source whose receiver hears one steady carrier.
struct Downlink {
    phase: f64,
}

impl IqSource for Downlink {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Paced like hardware, so the engine loop ticks at the real rate.
        let n = buf.len().min(4096);
        std::thread::sleep(Duration::from_secs_f64(n as f64 / RATE));
        let step = std::f64::consts::TAU * TONE / RATE;
        for b in buf.iter_mut().take(n) {
            let (s, c) = self.phase.sin_cos();
            *b = Complex32::new(c as f32, s as f32) * 0.1;
            self.phase = (self.phase + step) % std::f64::consts::TAU;
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "downlink".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn caps(full_duplex: bool) -> DeviceCaps {
    DeviceCaps {
        driver: "downlink".into(),
        label: "downlink".into(),
        rx_channels: 1,
        tx_channels: 1,
        full_duplex,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// How far the tone's bin stands above the frame's median bin.
fn tone_above_median(f: &SpectrumFrame) -> i32 {
    let n = f.bins.len();
    let lo = f.center_hz - f.span_hz / 2.0;
    let at = ((CENTER + TONE - lo) / f.span_hz * n as f64) as usize;
    let peak = f.bins[at.saturating_sub(2)..(at + 3).min(n)].iter().copied().max().unwrap_or(0);
    let mut sorted = f.bins.clone();
    sorted.sort_unstable();
    i32::from(peak) - i32::from(sorted[n / 2])
}

/// Key a carrier and hand back the last frame published while keyed.
fn frame_during_tx(full_duplex: bool) -> SpectrumFrame {
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let mut h = start_engine(Box::new(Downlink { phase: 0.0 }), caps(full_duplex), cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.cmd_tx.send(Command::SetTune(true)).unwrap();
    // Past the first frames after key-down, which may still be receive ones.
    std::thread::sleep(Duration::from_millis(300));
    let deadline = Instant::now() + Duration::from_millis(600);
    let mut last = None;
    while Instant::now() < deadline {
        if h.spectrum_out.update() {
            let frame = h.spectrum_out.output_buffer();
            if !frame.bins.is_empty() {
                last = Some(frame.clone());
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    h.cmd_tx.send(Command::SetTune(false)).unwrap();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("frames are published while keyed")
}

#[test]
fn a_full_duplex_receiver_stays_on_the_panadapter_through_an_over() {
    let f = frame_during_tx(true);
    assert_eq!(f.center_hz, CENTER, "the frame is the receiver's, not the uplink's");
    assert!(f.rows_clocked, "the waterfall keeps scrolling the receiver's rows");
    let above = tone_above_median(&f);
    assert!(above > 40, "the received carrier should stand out, not {above} steps");
}

#[test]
fn a_half_duplex_radio_still_shows_the_transmit_monitor() {
    let f = frame_during_tx(false);
    assert!(!f.rows_clocked, "a transmit monitor does not clock rows");
}
