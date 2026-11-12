//! Per-mode settings through the whole engine.
//!
//! The profile vocabulary is held to its own arithmetic in `sdroxide-types`.
//! What only the engine can show is the three things the feature promises:
//! that choosing a mode lays its defaults on the receiver, that what the
//! operator changes while a mode is selected is remembered against *that* mode
//! alone, and that the reset puts the mode's own values back and forgets the
//! overrides for good. The last one is also checked across a restart, because
//! "remembered" that does not survive a launch is not remembered.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{
    AudioParams, Complex32, ControlUpdate, EngineConfig, EngineHandles, IqSource, RadioError,
    Result, rtrb, start_engine,
};
use sdroxide_types::{AgcMode, Command, DeviceCaps, Mode, NrLevel, RadioEvent, RadioState, RxId};

const RATE: f64 = 48_000.0;
const DIAL: f64 = 14_074_000.0;

/// A front end with nothing on it. The settings are the whole subject, so the
/// samples only have to keep the loop turning.
struct Quiet;

impl IqSource for Quiet {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        DIAL
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(480);
        for z in buf[..n].iter_mut() {
            *z = Complex32::default();
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "quiet mock".into()
    }
}

/// [`Quiet`] with a radio's own controls: what is pushed onto `knob` is
/// reported to the engine the way a CAT rig reports its front panel.
struct RigKnob {
    knob: Arc<Mutex<Vec<ControlUpdate>>>,
}

impl IqSource for RigKnob {
    fn sample_rate(&self) -> f64 {
        Quiet.sample_rate()
    }
    fn center_hz(&self) -> f64 {
        Quiet.center_hz()
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        Quiet.read(buf)
    }
    fn describe(&self) -> String {
        "rig knob mock".into()
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        std::mem::take(&mut *self.knob.lock().unwrap())
    }
}

/// [`Quiet`] until `unplug` is set, and then a front end whose link is gone:
/// every read fails, which takes the engine down the way a pulled USB cable
/// does.
struct Unpluggable {
    unplug: Arc<std::sync::atomic::AtomicBool>,
}

impl IqSource for Unpluggable {
    fn sample_rate(&self) -> f64 {
        Quiet.sample_rate()
    }
    fn center_hz(&self) -> f64 {
        Quiet.center_hz()
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        if self.unplug.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(RadioError::Msg("the device went away".into()));
        }
        Quiet.read(buf)
    }
    fn describe(&self) -> String {
        "unpluggable mock".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 30_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// A config directory of this test's own, so the profiles file it writes is not
/// the operator's and no session is picked up from one.
///
/// `SDROXIDE_CONFIG_DIR` is process-global, so the tests in this file take
/// [`CONFIG_LOCK`] around it: run in parallel they would each point the process
/// at a different directory and the engines would write over one another.
static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn isolate(name: &str) {
    let root = std::env::temp_dir().join(format!("sdroxide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
}

fn start(mode: Mode) -> EngineHandles {
    start_on(Box::new(Quiet), mode)
}

fn start_on(source: Box<dyn IqSource>, mode: Mode) -> EngineHandles {
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    start_engine(
        source,
        caps(),
        EngineConfig {
            audio: Some(AudioParams { producer, out_rate: RATE }),
            initial_mode: Some(mode),
            // The profiles are read and written by an engine that remembers,
            // which is also the only kind that will find the file this test
            // writes. The config directory above keeps it out of the operator's.
            remember_session: true,
            ..Default::default()
        },
    )
}

fn stop(mut h: EngineHandles) {
    let thread = h.thread.take();
    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Wait for a state that satisfies `f`, or say what the last one was.
fn wait_for(h: &EngineHandles, what: &str, f: impl Fn(&RadioState) -> bool) -> RadioState {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last: Option<RadioState> = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev {
                if f(&s) {
                    return s;
                }
                last = Some(s);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "the state never showed {what}; last: {:?}",
        last.map(|s| (s.rx[0].mode, s.rx[0].noise_reduction, s.rx[0].agc))
    );
}

fn send(h: &EngineHandles, c: Command) {
    h.cmd_tx.send(c).unwrap();
}

/// Choosing a mode lays its own settings on the receiver: an SSB voice mode
/// keeps the stock AGC, a weak-signal digital mode gets a slow one. The noise
/// reduction is off in both, because it is never defaulted on.
#[test]
fn a_mode_change_applies_that_modes_defaults() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-apply");
    let h = start(Mode::Usb);

    let s = wait_for(&h, "USB's defaults", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].agc == AgcMode::Med
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::Off);

    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 });
    let s = wait_for(&h, "FT8's defaults", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].agc == AgcMode::Slow
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::Off);

    stop(h);
}

