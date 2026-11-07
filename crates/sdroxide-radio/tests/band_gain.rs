//! Receive gain remembered per band (#605): an RTL-SDR wants little gain on HF
//! and a lot on 2 m, and the operator should not have to set it again on every
//! change of band.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, RadioError, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, GainElement, Vfo};

const RATE: f64 = 2_000_000.0;

struct Rig {
    center: f64,
    lna: Arc<Mutex<f64>>,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(4096);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "gain rig".into()
    }
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        if name != "LNA" {
            return Err(RadioError::Msg(format!("no such gain element: {name}")));
        }
        *self.lna.lock().unwrap() = db;
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![("LNA".into(), *self.lna.lock().unwrap())]
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        gains: vec![GainElement::db("LNA", Direction::Rx, 0.0, 50.0, 1.0)],
        ..DeviceCaps::default()
    }
}

/// Wait until the hardware's LNA reads `want`, or say what it read.
fn lna_becomes(lna: &Arc<Mutex<f64>>, want: f64, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if *lna.lock().unwrap() == want {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("{what}: LNA is {} dB, not {want} dB", lna.lock().unwrap());
}

#[test]
fn each_band_gets_back_the_gain_last_set_on_it() {
    let root = std::env::temp_dir().join(format!("sdroxide-band-gain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let lna = Arc::new(Mutex::new(0.0));
    let mut h = start_engine(
        Box::new(Rig { center: 7_074_000.0, lna: Arc::clone(&lna) }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();
    let settle = || std::thread::sleep(Duration::from_millis(250));

    send(Command::SetVfo { vfo: Vfo::A, hz: 7_074_000.0 });
    settle();
    send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 10.0 });
    lna_becomes(&lna, 10.0, "set on 40 m");

    send(Command::SetVfo { vfo: Vfo::A, hz: 145_500_000.0 });
    settle();
    // A band never set keeps the gain in force.
    assert_eq!(*lna.lock().unwrap(), 10.0, "2 m had nothing remembered");
    send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 40.0 });
    lna_becomes(&lna, 40.0, "set on 2 m");

    send(Command::SetVfo { vfo: Vfo::A, hz: 7_100_000.0 });
    lna_becomes(&lna, 10.0, "back on 40 m");
    send(Command::SetVfo { vfo: Vfo::A, hz: 145_400_000.0 });
    lna_becomes(&lna, 40.0, "back on 2 m");

    // General coverage is split by range: the airband is not HF broadcast.
    send(Command::SetVfo { vfo: Vfo::A, hz: 6_000_000.0 });
    settle();
    send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 5.0 });
    lna_becomes(&lna, 5.0, "set in HF general coverage");
    send(Command::SetVfo { vfo: Vfo::A, hz: 120_000_000.0 });
    settle();
    send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 30.0 });
    lna_becomes(&lna, 30.0, "set in VHF general coverage");
    send(Command::SetVfo { vfo: Vfo::A, hz: 6_100_000.0 });
    lna_becomes(&lna, 5.0, "back in HF general coverage");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&root);
}
