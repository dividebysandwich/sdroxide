//! **A receive-only mode never keys the transmitter** (issue #239).
//!
//! The strip stopped offering a PTT in WEFAX, NAVTEX and the rest, but a hidden
//! button is not a protection: a keyboard binding, a MIDI button, an external
//! PTT line, a TCI client, a remote `SetPtt` and a digital burst all reach the
//! engine without touching the chrome. This pins the engine's own refusal, and
//! the other direction too — switching to a receive-only mode while the
//! transmitter is keyed must let go of it, because the controls that could
//! unkey it are the ones that have just gone away.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId};

#[derive(Default)]
struct Log {
    began: usize,
    ended: usize,
}

#[derive(Clone, Default)]
struct Rig {
    log: Arc<Mutex<Log>>,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        192_000.0
    }
    fn center_hz(&self) -> f64 {
        3_855_000.0
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
        "fax rig".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        self.log.lock().unwrap().began += 1;
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        self.log.lock().unwrap().ended += 1;
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "fax".into(),
        label: "fax".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![192_000.0],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn engine() -> (sdroxide_radio::EngineHandles, Arc<Mutex<Log>>, Option<std::thread::JoinHandle<()>>)
{
    let log = Arc::new(Mutex::new(Log::default()));
    let (producer, _consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let cfg = EngineConfig {
        tx_ham_only: false,
        audio: Some(sdroxide_radio::AudioParams { producer, out_rate: 48_000.0 }),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { log: Arc::clone(&log) }), caps(), cfg);
    let thread = h.thread.take();
    (h, log, thread)
}

fn stop(h: sdroxide_radio::EngineHandles, thread: Option<std::thread::JoinHandle<()>>) {
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// PTT and TUNE are both refused in WEFAX, however they are asked for — and the
/// refusal is silent on the air, not merely absent from the screen.
#[test]
fn a_receive_only_mode_never_keys_ptt_or_tune() {
    let (h, log, thread) = engine();
    // A weather fax frequency, because the mode belongs on shortwave and the
    // engine refuses a mode on a band it is not used on.
    h.cmd_tx.send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 3_855_000.0 }).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Wefax }).unwrap();
    std::thread::sleep(Duration::from_millis(400));

    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.cmd_tx.send(Command::SetTune(true)).unwrap();
    std::thread::sleep(Duration::from_millis(400));

    let began = log.lock().unwrap().began;
    assert_eq!(began, 0, "a weather fax keyed the transmitter {began} times");

    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    h.cmd_tx.send(Command::SetTune(false)).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    stop(h, thread);
}

/// The other direction: keyed on a voice mode, then the operator picks WEFAX.
/// The transmitter has to come off the air, because the controls that could do
/// it are gone with the PTT box.
#[test]
fn switching_to_a_receive_only_mode_unkeys_the_transmitter() {
    let (h, log, thread) = engine();
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    std::thread::sleep(Duration::from_millis(400));

    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(log.lock().unwrap().began, 1, "USB did not key — the test proves nothing");

    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Wefax }).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    let l = log.lock().unwrap();
    assert_eq!(l.ended, 1, "the transmitter stayed keyed into a receive-only mode");
    drop(l);

    // And it stays off: nothing re-keys it once the mode is on.
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(log.lock().unwrap().began, 1, "it keyed again after the mode change");
    stop(h, thread);
}

/// A transmitting mode on the same radio still keys — the guard is about the
/// mode, not about the band or the rig.
#[test]
fn a_transmitting_mode_still_keys() {
    let (h, log, thread) = engine();
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Usb }).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(log.lock().unwrap().began, 1, "USB on the same radio stopped transmitting");
    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    stop(h, thread);
}