/// What the operator changes is remembered against the mode it was changed in,
/// comes back with that mode, and is forgotten when the mode is reset.
#[test]
fn the_operators_change_is_kept_per_mode_and_reset() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-remember");
    let h = start(Mode::Ft8);

    // Turn the noise reduction up in FT8. It ships off there, so this is a
    // departure and worth remembering.
    send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::High });
    let _ = wait_for(&h, "FT8's changed NR", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::High
    });

    // Usb is not FT8: it comes up with its own default, not the FT8 value.
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    let s = wait_for(&h, "Usb's own default", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction != NrLevel::High
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::Off);

    // ...and FT8 remembers.
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 });
    let s = wait_for(&h, "FT8's remembered NR", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::High
    });
    assert_eq!(s.rx[0].agc, AgcMode::Slow, "the untouched fields are still the mode's");

    // Reset: the mode's own values return...
    send(&h, Command::ResetModeDefaults { mode: Some(Mode::Ft8) });
    let s = wait_for(&h, "FT8 back on its defaults", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::Off
    });
    assert_eq!(s.rx[0].agc, AgcMode::Slow);

    // ...and the override is gone, not merely hidden: leaving and returning is
    // still the default.
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    let _ = wait_for(&h, "Usb", |s| s.rx[0].mode == Mode::Usb);
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 });
    let s = wait_for(&h, "FT8 with nothing remembered", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::Off
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::Off);

    stop(h);
}

/// A mode chosen on the radio's own controls gets that mode's settings, the
/// same as one chosen here.
#[test]
fn a_mode_changed_on_the_rig_gets_that_modes_settings() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-rig-knob");
    let knob = Arc::new(Mutex::new(Vec::new()));
    let h = start_on(Box::new(RigKnob { knob: knob.clone() }), Mode::Usb);

    // LSB remembers its noise reduction up; USB leaves it off. Waited for in
    // turn, so the USB state below is the one after the change back and not the
    // one the engine started with.
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Lsb });
    send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::High });
    let _ = wait_for(&h, "LSB's NR up", |s| {
        s.rx[0].mode == Mode::Lsb && s.rx[0].noise_reduction == NrLevel::High
    });
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    let _ = wait_for(&h, "USB with its NR off", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::Off
    });

    // The operator turns the rig's mode knob to LSB.
    knob.lock().unwrap().push(ControlUpdate::Mode(Mode::Lsb));
    let s = wait_for(&h, "LSB from the rig, with LSB's NR", |s| {
        s.rx[0].mode == Mode::Lsb && s.rx[0].noise_reduction == NrLevel::High
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::High);
    stop(h);
}

/// A reset with no mode forgets every mode's overrides.
#[test]
fn resetting_every_mode_puts_all_of_them_back() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-reset-all");
    let h = start(Mode::Ft8);

    send(&h, Command::SetSquelch { rx: RxId::Main, db: -80.0 });
    let _ = wait_for(&h, "FT8's squelch", |s| (s.rx[0].squelch_db + 80.0).abs() < 0.01);
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    send(&h, Command::SetSquelch { rx: RxId::Main, db: -70.0 });
    let _ = wait_for(&h, "Usb's squelch", |s| {
        s.rx[0].mode == Mode::Usb && (s.rx[0].squelch_db + 70.0).abs() < 0.01
    });

    send(&h, Command::ResetModeDefaults { mode: None });
    let s = wait_for(&h, "Usb open again", |s| {
        s.rx[0].mode == Mode::Usb
            && (s.rx[0].squelch_db - sdroxide_types::SQUELCH_OPEN_DB).abs() < 0.01
    });
    assert!((s.rx[0].squelch_db - sdroxide_types::SQUELCH_OPEN_DB).abs() < 0.01);

    stop(h);
}

