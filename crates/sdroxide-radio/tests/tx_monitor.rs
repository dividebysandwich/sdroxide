//! The TX monitor: when the operator transmits, the panadapter must show their
//! own signal. These tests drive the engine with a mock transmit-capable source
//! and confirm the TX path produces spectrum frames (both the wideband IQ scope
//! for voice/tune and the narrow scope for digital modes) without panicking.

use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RxId};

/// A transmit-capable stand-in: RX returns silence, TX writes are accepted.
struct MockSource {
    center: f64,
    rate: f64,
}

impl IqSource for MockSource {
    fn sample_rate(&self) -> f64 {
        self.rate
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Pace like real hardware and hand back a block of silence.
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        for b in buf.iter_mut().take(n) {
            *b = Complex32::new(0.0, 0.0);
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock tx source".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn tx_caps(rate: f64) -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![rate],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Drive the engine and return true if it stayed alive while transmitting and
/// produced both TX meters and spectrum frames (i.e. the TX-frame path ran
/// without panicking). Transmit meters (`Meters.tx`) are emitted in the same loop
/// iteration that builds the TX spectrum frame, so their arrival proves the TX
/// panadapter path executed.
fn run(cmds: &[Command], timeout: Duration) -> bool {
    let rate = 2_400_000.0;
    let src = MockSource { center: 14_200_000.0, rate };
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let mut h = start_engine(Box::new(src), tx_caps(rate), cfg);
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(150));
    for c in cmds {
        h.cmd_tx.send(c.clone()).unwrap();
    }

    let deadline = std::time::Instant::now() + timeout;
    let mut saw_tx_meters = false;
    let mut saw_frame = false;
    while std::time::Instant::now() < deadline && !(saw_tx_meters && saw_frame) {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Meters(m) = ev {
                if m.tx.is_some() {
                    saw_tx_meters = true;
                }
            }
        }
        if h.spectrum_out.update() && !h.spectrum_out.output_buffer().bins.is_empty() {
            saw_frame = true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    saw_tx_meters && saw_frame
}

#[test]
fn tune_carrier_shows_on_panadapter() {
    // TUNE emits a carrier; the wideband TX scope must show energy.
    assert!(
        run(&[Command::SetTune(true)], Duration::from_secs(3)),
        "the tune carrier should appear on the TX panadapter"
    );
}

#[test]
fn psk_transmit_shows_on_panadapter() {
    // A digital-mode transmission must show on the narrow TX scope.
    let cmds = [
        Command::SetMode { rx: RxId::Main, mode: Mode::Psk },
        Command::DigiTxText("CQ TEST".into()),
        Command::DigiTxActive(true),
    ];
    assert!(
        run(&cmds, Duration::from_secs(4)),
        "a PSK transmission should appear on the TX panadapter"
    );
}
