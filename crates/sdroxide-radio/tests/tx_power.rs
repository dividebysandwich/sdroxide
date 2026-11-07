//! The transmit level on a rig that has its own power control — a CAT rig, a
//! TCI rig — reaches it before the transmitter comes up, whichever way the
//! transmitter comes up.
//!
//! On such a rig the Drive slider is not a gain in our own chain: the audio we
//! send is only the modulating signal, and on CW there is no audio at all
//! because the rig keys itself from its own keyer. The slider means something
//! only once it has been commanded to the radio, and these hold the two paths
//! where nothing else would command it — a CW message handed to the rig's
//! keyer, which never goes near the transmit chain, and the end of a TUNE,
//! which otherwise walks away leaving the radio at the tune level.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, ControlUpdate, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RxId};

const RATE: f64 = 48_000.0;

/// What the rig was told, in the order it was told.
#[derive(Debug, Clone, PartialEq)]
enum Told {
    Power(f64),
    Cw(String),
    Key(bool),
}

/// A stand-in for a rig with its own power control and its own keyer: a CAT
/// transceiver on a sound card.
struct PowerRig {
    center: f64,
    log: Arc<Mutex<Vec<Told>>>,
    /// Report every level it is told back as its own, the way a rig with
    /// auto-information on does.
    echo: bool,
    reports: Vec<ControlUpdate>,
}

impl IqSource for PowerRig {
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
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock rig with its own power control".into()
    }
    fn display_bandwidth(&self) -> Option<f64> {
        Some(4000.0)
    }
    fn commands_tx_power(&self) -> bool {
        true
    }
    fn set_tx_drive(&mut self, frac: f64) {
        self.log.lock().unwrap().push(Told::Power(frac));
        if self.echo {
            self.reports.push(ControlUpdate::TxDrive(frac as f32));
        }
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        std::mem::take(&mut self.reports)
    }
    fn cw_text_keying(&self) -> Option<usize> {
        Some(30)
    }
    fn send_cw(&mut self, text: &str) {
        self.log.lock().unwrap().push(Told::Cw(text.to_string()));
    }
    fn tx_begin(&mut self, _center_hz: f64, _rate: f64) -> Result<f64> {
        self.log.lock().unwrap().push(Told::Key(true));
        Ok(RATE)
    }
    fn tx_end(&mut self) -> Result<()> {
        self.log.lock().unwrap().push(Told::Key(false));
        Ok(())
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "cat".into(),
        label: "mock CAT rig".into(),
        rx_channels: 1,
        tx_channels: 1,
        audio_mode: true,
        freq_ranges_rx: vec![(10_000.0, 148_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Run the engine on the stand-in rig, feed it `cmds`, and return everything the
/// rig was told. `settle` is how long to keep the engine running afterwards, for
/// the paths that take a moment (the CW keyer hands its queue over on a tick).
fn run(cmds: Vec<Command>, settle: Duration) -> Vec<Told> {
    run_rig(cmds, settle, false)
}

fn run_rig(cmds: Vec<Command>, settle: Duration, echo: bool) -> Vec<Told> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let src = PowerRig { center: 14_074_000.0, log: Arc::clone(&log), echo, reports: Vec::new() };
    let mut h = start_engine(Box::new(src), caps(), EngineConfig::default());
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(300));
    for c in cmds {
        h.cmd_tx.send(c).expect("engine gone");
        std::thread::sleep(Duration::from_millis(60));
    }
    let deadline = Instant::now() + settle;
    while Instant::now() < deadline {
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(h.cmd_tx);
    drop(h.event_rx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let told = log.lock().unwrap().clone();
    told
}

/// CW on a rig that keys itself never touches the transmit chain — no PTT, no
/// modulator, no audio — so nothing on that path could assert the drive. Before
/// this it was asserted nowhere at all, and the Drive slider did nothing in the
/// one mode where the rig's power is the only transmit control there is.
#[test]
fn a_message_handed_to_the_rigs_keyer_is_sent_at_the_drive_the_operator_set() {
    let told = run(
        vec![
            Command::SetMode { rx: RxId::Main, mode: Mode::Cw },
            Command::SetTxDrive(0.42),
            Command::DigiTxText("cq de w1aw\n".into()),
            Command::DigiTxActive(true),
        ],
        Duration::from_millis(1200),
    );
    let cw = told.iter().position(|t| matches!(t, Told::Cw(_))).unwrap_or_else(|| {
        panic!("the rig was never given the message to key: {told:?}");
    });
    // The last thing it was told before the message is the level to key it at.
    let power = told[..cw]
        .iter()
        .rev()
        .find_map(|t| match t {
            Told::Power(v) => Some(*v),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no drive was asserted before the message: {told:?}"));
    assert!((power - 0.42).abs() < 1e-6, "keyed at {power} instead of the 0.42 asked for");
    // And it keyed itself: no PTT was asserted around it, which on a rig whose
    // keyer does the keying would hold a carrier the keyer cannot key.
    assert!(!told[..cw].contains(&Told::Key(true)), "CW must not be wrapped in PTT: {told:?}");
}

/// TUNE holds the rig at the (deliberately low) tune level. A radio left there
/// afterwards answers the next call — from its own hand microphone, where
/// nothing here is in the way — at a few watts.
#[test]
fn the_operating_level_comes_back_when_a_tune_ends() {
    for echo in [false, true] {
        the_operating_level_comes_back(echo);
    }
}

/// With `echo`, the rig reports the tune level back while TUNE is keyed. That
/// is the rig on a level this end chose, not the operator's, and adopting it
/// left Drive at the tune level after every TUNE (#741).
fn the_operating_level_comes_back(echo: bool) {
    let told = run_rig(
        vec![
            Command::SetTxDrive(0.6),
            Command::SetTuneDrive(0.05),
            Command::SetTune(true),
            Command::SetTune(false),
        ],
        Duration::from_millis(300),
        echo,
    );
    let up = told.iter().position(|t| t == &Told::Key(true)).expect("TUNE never keyed");
    let down = told.iter().position(|t| t == &Told::Key(false)).expect("TUNE never unkeyed");
    let before_key = |from: usize| {
        told[..from].iter().rev().find_map(|t| match t {
            Told::Power(v) => Some(*v),
            _ => None,
        })
    };
    let tune = before_key(up).unwrap_or_else(|| panic!("no level before the tune: {told:?}"));
    assert!((tune - 0.05).abs() < 1e-6, "tuned at {tune} instead of the 0.05 tune level");
    let after = told[down..]
        .iter()
        .find_map(|t| match t {
            Told::Power(v) => Some(*v),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the rig was left at the tune level: {told:?}"));
    assert!((after - 0.6).abs() < 1e-6, "left at {after} instead of the 0.6 operating level");
}
