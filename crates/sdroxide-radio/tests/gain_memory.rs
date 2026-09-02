//! The settings an operator arrives at by listening — the front end's gain
//! stages, the squelch, the noise reduction — have to be where they left them
//! at the next start. A receiver that comes back up on its driver's power-up
//! gains with the squelch wide open is one the operator has to set up again
//! every session, and the figures they had were arrived at against their own
//! antenna and noise floor.
//!
//! One test function on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! this one writes a real `session.json` under it.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, RadioError, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, GainElement, NrLevel, RadioEvent, RxId};

const RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;

/// Where a front end's gain stages actually are, shared with the test so it can
/// see what reached the hardware rather than only what the state says.
#[derive(Clone)]
struct Stages {
    rx: Arc<Mutex<Vec<(String, f64)>>>,
    tx: Arc<Mutex<Vec<(String, f64)>>>,
    /// Every set the engine made, in order.
    asked: Arc<Mutex<Vec<(Direction, String, f64)>>>,
}

impl Stages {
    /// A device as its driver powers it up: everything at the bottom of its
    /// range, which is what a reopen lands on however the last one was left.
    fn fresh() -> Self {
        Stages {
            rx: Arc::new(Mutex::new(vec![("LNA".into(), 0.0), ("VGA".into(), 0.0)])),
            tx: Arc::new(Mutex::new(vec![("PA".into(), 0.0)])),
            asked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn rx_db(&self, name: &str) -> f64 {
        self.rx.lock().unwrap().iter().find(|(n, _)| n == name).map(|(_, d)| *d).unwrap()
    }

    fn tx_db(&self, name: &str) -> f64 {
        self.tx.lock().unwrap().iter().find(|(n, _)| n == name).map(|(_, d)| *d).unwrap()
    }

    fn asked(&self) -> Vec<(Direction, String, f64)> {
        self.asked.lock().unwrap().clone()
    }
}

struct Rig {
    stages: Stages,
}

impl IqSource for Rig {
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
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "gain rig".into()
    }
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.stages.asked.lock().unwrap().push((Direction::Rx, name.into(), db));
        let mut g = self.stages.rx.lock().unwrap();
        match g.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = db,
            // A driver refuses a stage it does not have; the engine is not
            // supposed to ask for one in the first place.
            None => return Err(RadioError::Msg(format!("no such gain element: {name}"))),
        }
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        self.stages.rx.lock().unwrap().clone()
    }
    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.stages.asked.lock().unwrap().push((Direction::Tx, name.into(), db));
        let mut g = self.stages.tx.lock().unwrap();
        match g.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = db,
            None => return Err(RadioError::Msg(format!("no such TX gain element: {name}"))),
        }
        Ok(())
    }
    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        self.stages.tx.lock().unwrap().clone()
    }
}

fn caps() -> DeviceCaps {
    let el = |name: &str, dir, max_db| GainElement::db(name, dir, 0.0, max_db, 1.0);
    DeviceCaps {
        driver: "bench".into(),
        label: "bench rig".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        freq_ranges_tx: vec![(0.0, 60_000_000.0)],
        gains: vec![
            el("LNA", Direction::Rx, 40.0),
            el("VGA", Direction::Rx, 62.0),
            el("PA", Direction::Tx, 47.0),
        ],
        ..DeviceCaps::default()
    }
}

fn remembering(stages: &Stages) -> (Box<dyn IqSource>, DeviceCaps, EngineConfig) {
    let cfg = EngineConfig { remember_session: true, ..Default::default() };
    (Box::new(Rig { stages: stages.clone() }), caps(), cfg)
}

/// Wait for a published state that satisfies `want`, so the assertions are
/// about what every UI sees rather than about internal timing.
fn wait_for_state(
    rx: &crossbeam_channel::Receiver<RadioEvent>,
    want: impl Fn(&sdroxide_types::RadioState) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(RadioEvent::State(s)) if want(&s) => return true,
            Ok(_) => {}
            Err(_) => {}
        }
    }
    false
}

