//! **Two-tone belongs to the TUNE it was chosen for** (review of #743).
//!
//! Only the main window's chips send `SetTuneTwoTone`. The footswitch and
//! hotkey, rigctld's `TUNE`, TCI's `tune:`, remote clients and a headless tune
//! all send a bare `SetTune(true)`, and each means a carrier. A two-tone left
//! set after its TUNE ended would key the next of those as a two-tone instead.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId};

#[derive(Clone, Default)]
struct Rig {
    sent: Arc<Mutex<Vec<Complex32>>>,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        192_000.0
    }
    fn center_hz(&self) -> f64 {
        14_200_000.0
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(1920);
        std::thread::sleep(Duration::from_secs_f64(n as f64 / 192_000.0));
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "two-tone rig".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.sent.lock().unwrap().extend_from_slice(samples);
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "twotone".into(),
        label: "twotone".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![192_000.0],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// How far the envelope of what was sent swings, as the ratio of its lowest
/// to its highest magnitude over the middle of the capture (the ends ramp).
/// A steady carrier stays near 1; a two-tone beats down towards 0.
fn envelope_floor(sent: &[Complex32]) -> f32 {
    let mid = &sent[sent.len() / 4..sent.len() * 3 / 4];
    let peak = mid.iter().map(|z| z.norm()).fold(0.0f32, f32::max);
    let low = mid.iter().map(|z| z.norm()).fold(f32::MAX, f32::min);
    low / peak.max(1e-9)
}

#[test]
fn a_bare_tune_after_a_two_tone_is_a_carrier() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let (producer, _consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let cfg = EngineConfig {
        tx_ham_only: false,
        audio: Some(sdroxide_radio::AudioParams { producer, out_rate: 48_000.0 }),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { sent: Arc::clone(&sent) }), caps(), cfg);
    let thread = h.thread.take();
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    std::thread::sleep(Duration::from_millis(400));

    let over = |cmds: &[Command]| {
        sent.lock().unwrap().clear();
        for c in cmds {
            h.cmd_tx.send(c.clone()).unwrap();
        }
        std::thread::sleep(Duration::from_millis(700));
        h.cmd_tx.send(Command::SetTune(false)).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        let s = sent.lock().unwrap().clone();
        assert!(s.len() > 10_000, "the over sent too little to judge: {}", s.len());
        envelope_floor(&s)
    };

    // As the 2T chip sends it: a two-tone, beating down.
    let two_tone = over(&[Command::SetTuneTwoTone(true), Command::SetTune(true)]);
    assert!(two_tone < 0.2, "the 2T over was not a two-tone (floor {two_tone:.2})");

    // As a footswitch, rigctld or TCI sends it: a carrier, not the two-tone
    // the previous TUNE was.
    let carrier = over(&[Command::SetTune(true)]);
    assert!(carrier > 0.8, "a bare TUNE after a 2T over keyed a two-tone (floor {carrier:.2})");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
