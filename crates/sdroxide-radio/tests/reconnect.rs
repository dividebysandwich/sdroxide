//! Automatic reconnect: a radio that isn't there when the engine starts — a TCI
//! server still coming up, sdroxide launched before ExpertSDR3 — must attach by
//! itself, without the operator opening Settings → Radio and pressing
//! "Apply / reconnect".

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, EngineSwap, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RadioState, RxId, Vfo};

const RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;

/// The placeholder the binary installs when the configured interface can't be
/// opened: no samples, and it asks to be reopened.
struct Offline;

impl IqSource for Offline {
    fn sample_rate(&self) -> f64 {
        RATE
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
    fn open_status(&self) -> Option<String> {
        Some("radio interface unavailable".into())
    }
    fn needs_reopen(&self) -> bool {
        true
    }
}

/// The rig, once it answers.
struct Online;

impl IqSource for Online {
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
        "live rig".into()
    }
}

/// A dongle: it holds its device exclusively, so it has to be stood down
/// *before* its replacement is built. `claimed` stands in for the kernel's
/// claim on the USB interface — the factory below refuses to open anything
/// while it is still held, exactly as a second `claim_interface` is refused.
struct Exclusive {
    claimed: Arc<AtomicBool>,
}

impl IqSource for Exclusive {
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
        "dongle".into()
    }
    fn release(&mut self) {
        self.claimed.store(false, Ordering::SeqCst);
    }
}