#[test]
fn the_gains_squelch_and_nr_survive_a_restart() {
    let root = std::env::temp_dir().join(format!("sdroxide-gain-memory-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // ---- The session the operator works in ----
    let first = Stages::fresh();
    let (src, caps, cfg) = remembering(&first);
    let mut h = start_engine(src, caps, cfg);
    let thread = h.thread.take();

    h.cmd_tx
        .send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 24.0 })
        .unwrap();
    h.cmd_tx
        .send(Command::SetGain { dir: Direction::Rx, element: "VGA".into(), db: 30.0 })
        .unwrap();
    h.cmd_tx.send(Command::SetGain { dir: Direction::Tx, element: "PA".into(), db: 20.0 }).unwrap();
    h.cmd_tx.send(Command::SetSquelch { rx: RxId::Main, db: -60.0 }).unwrap();
    h.cmd_tx.send(Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::RnnMed }).unwrap();
    assert!(
        wait_for_state(&h.event_rx, |s| s.rx[0].squelch_db == -60.0
            && s.rx[0].noise_reduction == NrLevel::RnnMed
            && s.gains.contains(&("LNA".to_string(), 24.0))),
        "the settings must reach the published state first"
    );
    assert_eq!(first.rx_db("VGA"), 30.0, "and the hardware");
    assert_eq!(first.tx_db("PA"), 20.0);

    // Quitting is what writes the session; joining the thread makes sure the
    // file is on disk before it is read back.
    drop(h);
    let _ = thread.map(|t| t.join());

    let saved = sdroxide_config::load_session();
    assert_eq!(saved.squelch_db, -60.0, "the squelch has to be in session.json");
    assert_eq!(saved.noise_reduction, NrLevel::RnnMed);
    assert_eq!(saved.gains, vec![("LNA".into(), 24.0), ("VGA".into(), 30.0)]);
    assert_eq!(saved.tx_gains, vec![("PA".into(), 20.0)]);

    // ---- The next start, on a device fresh out of its driver ----
    let second = Stages::fresh();
    let (src, caps, cfg) = remembering(&second);
    let mut h = start_engine(src, caps, cfg);
    let thread = h.thread.take();

    assert!(
        wait_for_state(&h.event_rx, |s| s.rx[0].squelch_db == -60.0
            && s.rx[0].noise_reduction == NrLevel::RnnMed
            && s.gains.contains(&("LNA".to_string(), 24.0))
            && s.gains.contains(&("VGA".to_string(), 30.0))),
        "the second start must come up on the settings the first one was left with"
    );
    assert_eq!(second.rx_db("LNA"), 24.0, "the remembered gain has to reach the hardware");
    assert_eq!(second.rx_db("VGA"), 30.0);
    assert_eq!(second.tx_db("PA"), 20.0);

    drop(h);
    let _ = thread.map(|t| t.join());

    // ---- A session carrying a stage this front end does not have ----
    // The operator switched interfaces: the memory outlived the device it was
    // taken from. Asking for a stage the driver has never heard of only makes
    // it log an error, and a figure past the range this device offers has to
    // land on the nearest setting it can actually do.
    let mut edited = sdroxide_config::load_session();
    edited.gains = vec![("HIFI".into(), 99.0), ("LNA".into(), 999.0)];
    sdroxide_config::save_session(&edited).unwrap();

    let third = Stages::fresh();
    let (src, caps, cfg) = remembering(&third);
    let mut h = start_engine(src, caps, cfg);
    let thread = h.thread.take();

    assert!(wait_for_state(&h.event_rx, |s| s.gains.contains(&("LNA".to_string(), 40.0))));
    assert_eq!(third.rx_db("LNA"), 40.0, "clamped to this device's maximum, not dropped");
    assert!(
        !third.asked().iter().any(|(_, name, _)| name == "HIFI"),
        "a stage this front end does not have must never be asked for"
    );

    drop(h);
    let _ = thread.map(|t| t.join());

    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
    let _ = std::fs::remove_dir_all(&root);
}
