use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_config::{Session, Store};
use sdroxide_radio::{
    Complex32, ConvertedSource, ConverterPlan, ConverterStep, EngineConfig, EngineHandles,
    EngineSwap, IqSource, ReopenFn, Result, plan_caps, start_engine,
};
use sdroxide_types::{Band, Command, DeviceCaps, Mode, RadioEvent, RadioState, Vfo};

const HF: f64 = 7_100_000.0;
const RF6: f64 = 50_174_000.0;
const RF2: f64 = 144_174_000.0;
const RATE: f64 = 48_000.0;

#[derive(Default)]
struct HardwareState {
    pa: bool,
    drive: f64,
    tune: f64,
    keyed: Option<(f64, bool)>,
}

struct Rig {
    center: f64,
    pa: bool,
    hardware: Arc<Mutex<HardwareState>>,
}

impl IqSource for Rig {
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock HL2".into()
    }
    fn onboard_pa(&self) -> Option<bool> {
        Some(self.pa)
    }
    fn set_onboard_pa(&mut self, enabled: bool) -> Result<()> {
        self.pa = enabled;
        self.hardware.lock().unwrap().pa = enabled;
        Ok(())
    }
    fn set_tx_drive(&mut self, level: f64) {
        self.hardware.lock().unwrap().drive = level;
    }
    fn set_tune_drive(&mut self, level: f64) {
        self.hardware.lock().unwrap().tune = level;
    }
    fn tx_begin(&mut self, hz: f64, rate: f64) -> Result<f64> {
        self.hardware.lock().unwrap().keyed = Some((hz, self.pa));
        Ok(rate)
    }
    fn tx_write(&mut self, _: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn plan() -> ConverterPlan {
    ConverterPlan::from_steps([
        ConverterStep {
            band: Some((50_000_000.0, 54_000_000.0)),
            rx_offset_hz: -22_000_000.0,
            tx_offset_hz: Some(-22_000_000.0),
            tx_drive: Some(0.03),
        },
        ConverterStep {
            band: Some((144_000_000.0, 148_000_000.0)),
            rx_offset_hz: -116_000_000.0,
            tx_offset_hz: Some(-116_000_000.0),
            tx_drive: Some(0.02),
        },
    ])
}

fn caps() -> DeviceCaps {
    plan_caps(
        DeviceCaps {
            rx_channels: 1,
            tx_channels: 1,
            sample_rates: vec![RATE],
            freq_ranges_rx: vec![(0.0, 38_400_000.0)],
            freq_ranges_tx: vec![(0.0, 38_400_000.0)],
            ..DeviceCaps::default()
        },
        &plan(),
        &[],
        &[],
    )
}

fn source(hz: f64, pa: bool, hardware: &Arc<Mutex<HardwareState>>) -> Box<dyn IqSource> {
    let plan = plan();
    Box::new(ConvertedSource::with_plan_at_dial(
        Box::new(Rig { center: hz + plan.offset_for(hz), pa, hardware: Arc::clone(hardware) }),
        plan,
        hz,
    ))
}

fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root = std::env::temp_dir().join(format!("sdroxide-band-tx-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", root) };
    });
}

fn wait(h: &EngineHandles, want: impl Fn(&RadioState) -> bool) -> RadioState {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(RadioEvent::State(s)) = h.event_rx.recv_timeout(Duration::from_millis(100))
            && want(&s)
        {
            return s;
        }
    }
    panic!("expected transmit settings were not published");
}

fn levels(s: &RadioState, drive: f32, tune: f32, pa: bool) -> bool {
    (s.tx.drive - drive).abs() < 1e-6
        && (s.tx.tune_drive - tune).abs() < 1e-6
        && s.tx.onboard_pa == Some(pa)
}