fn caps(driver: &str) -> DeviceCaps {
    DeviceCaps {
        driver: driver.into(),
        label: driver.into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

#[test]
fn offline_interface_attaches_by_itself() {
    // The interface refuses the first attempt (the rig is still starting) and
    // answers the next one.
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&attempts);
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        if seen.fetch_add(1, Ordering::Relaxed) == 0 {
            return Err("connection refused".into());
        }
        Ok((Box::new(Online) as Box<dyn IqSource>, caps("live")))
    });

    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Offline), caps("offline"), cfg);
    let thread = h.thread.take();

    // First attempt after ~1 s, the successful one ~2 s later.
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut connected = false;
    while !connected && Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Capabilities(c) = ev {
                connected |= c.driver == "live";
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(connected, "the engine should have reconnected on its own");

    // And having connected, it stops trying: the live source doesn't ask to be
    // reopened, so no further attempt may run.
    let after_connect = attempts.load(Ordering::Relaxed);
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(
        attempts.load(Ordering::Relaxed),
        after_connect,
        "a connected interface must not be reopened again"
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Applying a settings change on a running dongle must not fail against the
/// dongle itself. The outgoing source is released before the factory runs, so
/// the replacement finds the device free rather than "held by another program".
#[test]
fn a_settings_change_releases_the_running_device_first() {
    let claimed = Arc::new(AtomicBool::new(true));
    let held = Arc::clone(&claimed);
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        if held.load(Ordering::SeqCst) {
            return Err("the device is held by another program".into());
        }
        Ok((Box::new(Online) as Box<dyn IqSource>, caps("live")))
    });

    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let source = Exclusive { claimed: Arc::clone(&claimed) };
    let mut h = start_engine(Box::new(source), caps("dongle"), cfg);
    let thread = h.thread.take();

    // Settings → Radio → Apply.
    h.swap_tx.send(EngineSwap::ReopenSource).expect("engine is running");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut swapped = false;
    let mut failure = None;
    while !swapped && Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            match ev {
                RadioEvent::Capabilities(c) => swapped |= c.driver == "live",
                RadioEvent::Notice(Some(n)) if n.contains("failed") => failure = Some(n),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(failure, None, "the interface change reported an error");
    assert!(swapped, "the engine should have swapped to the new interface");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// An interface that never comes up must not turn into a retry storm — the
/// spacing backs off instead.
#[test]
fn failing_interface_backs_off() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&attempts);
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        seen.fetch_add(1, Ordering::Relaxed);
        Err("no such device".into())
    });

    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Offline), caps("offline"), cfg);
    let thread = h.thread.take();

    // 1 s + 2 s + 4 s: three attempts fit in five seconds, an unthrottled loop
    // would be far past that.
    std::thread::sleep(Duration::from_secs(5));
    let n = attempts.load(Ordering::Relaxed);
    assert!((1..=4).contains(&n), "expected a handful of backed-off attempts, got {n}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A source that reports whatever rate it was built with, standing in for an
/// RX-888 whose ADC clock the operator has just changed.
struct AtRate(f64);

impl IqSource for AtRate {
    fn sample_rate(&self) -> f64 {
        self.0
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
        format!("source at {:.0} Hz", self.0)
    }
}

/// Changing the sample rate must take effect on a reopen, without restarting.
///
/// This is the path a setting like the RX-888's ADC clock takes: persist the
/// config, ask the engine to reopen, and let the factory build a source with a
/// *different* rate from the one the engine is currently running. The engine has
/// to rebuild everything it sized against the old rate rather than carry it
/// over — a stale rate is not a cosmetic problem, it puts every later frequency
/// calculation out by the ratio between the two.
#[test]
fn a_changed_sample_rate_takes_effect_on_reopen() {
    const NEW_RATE: f64 = 96_000.0;

    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        let mut c = caps("rerated");
        c.sample_rates = vec![NEW_RATE];
        Ok((Box::new(AtRate(NEW_RATE)) as Box<dyn IqSource>, c))
    });

    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(AtRate(RATE)), caps("initial"), cfg);
    let thread = h.thread.take();

    // Let it settle at the original rate, then ask for the swap.
    std::thread::sleep(Duration::from_millis(200));
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut rerated = false;
    let mut state_rate = 0.0f64;
    while !rerated && Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            match ev {
                RadioEvent::Capabilities(c) if c.driver == "rerated" => rerated = true,
                RadioEvent::State(s) => state_rate = s.sample_rate,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(rerated, "the engine never adopted the re-rated source");

    // The state the UI draws from must describe the new rate, not the old one.
    let deadline = Instant::now() + Duration::from_secs(3);
    while state_rate != NEW_RATE && Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev {
                state_rate = s.sample_rate;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(state_rate, NEW_RATE, "the engine kept reporting the old sample rate");

    drop(h.cmd_tx);
    drop(h.swap_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A source that reports whatever frequency it was last asked for, so a reopen
/// puts the replacement exactly where the operator was — what real hardware
/// covering the band does.
struct Tunable {
    center: f64,
}

impl IqSource for Tunable {
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
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "tunable".into()
    }
}

/// Drain events for the whole of `dur`, returning whether `want_driver` was
/// announced and the **last** state snapshot seen.
///
/// Both come out of one drain because the swap publishes its capabilities and
/// its state together, and a loop watching for only one would throw the other
/// away. It runs the window out rather than stopping at the first snapshot: the
/// interesting state is the one the engine settles on, not the one it happened
/// to publish first.
fn settle(
    h: &sdroxide_radio::EngineHandles,
    want_driver: &str,
    dur: Duration,
) -> (bool, Option<RadioState>) {
    let deadline = Instant::now() + dur;
    let (mut seen, mut state) = (false, None);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            match ev {
                RadioEvent::Capabilities(c) => seen |= c.driver == want_driver,
                RadioEvent::State(s) => state = Some(s),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    (seen, state)
}

/// Settings → Radio → **Apply / reconnect** swaps the front end. It must not
/// touch the operating position: the mode, filters, levels, RIT and split
/// belong to the operator, not to the device.
///
/// The whole radio used to reset to `RadioState::default()` here — 20 m USB —
/// on any Apply, including one that changed nothing, and on the *automatic*
/// reconnect after a dropped link too.
#[test]
fn applying_a_settings_change_keeps_the_operating_position() {
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |center: f64| {
        Ok((Box::new(Tunable { center }) as Box<dyn IqSource>, caps("live")))
    });
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Tunable { center: CENTER }), caps("dongle"), cfg);
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(150));

    // An operating position: 40 m LSB, a narrowed filter, RIT on, split to a
    // second VFO, a raised drive and a set audio level.
    let on_air = 7_100_000.0;
    let split_to = 7_150_000.0;
    for c in [
        Command::SetVfo { vfo: Vfo::B, hz: split_to },
        Command::SetVfo { vfo: Vfo::A, hz: on_air },
        Command::SetMode { rx: RxId::Main, mode: Mode::Lsb },
        Command::SetFilter { rx: RxId::Main, lo: -2_100.0, hi: -300.0 },
        Command::SetRit { enabled: true, hz: 250 },
        Command::SetSplit(true),
        Command::SetTxDrive(0.4),
        Command::SetVolume { rx: RxId::Main, v: 0.8 },
    ] {
        h.cmd_tx.send(c).expect("engine is running");
    }
    let (_, before) = settle(&h, "dongle", Duration::from_millis(500));
    let before = before.expect("the engine publishes state");
    assert_eq!(before.rx[0].mode, Mode::Lsb, "the test set up LSB");

    // Apply / reconnect, changing nothing else.
    h.swap_tx.send(EngineSwap::ReopenSource).expect("engine is running");
    let (swapped, after) = settle(&h, "live", Duration::from_secs(2));
    assert!(swapped, "the engine should have swapped to the new interface");
    let after = after.expect("the swap publishes state");

    assert_eq!(after.rx[0].mode, Mode::Lsb, "the mode is the operator's, not the device's");
    assert_eq!((after.rx[0].filter_lo, after.rx[0].filter_hi), (-2_100.0, -300.0), "filter");
    assert_eq!(after.vfo_a_hz, on_air, "still on the same frequency");
    assert_eq!(after.vfo_b_hz, split_to, "the split VFO survives too");
    assert!(after.split, "split");
    assert_eq!(after.rit, before.rit, "RIT");
    assert_eq!(after.tx.drive, 0.4, "transmit drive");
    assert_eq!(after.rx[0].volume, 0.8, "audio level");
    // And nothing is left keyed by the swap.
    assert!(!after.tx.ptt && !after.tx.tune, "a swap must leave the transmitter down");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// An Icom over LAN whose first frequency reply came after the connect wait
