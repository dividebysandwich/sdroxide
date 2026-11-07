//! PTT in CW (#625). A keyer with a PTT line holds the transmitter for the
//! length of a transmission and keys the carrier on its own key line. PTT used
//! to key the voice path instead, which in CW is a steady carrier at the tune
//! level — full power for as long as PTT was held.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId};

#[derive(Default)]
struct Log {
    began: bool,
    ended: bool,
    peak: f32,
}

struct Rig {
    log: Arc<Mutex<Log>>,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        192_000.0
    }
    fn center_hz(&self) -> f64 {
        14_030_000.0
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
        "cw rig".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        self.log.lock().unwrap().began = true;
        Ok(rate)
    }
    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        let mut l = self.log.lock().unwrap();
        l.peak = samples.iter().fold(l.peak, |p, z| p.max(z.norm()));
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        self.log.lock().unwrap().ended = true;
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "cw".into(),
        label: "cw".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![192_000.0],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

#[test]
fn ptt_in_cw_keys_the_transmitter_without_a_carrier() {
    let log = Arc::new(Mutex::new(Log::default()));
    let (producer, _consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let cfg = EngineConfig {
        tx_ham_only: false,
        audio: Some(sdroxide_radio::AudioParams { producer, out_rate: 48_000.0 }),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { log: Arc::clone(&log) }), caps(), cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Cw }).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    {
        let l = log.lock().unwrap();
        assert!(l.began, "PTT did not key the transmitter");
        assert!(l.peak < 1e-3, "PTT alone put a carrier on the air, peak {}", l.peak);
    }
    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert!(log.lock().unwrap().ended, "releasing PTT did not unkey");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Issue #716: releasing PTT on an over the key started, not PTT, used to unkey
/// the radio while the keyer stayed latched, and nothing keyed again until
/// CLEAR.
#[test]
fn a_ptt_release_on_a_keyed_over_leaves_the_key_working() {
    let log = Arc::new(Mutex::new(Log::default()));
    let (producer, _consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let cfg = EngineConfig {
        tx_ham_only: false,
        audio: Some(sdroxide_radio::AudioParams { producer, out_rate: 48_000.0 }),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { log: Arc::clone(&log) }), caps(), cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Cw }).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    h.cmd_tx.send(Command::CwStraight(true)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    h.cmd_tx.send(Command::CwKey(true)).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    h.cmd_tx.send(Command::CwKey(false)).unwrap();
    assert!(log.lock().unwrap().began, "the key did not key the transmitter");
    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    assert!(log.lock().unwrap().ended, "releasing PTT did not end the over");

    *log.lock().unwrap() = Log::default();
    h.cmd_tx.send(Command::CwKey(true)).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    h.cmd_tx.send(Command::CwKey(false)).unwrap();
    {
        let l = log.lock().unwrap();
        assert!(l.began, "the key no longer keys after the PTT release");
        assert!(l.peak > 1e-3, "keyed, but no carrier");
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