#[test]
fn band_levels_and_pa_follow_split_reopen_and_restart() {
    isolate_config();
    let store = Store::radio(901);
    let hardware = Arc::new(Mutex::new(HardwareState::default()));
    let reopened = Arc::clone(&hardware);
    let factory: ReopenFn = Box::new(move |hz| Ok((source(hz, true, &reopened), caps())));
    let mut h = start_engine(
        source(HF, true, &hardware),
        caps(),
        EngineConfig {
            initial_mode: Some(Mode::Cw),
            remember_session: true,
            store: store.clone(),
            reopen: Some(factory),
            ..Default::default()
        },
    );
    let thread = h.thread.take().unwrap();
    wait(&h, |s| levels(s, 0.1, 0.05, true));
    h.cmd_tx.send(Command::SetTxDrive(0.4)).unwrap();
    h.cmd_tx.send(Command::SetTuneDrive(0.2)).unwrap();
    h.cmd_tx.send(Command::SetOnboardPa(true)).unwrap();
    wait(&h, |s| levels(s, 0.4, 0.2, true));

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: RF6 }).unwrap();
    wait(&h, |s| s.active_freq_hz() == RF6 && levels(s, 0.1, 0.05, true));
    h.cmd_tx.send(Command::SetTxDrive(0.02)).unwrap();
    h.cmd_tx.send(Command::SetTuneDrive(0.01)).unwrap();
    h.cmd_tx.send(Command::SetOnboardPa(false)).unwrap();
    wait(&h, |s| levels(s, 0.02, 0.01, false));
    assert!(!hardware.lock().unwrap().pa);

    h.cmd_tx.send(Command::ProfileSave("transverters".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saved = false;
    while Instant::now() < deadline {
        if let Ok(RadioEvent::Profiles(names)) = h.event_rx.recv_timeout(Duration::from_millis(100))
            && names.iter().any(|n| n == "transverters")
        {
            saved = true;
            break;
        }
    }
    assert!(saved, "the profile must be saved before changing its levels");
    h.cmd_tx.send(Command::SetTxDrive(0.025)).unwrap();
    h.cmd_tx.send(Command::SetTuneDrive(0.012)).unwrap();
    h.cmd_tx.send(Command::SetOnboardPa(true)).unwrap();
    wait(&h, |s| levels(s, 0.025, 0.012, true));
    h.cmd_tx.send(Command::ProfileApply("transverters".into())).unwrap();
    wait(&h, |s| levels(s, 0.02, 0.01, false));

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: HF }).unwrap();
    wait(&h, |s| s.active_freq_hz() == HF && levels(s, 0.4, 0.2, true));
    assert!(hardware.lock().unwrap().pa);

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: RF6 }).unwrap();
    h.cmd_tx.send(Command::SetSplit(true)).unwrap();
    wait(&h, |s| s.split && levels(s, 0.02, 0.01, false));
    h.cmd_tx.send(Command::SetTuneDrive(0.015)).unwrap();
    wait(&h, |s| levels(s, 0.02, 0.015, false));
    h.cmd_tx.send(Command::SetSplit(false)).unwrap();
    wait(&h, |s| !s.split && levels(s, 0.4, 0.2, true));
    h.cmd_tx.send(Command::SelectVfo(Vfo::B)).unwrap();
    wait(&h, |s| s.active_vfo == Vfo::B && levels(s, 0.02, 0.015, false));

    h.cmd_tx.send(Command::SetTune(true)).unwrap();
    wait(&h, |s| s.tx.tune);
    h.cmd_tx.send(Command::SetOnboardPa(true)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut refused = false;
    while Instant::now() < deadline {
        if let Ok(RadioEvent::Notice(Some(n))) = h.event_rx.recv_timeout(Duration::from_millis(100))
            && n.contains("Release PTT")
        {
            refused = true;
            break;
        }
    }
    assert!(refused, "PA routing must not be changed while keyed");
    assert!(!hardware.lock().unwrap().pa);
    h.cmd_tx.send(Command::SetTune(false)).unwrap();
    wait(&h, |s| !s.tx.tune && levels(s, 0.02, 0.015, false));

    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();
    wait(&h, |s| s.active_freq_hz() == RF6 && levels(s, 0.02, 0.015, false));
    drop(h);
    thread.join().unwrap();
    let session = store.load_session();
    assert_eq!(session.band_tx[&Band::M40].drive, Some(0.4));
    assert_eq!(session.band_tx[&Band::M6].tune_drive, Some(0.015));
    assert_eq!(session.band_tx[&Band::M6].onboard_pa, Some(false));

    let reopened = Arc::clone(&hardware);
    let factory: ReopenFn = Box::new(move |hz| Ok((source(hz, false, &reopened), caps())));
    let mut h = start_engine(
        source(RF6, true, &hardware),
        caps(),
        EngineConfig {
            initial_mode: Some(Mode::Cw),
            remember_session: true,
            store,
            reopen: Some(factory),
            ..Default::default()
        },
    );
    let thread = h.thread.take().unwrap();
    wait(&h, |s| levels(s, 0.02, 0.015, false));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: RF2 }).unwrap();
    wait(&h, |s| s.active_freq_hz() == RF2 && levels(s, 0.1, 0.05, true));
    // The level shown stays 10%, but the transverter's hard ceiling still wins.
    assert!((hardware.lock().unwrap().drive - 0.02).abs() < 1e-6);
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();
    wait(&h, |s| levels(s, 0.1, 0.05, false));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: HF }).unwrap();
    wait(&h, |s| s.active_freq_hz() == HF && levels(s, 0.4, 0.2, true));
    // A tune and PTT in the same command batch must restore PA before tx_begin.
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: RF6 }).unwrap();
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    wait(&h, |s| s.tx.ptt && levels(s, 0.02, 0.015, false));
    assert_eq!(hardware.lock().unwrap().keyed, Some((28_174_000.0, false)));
    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    drop(h);
    thread.join().unwrap();
}

#[test]
fn old_session_levels_migrate_only_to_the_starting_band() {
    isolate_config();
    let store = Store::radio(902);
    store
        .save_session(&Session {
            freq_hz: RF6,
            drive: 0.07,
            tune_drive: 0.025,
            ..Session::default()
        })
        .unwrap();
    let hardware = Arc::new(Mutex::new(HardwareState::default()));
    let mut h = start_engine(
        source(RF6, false, &hardware),
        caps(),
        EngineConfig { remember_session: true, store, ..Default::default() },
    );
    let thread = h.thread.take().unwrap();
    wait(&h, |s| levels(s, 0.07, 0.025, false));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: HF }).unwrap();
    wait(&h, |s| s.active_freq_hz() == HF && levels(s, 0.1, 0.05, false));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: RF6 }).unwrap();
    wait(&h, |s| s.active_freq_hz() == RF6 && levels(s, 0.07, 0.025, false));
    drop(h);
    thread.join().unwrap();
}