/// A restored session is not undone at startup by the mode's own defaults.
///
/// The upgrade case, and the reason this was a bug: a build that has per-mode
/// settings arrives with an empty `modeprofiles.json`, but the operator's
/// `session.json` carries real settings. If startup lays the mode's defaults
/// on the receiver *after* restoring the session (as the first version did),
/// every saved AGC, squelch, NR, binaural and RX gain is silently reset on the
/// first launch. The session is the operator's last word on the mode it was
/// left in, and is recorded as that mode's own values before the profile is
/// laid on.
#[test]
fn a_restored_session_is_not_reset_by_the_modes_defaults() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-session");

    // A session carrying a departure — FT8 ships NR off. This also writes an
    // override; removing that file afterwards leaves exactly the state an
    // upgrade starts from: a real session and no overrides.
    {
        let h = start(Mode::Ft8);
        send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::High });
        let _ = wait_for(&h, "FT8's changed NR", |s| {
            s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::High
        });
        stop(h);
    }
    let dir = std::env::var("SDROXIDE_CONFIG_DIR").unwrap();
    let _ = std::fs::remove_file(std::path::Path::new(&dir).join("modeprofiles.json"));

    let h = start(Mode::Ft8);
    let s = wait_for(&h, "FT8's restored session", |s| {
        s.rx[0].mode == Mode::Ft8 && s.rx[0].noise_reduction == NrLevel::High
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::High, "the restored session must stand");
    stop(h);
}

/// The upgrade case one step on: the restored session's levels are recorded as
/// its mode's own, so they survive leaving the mode and coming back.
///
/// Standing on the receiver alone they would not. The mode being returned to
/// is laid out from `modeprofiles.json`, and on the first start after an
/// upgrade that is empty — so without the session being recorded, the
/// operator's settings would last exactly until the first mode change.
#[test]
fn a_restored_sessions_levels_become_its_modes_own() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-upgrade");
    {
        let h = start(Mode::Usb);
        send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::High });
        let _ = wait_for(&h, "USB's changed NR", |s| {
            s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::High
        });
        stop(h);
    }
    let dir = std::env::var("SDROXIDE_CONFIG_DIR").unwrap();
    let _ = std::fs::remove_file(std::path::Path::new(&dir).join("modeprofiles.json"));

    let h = start(Mode::Usb);
    let _ = wait_for(&h, "USB's restored session", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::High
    });
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Lsb });
    let s = wait_for(&h, "LSB", |s| s.rx[0].mode == Mode::Lsb);
    assert_eq!(s.rx[0].noise_reduction, NrLevel::Off, "LSB has its own settings");
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    let s = wait_for(&h, "USB with its NR back", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::High
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::High);
    stop(h);
}

/// A working setup put back on sets the receiver's levels, and those become the
/// mode's own: leaving the mode and coming back keeps them rather than returning
/// to what the mode had before the setup was applied.
#[test]
fn a_working_setups_levels_become_its_modes_own() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-working-setup");
    let h = start(Mode::Usb);

    // Saved with the noise reduction up, then turned back off in USB.
    send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::High });
    send(&h, Command::ProfileSave("nr".into()));
    send(&h, Command::SetNoiseReduction { rx: RxId::Main, level: NrLevel::Off });
    let _ = wait_for(&h, "USB's NR off again", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::Off
    });

    send(&h, Command::ProfileApply("nr".into()));
    let _ = wait_for(&h, "the setup's NR", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::High
    });
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Lsb });
    let _ = wait_for(&h, "LSB", |s| s.rx[0].mode == Mode::Lsb);
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Usb });
    let s = wait_for(&h, "USB with the setup's NR", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].noise_reduction == NrLevel::High
    });
    assert_eq!(s.rx[0].noise_reduction, NrLevel::High);
    stop(h);
}

