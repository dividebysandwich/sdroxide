//! Switching a radio off and on again is a power switch, not a factory reset:
//! the front-end decimation and the gain stages the operator arrived at have to
//! be exactly where they were when the interface comes back (issue #209).
//!
//! While the radio is off its front end is a 48 kHz stand-in with no gain to
//! set — which is the whole trap. Nothing the stand-in can carry may be allowed
//! to overwrite what the operator asked for.
//!
//! One test function on purpose: `SDROXIDE_CONFIG_DIR` is process-global and
//! this one writes a real `session.json` under it.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineSwap, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, GainElement, RadioEvent};

const WIDE_RATE: f64 = 2_000_000.0;
const STANDIN_RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;

/// The real front end: wide enough to have decimation worth setting, with two
/// gain stages that come up on their driver's power-up values.
struct Wide {
    gains: Arc<Mutex<Vec<(String, f64)>>>,
}

impl IqSource for Wide {
    fn sample_rate(&self) -> f64 {
        WIDE_RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(4096);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "wide rig".into()
    }
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        let mut g = self.gains.lock().unwrap();
        match g.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = db,
            None => return Err(sdroxide_radio::RadioError::Msg(format!("no stage {name}"))),
        }
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        self.gains.lock().unwrap().clone()
    }
}

/// What a radio switched off has instead of an interface: 48 kHz of nothing,
/// no gain stages, and no wish to be reopened.
struct SwitchedOff;

impl IqSource for SwitchedOff {
    fn sample_rate(&self) -> f64 {
        STANDIN_RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, _buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        Ok(0)
    }
    fn describe(&self) -> String {
        "no radio".into()
    }
}

fn wide_caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "wide rig".into(),
        rx_channels: 1,
        sample_rates: vec![WIDE_RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        gains: vec![GainElement::db("LNA", Direction::Rx, -6.0, 40.0, 1.0)],
        ..DeviceCaps::default()
    }
}

fn off_caps() -> DeviceCaps {
    DeviceCaps {
        driver: "off".into(),
        label: "Switched off".into(),
        rx_channels: 1,
        sample_rates: vec![STANDIN_RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn wait_for_state(
    rx: &crossbeam_channel::Receiver<RadioEvent>,
    want: impl Fn(&sdroxide_types::RadioState) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(RadioEvent::State(s)) = rx.recv_timeout(Duration::from_millis(100))
            && want(&s)
        {
            return true;
        }
    }
    false
}

#[test]
fn the_decimation_and_gains_survive_the_power_switch() {
    let root = std::env::temp_dir().join(format!("sdroxide-power-switch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let off = Arc::new(AtomicBool::new(false));
    let gains = Arc::new(Mutex::new(vec![("LNA".to_string(), 0.0)]));
    let (f_off, f_gains) = (Arc::clone(&off), Arc::clone(&gains));
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        if f_off.load(Ordering::SeqCst) {
            return Ok((Box::new(SwitchedOff) as Box<dyn IqSource>, off_caps()));
        }
        // A device fresh out of its driver, exactly as a reopen finds one.
        *f_gains.lock().unwrap() = vec![("LNA".to_string(), 0.0)];
        Ok((Box::new(Wide { gains: Arc::clone(&f_gains) }) as Box<dyn IqSource>, wide_caps()))
    });

    let cfg = EngineConfig { remember_session: true, reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Wide { gains: Arc::clone(&gains) }), wide_caps(), cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetDecimation(4)).unwrap();
    h.cmd_tx
        .send(Command::SetGain { dir: Direction::Rx, element: "LNA".into(), db: 24.0 })
        .unwrap();
    assert!(
        wait_for_state(&h.event_rx, |s| s.decimation == 4
            && s.gains.contains(&("LNA".to_string(), 24.0))),
        "the operator's decimation and gain must reach the published state first"
    );

    // ---- Off ----
    off.store(true, Ordering::SeqCst);
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();
    assert!(
        wait_for_state(&h.event_rx, |s| s.sample_rate == STANDIN_RATE),
        "the stand-in must be on the air"
    );

    // ---- And on again ----
    off.store(false, Ordering::SeqCst);
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();
    assert!(
        wait_for_state(&h.event_rx, |s| s.sample_rate == WIDE_RATE / 4.0),
        "the radio comes back on the decimation it was switched off with"
    );
    assert!(
        wait_for_state(&h.event_rx, |s| s.decimation == 4
            && s.gains.contains(&("LNA".to_string(), 24.0))),
        "…and on its gain stages"
    );
    assert_eq!(gains.lock().unwrap()[0].1, 24.0, "the gain has to reach the hardware");

    drop(h);
    let _ = thread.map(|t| t.join());

    let saved = sdroxide_config::load_session();
    assert_eq!(saved.decimation, 4, "and session.json must not have been overwritten with 1");
    assert_eq!(saved.gains, vec![("LNA".into(), 24.0)]);

    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
    let _ = std::fs::remove_dir_all(&root);
}