/// reports 0 Hz: it has not heard its dial yet. Taken as the radio's answer,
/// that put both VFOs on 0, clamped them to the bottom of the receive range,
/// band GEN, and tuned the radio there (#576). The dial stays the operator's.
#[test]
fn a_source_that_has_not_heard_its_dial_keeps_the_operators() {
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        let mut c = caps("unheard");
        c.freq_ranges_rx = vec![(30_000.0, 60_000_000.0)];
        Ok((Box::new(Tunable { center: 0.0 }) as Box<dyn IqSource>, c))
    });
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Tunable { center: CENTER }), caps("dongle"), cfg);
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(150));

    let on_air = 14_080_000.0;
    let other = 7_040_000.0;
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: other }).unwrap();
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: on_air }).unwrap();
    let _ = settle(&h, "dongle", Duration::from_millis(300));

    h.swap_tx.send(EngineSwap::ReopenSource).expect("engine is running");
    let (swapped, after) = settle(&h, "unheard", Duration::from_secs(2));
    assert!(swapped, "the engine should have swapped to the new interface");
    let after = after.expect("the swap publishes state");
    assert_eq!(after.vfo_a_hz, on_air, "VFO A went to the radio's unknown 0 Hz");
    assert_eq!(after.vfo_b_hz, other, "VFO B went to the radio's unknown 0 Hz");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The stand-in for a radio the operator has switched off: no samples, and —
/// unlike [`Offline`] — it does not ask to be reopened. A radio that is off is
/// not one waiting to come back.
struct SwitchedOff;

impl IqSource for SwitchedOff {
    fn sample_rate(&self) -> f64 {
        RATE
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
        "switched off".into()
    }
    fn open_status(&self) -> Option<String> {
        Some("this radio is switched off".into())
    }
    fn needs_reopen(&self) -> bool {
        false
    }
}

/// The rig, with a witness for having been stood down again.
struct Tracked {
    released: Arc<AtomicBool>,
}

impl IqSource for Tracked {
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
        "live rig".into()
    }
    fn release(&mut self) {
        self.released.store(true, Ordering::SeqCst);
    }
}

/// Switching a radio off while it is reconnecting must switch it off.
///
/// The engine's background reconnect can be halfway through opening a rig when
/// the operator throws the power switch — which is exactly when they throw it,
/// a radio that has just dropped its link being the one you reach for. The
/// attempt then finishes and answers with a live interface for a radio nobody
/// wants opened any more: adopting it would claim the device again, put the
/// LAN session back up and leave the switch reading OFF over a radio that is
/// on. The operator's own change is the later word and must be the last one.
#[test]
fn switching_off_mid_reconnect_is_not_undone_by_the_attempt_in_flight() {
    // Held by the factory's first (background) call until the test lets go, so
    // the switch is thrown with an attempt genuinely in flight.
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let in_flight = Arc::new(AtomicBool::new(false));
    let entered = Arc::clone(&in_flight);
    let switched_off = Arc::new(AtomicBool::new(false));
    let is_off = Arc::clone(&switched_off);
    let released = Arc::new(AtomicBool::new(false));
    let witness = Arc::clone(&released);

    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        // What the binary's factory does: the roster is the authority, and a
        // radio that is off is answered with the quiet stand-in rather than a
        // refusal.
        if is_off.load(Ordering::SeqCst) {
            return Ok((Box::new(SwitchedOff) as Box<dyn IqSource>, caps("off")));
        }
        entered.store(true, Ordering::SeqCst);
        // A network rig that takes its time to answer.
        let _ = release_rx.recv();
        Ok((
            Box::new(Tracked { released: Arc::clone(&witness) }) as Box<dyn IqSource>,
            caps("live"),
        ))
    });

    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Offline), caps("offline"), cfg);
    let thread = h.thread.take();

    // Wait for the reconnect worker to be inside the open (~1 s in).
    let deadline = Instant::now() + Duration::from_secs(5);
    while !in_flight.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(in_flight.load(Ordering::SeqCst), "the reconnect attempt never started");

    // The power switch: the roster is written, then the engine is asked to
    // build its front end again from it.
    switched_off.store(true, Ordering::SeqCst);
    h.swap_tx.send(EngineSwap::ReopenSource).expect("engine is running");
    // ...and only now does the attempt already in flight answer.
    std::thread::sleep(Duration::from_millis(100));
    let _ = release_tx.send(());

    // The engine settles on the stand-in, and stays there: the late answer is
    // let go of rather than adopted on top of the operator's change. Long
    // enough to cover both the swap and the first reconnect tick that would
    // have collected the attempt (~1 s).
    let until = Instant::now() + Duration::from_secs(3);
    let mut driver = String::new();
    while Instant::now() < until {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Capabilities(c) = ev {
                driver = c.driver;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(driver, "off", "a radio switched off must stay off");
    assert!(released.load(Ordering::SeqCst), "the abandoned attempt's device was never let go");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