/// A change made just before the front end drops its connection is still on
/// disk afterwards. The engine stops on a lost link without waiting for the
/// session tick, so this is the path that shows whether the file is written on
/// every way out or only a clean one.
#[test]
fn a_change_survives_the_front_end_dropping_its_connection() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-unplugged");
    {
        let unplug = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let h = start_on(Box::new(Unpluggable { unplug: unplug.clone() }), Mode::Usb);
        send(&h, Command::SetAutoNotch { rx: RxId::Main, on: true });
        let _ = wait_for(&h, "USB's notch on", |s| s.rx[0].auto_notch);
        unplug.store(true, std::sync::atomic::Ordering::Relaxed);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !h.thread.as_ref().is_some_and(|t| t.is_finished()) {
            assert!(Instant::now() < deadline, "the engine never noticed the link was gone");
            std::thread::sleep(Duration::from_millis(10));
        }
        stop(h);
    }

    let h = start(Mode::Usb);
    let s =
        wait_for(&h, "USB's remembered notch", |s| s.rx[0].mode == Mode::Usb && s.rx[0].auto_notch);
    assert!(s.rx[0].auto_notch);
    stop(h);
}

/// Remembered means it survives a launch: a fresh engine in the same mode reads
/// the file and applies the override with no command having been sent.
///
/// Auto-notch and AGC max gain because `session.json` carries neither, so
/// nothing but `modeprofiles.json` can bring them back — a setting the session
/// also restores would pass this whether the profile was applied or not.
#[test]
fn the_overrides_survive_a_restart() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-restart");
    {
        let h = start(Mode::Wspr);
        send(&h, Command::SetAutoNotch { rx: RxId::Main, on: true });
        send(&h, Command::SetAgcMaxGain { rx: RxId::Main, db: 60.0 });
        let _ = wait_for(&h, "WSPR's notch and max gain", |s| {
            s.rx[0].auto_notch && (s.rx[0].agc_max_gain_db - 60.0).abs() < 0.01
        });
        // Written when the engine stops, if the session tick has not got to it
        // first.
        stop(h);
    }

    let h = start(Mode::Wspr);
    let s = wait_for(&h, "WSPR's remembered notch and max gain", |s| {
        s.rx[0].mode == Mode::Wspr
            && s.rx[0].auto_notch
            && (s.rx[0].agc_max_gain_db - 60.0).abs() < 0.01
    });
    assert_eq!(s.rx[0].agc, AgcMode::Slow, "the mode's own defaults are still under it");
    stop(h);
}

/// Proves: the NR menu's filters, HUM and the SK post-filter with its time
/// constant, width and place, are remembered per mode and come back after a
/// restart, like the NR engine beside them. Before, both fell back to off.
// Why: HDZ 2026-11-12: "die gesamte Konfig fehlt" — the SK filter was off
//      again after every start.
#[test]
fn the_nr_filters_survive_a_restart() {
    let _guard = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    isolate("modeprofiles-nr-filters");
    {
        let h = start(Mode::Usb);
        send(&h, Command::SetHumFilter { rx: RxId::Main, on: true });
        send(
            &h,
            Command::SetSkFilter {
                rx: RxId::Main,
                on: true,
                tau_s: 2.0,
                open_s: f32::NAN,
                close_s: f32::NAN,
                width: 3,
                after_nr: false,
            },
        );
        let _ = wait_for(&h, "USB's HUM and SK on", |s| {
            s.rx[0].hum_filter && s.rx[0].sk_filter && s.rx[0].sk_filter_width == 3
        });
        stop(h);
    }

    let h = start(Mode::Usb);
    let s = wait_for(&h, "USB's remembered HUM and SK", |s| {
        s.rx[0].mode == Mode::Usb && s.rx[0].hum_filter && s.rx[0].sk_filter
    });
    assert!((s.rx[0].sk_filter_tau_s - 2.0).abs() < 1e-6, "the time constant comes back");
    assert_eq!(s.rx[0].sk_filter_width, 3, "the width comes back");
    assert!(!s.rx[0].sk_filter_after_nr, "the place before NR comes back");
    stop(h);

    // And another mode keeps its own: LSB never had them switched on.
    let h = start(Mode::Lsb);
    let s = wait_for(&h, "LSB on its defaults", |s| s.rx[0].mode == Mode::Lsb);
    assert!(!s.rx[0].hum_filter && !s.rx[0].sk_filter, "per mode, not global");
    stop(h);
}
