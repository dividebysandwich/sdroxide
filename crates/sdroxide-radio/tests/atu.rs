//! A radio's built-in antenna tuner: offered only where the radio has one,
//! started and bypassed on the operator's click, and shown as the radio reports
//! it — never as what sdroxide hoped would happen.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{AtuState, AtuStatus, Command, DeviceCaps, RadioEvent};

const RATE: f64 = 48_000.0;
const CENTER: f64 = 7_100_000.0;

/// A transceiver whose tuner the test can see into: what it reports, and what
/// the engine asked of it.
struct AtuRig {
    /// What the radio says its tuner is doing. `None` is a radio without one.
    atu: Arc<Mutex<Option<AtuState>>>,
    tunes: Arc<Mutex<usize>>,
    bypasses: Arc<Mutex<usize>>,
}

impl AtuRig {
    fn new(atu: Option<AtuState>) -> AtuRig {
        AtuRig {
            atu: Arc::new(Mutex::new(atu)),
            tunes: Arc::new(Mutex::new(0)),
            bypasses: Arc::new(Mutex::new(0)),
        }
    }
}

impl IqSource for AtuRig {
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
        "rig with an antenna tuner".into()
    }
    fn atu(&self) -> Option<AtuState> {
        *self.atu.lock().unwrap()
    }
    fn atu_tune(&mut self) -> Result<()> {
        *self.tunes.lock().unwrap() += 1;
        // The radio starts its tune; its report follows.
        if let Some(a) = self.atu.lock().unwrap().as_mut() {
            a.status = AtuStatus::Tuning;
        }
        Ok(())
    }
    fn atu_bypass(&mut self) -> Result<()> {
        *self.bypasses.lock().unwrap() += 1;
        if let Some(a) = self.atu.lock().unwrap().as_mut() {
            a.status = AtuStatus::Bypass;
            a.enabled = false;
        }
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "smartsdr".into(),
        label: "FLEX".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        freq_ranges_tx: vec![(0.0, 60_000_000.0)],
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

const TUNED: AtuState = AtuState { status: AtuStatus::Tuned, enabled: true, using_memory: true };

/// The control appears only where the radio says it has a tuner.
#[test]
fn the_tuner_is_published_only_where_the_radio_has_one() {
    for (atu, want) in [(None, false), (Some(TUNED), true)] {
        let mut h = start_engine(Box::new(AtuRig::new(atu)), caps(), EngineConfig::default());
        let thread = h.thread.take();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = None;
        while Instant::now() < deadline && seen.is_none() {
            if let Ok(RadioEvent::Capabilities(c) | RadioEvent::CapabilitiesUpdated(c)) =
                h.event_rx.recv_timeout(Duration::from_millis(100))
            {
                seen = Some(c.has_atu);
            }
        }
        assert_eq!(seen, Some(want), "a radio reporting {atu:?}");
        drop(h);
        let _ = thread.map(|t| t.join());
    }
}

/// A click reaches the radio, and what is shown afterwards is the radio's own
/// account of it, all the way to the end of the tune.
#[test]
fn a_tune_reaches_the_radio_and_its_outcome_is_shown() {
    let rig = AtuRig::new(Some(TUNED));
    let (atu, tunes, bypasses) = (rig.atu.clone(), rig.tunes.clone(), rig.bypasses.clone());
    let mut h = start_engine(Box::new(rig), caps(), EngineConfig::default());
    let thread = h.thread.take();

    assert!(wait_for_state(&h.event_rx, |s| s.atu == TUNED), "the radio's state is adopted");

    h.cmd_tx.send(Command::AtuTune).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.atu.status == AtuStatus::Tuning));
    assert_eq!(*tunes.lock().unwrap(), 1);

    // The radio finishing the tune on its own.
    *atu.lock().unwrap() =
        Some(AtuState { status: AtuStatus::Tuned, enabled: true, using_memory: false });
    assert!(wait_for_state(&h.event_rx, |s| s.atu.status == AtuStatus::Tuned));

    h.cmd_tx.send(Command::AtuBypass).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.atu.status == AtuStatus::Bypass && !s.atu.enabled));
    assert_eq!(*bypasses.lock().unwrap(), 1);

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// On a radio without a tuner the commands go nowhere.
#[test]
fn a_radio_without_a_tuner_is_never_asked_to_tune() {
    let rig = AtuRig::new(None);
    let (tunes, bypasses) = (rig.tunes.clone(), rig.bypasses.clone());
    let mut h = start_engine(Box::new(rig), caps(), EngineConfig::default());
    let thread = h.thread.take();

    h.cmd_tx.send(Command::AtuTune).unwrap();
    h.cmd_tx.send(Command::AtuBypass).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!((*tunes.lock().unwrap(), *bypasses.lock().unwrap()), (0, 0));

    drop(h);
    let _ = thread.map(|t| t.join());
}
