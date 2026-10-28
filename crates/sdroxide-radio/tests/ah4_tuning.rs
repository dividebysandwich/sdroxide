
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use sdroxide_radio::{
    Complex32, ControlUpdate, EngineConfig, IqSource, Result, start_engine,
};
use sdroxide_types::{Command, DeviceCaps, RadioEvent};

const RATE: f64 = 600_000.0;
const CENTER_HZ: f64 = 14.1e6;

struct MockAh4Source {
    center: f64,
    updates: Arc<Mutex<VecDeque<ControlUpdate>>>,
    status_fresh: Arc<AtomicBool>,
    tuner_starts: Arc<AtomicUsize>,
    tuner_bypasses: Arc<AtomicUsize>,
    tuner_recovers: Arc<AtomicUsize>,
    keyed: Arc<AtomicBool>,
    tx_ends: Arc<AtomicUsize>,
}

impl IqSource for MockAh4Source {
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
        thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }

    fn describe(&self) -> String {
        "AH-4 test source".into()
    }

    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        self.keyed.store(true, Ordering::SeqCst);
        Ok(rate)
    }

    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }

    fn tx_end(&mut self) -> Result<()> {
        self.keyed.store(false, Ordering::SeqCst);
        self.tx_ends.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn supports_ah4_tuner(&self) -> bool {
        true
    }

    fn ah4_status_fresh(&self) -> bool {
        self.status_fresh.load(Ordering::SeqCst)
    }

    fn tuner_start(&mut self) -> Result<()> {
        self.tuner_starts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn tuner_bypass(&mut self) -> Result<()> {
        self.tuner_bypasses.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn tuner_recover(&mut self) -> Result<()> {
        self.tuner_recovers.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.updates.lock().unwrap().drain(..).collect()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock-ah4".into(),
        label: "mock-ah4".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1.0e6, 60.0e6)],
        freq_ranges_tx: vec![(1.0e6, 60.0e6)],
        ..DeviceCaps::default()
    }
}

fn wait_until(
    description: &str,
    timeout: Duration,
    mut condition: impl FnMut() -> bool,
) {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(condition(), "timed out waiting for {description}");
}

#[test]
fn ah4_tune_waits_for_fresh_rf_request_before_keying() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // With tuner presence still unknown, AH-4 tuning must be refused.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("presence guard notice", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::Notice(Some(message))
                if message.contains("not been positively detected")),
        )
    });
    assert_eq!(
        tuner_starts.load(Ordering::SeqCst),
        0,
        "AH-4 tuning must not start when tuner presence is unknown"
    );

    // A confirmed absence must also prevent AH-4 tuning.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(false)));

    wait_until("tuner absence event", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(false))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("absence guard notice", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::Notice(Some(message))
                if message.contains("not been positively detected")),
        )
    });

    assert_eq!(
        tuner_starts.load(Ordering::SeqCst),
        0,
        "AH-4 tuning must not start when the tuner is absent"
    );

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    // The start request itself must never key the transmitter.
    assert!(
        !keyed.load(Ordering::SeqCst),
        "transmitter keyed before the tuner requested RF"
    );

    // An explicit, fresh tuner request is the only event that should
    // authorize the tuning carrier.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    wait_until("RF authorization", Duration::from_secs(2), || {
        keyed.load(Ordering::SeqCst)
    });

    // An intermediate tuner status must withdraw RF permission and unkey,
    // while remembering that the tuner did request RF during this cycle.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x07)));

    wait_until("intermediate status unkeys transmitter", Duration::from_secs(2), || {
        !keyed.load(Ordering::SeqCst)
    });

    // Returning to idle after the intermediate status must still count as
    // a completed tune because the tuner previously requested RF.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("tune completion notice", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::Notice(Some(message))
                if message.contains("AH-4 tuning complete")),
        )
    });

    wait_until("transmitter unkey", Duration::from_secs(2), || {
        !keyed.load(Ordering::SeqCst)
    });

    // A confirmed tune makes the next idle click request bypass.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();
    wait_until("bypass request", Duration::from_secs(2), || {
        tuner_bypasses.load(Ordering::SeqCst) == 1
    });

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("bypass confirmation", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::Notice(Some(message))
                if message.contains("AH-4 bypass confirmed")),
        )
    });

    // After confirmed bypass, the next click starts tuning again.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();
    wait_until("second tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 2
    });

    // A duplicate click during tuning must not abort the active cycle
    // or send a bypass request.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("duplicate Tune command notice", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::Notice(Some(message))
                if message.contains("AH-4 tuning is already in progress")),
        )
    });

    // The duplicate command must not start another tuner operation.
    assert_eq!(
        tuner_starts.load(Ordering::SeqCst),
        2,
        "duplicate Tune command started another tuning cycle"
    );
    assert_eq!(
        tuner_bypasses.load(Ordering::SeqCst),
        1,
        "duplicate Tune command unexpectedly requested bypass"
    );

    // The pending cycle must still accept a fresh RF request.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    wait_until("RF authorization after duplicate command", Duration::from_secs(2), || {
        keyed.load(Ordering::SeqCst)
    });
    assert_eq!(
        tuner_bypasses.load(Ordering::SeqCst),
        1,
        "aborting an active tune must not issue a bypass request"
    );

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }

    assert_eq!(tuner_starts.load(Ordering::SeqCst), 2);
    assert_eq!(tuner_bypasses.load(Ordering::SeqCst), 1);
    assert!(
        tx_ends.load(Ordering::SeqCst) >= 1,
        "the engine should call tx_end when tuning RF stops"
    );
}

#[test]
fn ah4_tuning_unkeys_when_status_loses_freshness() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    wait_until("RF authorization", Duration::from_secs(2), || {
        keyed.load(Ordering::SeqCst)
    });

    // Simulate the status becoming stale or unavailable without relying
    // on another status event to trigger the safety action.
    status_fresh.store(false, Ordering::SeqCst);

    wait_until("unkey after freshness loss", Duration::from_secs(2), || {
        !keyed.load(Ordering::SeqCst)
    });

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }

    assert!(
        tx_ends.load(Ordering::SeqCst) >= 1,
        "freshness loss must cause the transmitter to be unkeyed"
    );
}

#[test]
fn ah4_failure_status_does_not_key_transmitter() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFC)));

    // Allow the engine to process the failure status.
    wait_until("tuning failure handled", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xFC))),
        )
    });

    thread::sleep(Duration::from_millis(50));

    assert!(
        !keyed.load(Ordering::SeqCst),
        "a terminal tuner failure must not enable RF"
    );

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}

#[test]
fn ah4_timeout_without_rf_request_never_keys_transmitter() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    // No 0xEE status is ever supplied. Wait beyond the engine's
    // four-second AH-4 timeout.
    thread::sleep(Duration::from_millis(4300));

    assert!(
        !keyed.load(Ordering::SeqCst),
        "the transmitter must remain unkeyed when no RF request arrives"
    );

    let timeout_notice_seen = engine.event_rx.try_iter().any(|event| {
        matches!(
            event,
            RadioEvent::Notice(Some(notice))
                if notice.to_lowercase().contains("ah-4")
                    && notice.to_lowercase().contains("timed out")
        )
    });

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }

    assert!(
        timeout_notice_seen,
        "the engine should publish an AH-4 timeout notice"
    );
}

#[test]
fn ah4_cancel_prevents_late_rf_authorization() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner-start request", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    assert!(!keyed.load(Ordering::SeqCst));

    // Cancel the pending tune using the normal PTT-off command.
    engine.cmd_tx.send(Command::SetPtt(false)).unwrap();

    // Give the engine time to process the cancellation before supplying
    // the delayed tuner status.
    thread::sleep(Duration::from_millis(100));

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    // A delayed RF-request status must not restart the cancelled tune.
    thread::sleep(Duration::from_millis(150));

    assert!(
        !keyed.load(Ordering::SeqCst),
        "late 0xEE status must not re-key a cancelled tuning operation"
    );

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}

#[test]
fn ah4_fault_after_tune_routes_next_request_to_recovery() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish positive tuner detection before testing AH-4 operation.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));

    wait_until("positive tuner presence", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerPresence(Some(true))),
        )
    });

    // Establish a known idle tuner state before requesting a tune.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("initial idle tuner status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0x00))),
        )
    });

    // Start and complete a normal tune.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("tuner start", Duration::from_secs(2), || {
        tuner_starts.load(Ordering::SeqCst) == 1
    });

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    wait_until("tuning RF", Duration::from_secs(2), || {
        keyed.load(Ordering::SeqCst)
    });

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("tuning unkeyed", Duration::from_secs(2), || {
        !keyed.load(Ordering::SeqCst)
    });

    // A recognised fault must take priority over the normal tune/bypass toggle.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFC)));

    wait_until("fault status delivered", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xFC))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("recovery request", Duration::from_secs(2), || {
        tuner_recovers.load(Ordering::SeqCst) == 1
    });

    assert_eq!(
        tuner_bypasses.load(Ordering::SeqCst),
        0,
        "a recognised fault must request recovery, not normal bypass"
    );
    assert!(
        !keyed.load(Ordering::SeqCst),
        "fault recovery must never key the transmitter"
    );

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}

#[test]
fn ah4_recovery_status_never_keys_transmitter() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish a recognised fault before requesting recovery.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFC)));

    wait_until("fault status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xFC))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("recovery request", Duration::from_secs(2), || {
        tuner_recovers.load(Ordering::SeqCst) == 1
    });

    // 0xEE must not grant RF while recovery is pending.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xEE)));

    wait_until("recovery 0xEE status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xEE))),
        )
    });

    thread::sleep(Duration::from_millis(50));

    assert!(
        !keyed.load(Ordering::SeqCst),
        "0xEE during recovery must never key the transmitter"
    );
    assert_eq!(tuner_starts.load(Ordering::SeqCst), 0);
    assert_eq!(tuner_bypasses.load(Ordering::SeqCst), 0);

    // Only a subsequent idle status confirms recovery.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0x00)));

    wait_until("recovery confirmation", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(
                event,
                RadioEvent::Notice(Some(message))
                    if message.contains("AH-4 recovery confirmed")
            ),
        )
    });

    assert!(
        !keyed.load(Ordering::SeqCst),
        "confirmed recovery must leave the transmitter unkeyed"
    );

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}

#[test]
fn ah4_recovery_fault_response_fails_safely() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFC)));

    wait_until("initial tuner fault", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xFC))),
        )
    });

    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("recovery request", Duration::from_secs(2), || {
        tuner_recovers.load(Ordering::SeqCst) == 1
    });

    // A further fault status must terminate the pending recovery.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFD)));

    wait_until("recovery failure notice", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(
                event,
                RadioEvent::Notice(Some(message))
                    if message.contains("AH-4 recovery failed")
            ),
        )
    });

    assert!(
        !keyed.load(Ordering::SeqCst),
        "a failed recovery must leave the transmitter unkeyed"
    );
    assert_eq!(tuner_starts.load(Ordering::SeqCst), 0);
    assert_eq!(tuner_bypasses.load(Ordering::SeqCst), 0);
    assert_eq!(tuner_recovers.load(Ordering::SeqCst), 1);

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}

#[test]
fn ah4_recovery_stays_busy_when_status_is_unavailable() {
    let updates = Arc::new(Mutex::new(VecDeque::new()));
    let status_fresh = Arc::new(AtomicBool::new(true));
    let tuner_starts = Arc::new(AtomicUsize::new(0));
    let tuner_bypasses = Arc::new(AtomicUsize::new(0));
    let tuner_recovers = Arc::new(AtomicUsize::new(0));
    let keyed = Arc::new(AtomicBool::new(false));
    let tx_ends = Arc::new(AtomicUsize::new(0));

    let source = MockAh4Source {
        center: CENTER_HZ,
        updates: Arc::clone(&updates),
        status_fresh: Arc::clone(&status_fresh),
        tuner_starts: Arc::clone(&tuner_starts),
        tuner_bypasses: Arc::clone(&tuner_bypasses),
        tuner_recovers: Arc::clone(&tuner_recovers),
        keyed: Arc::clone(&keyed),
        tx_ends: Arc::clone(&tx_ends),
    };

    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: false,
        primary: false,
        ..EngineConfig::default()
    };

    let mut engine = start_engine(Box::new(source), caps(), cfg);
    let thread = engine.thread.take();

    // Establish a recognised tuner fault.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(Some(0xFC)));

    wait_until("fault status", Duration::from_secs(2), || {
        engine.event_rx.try_iter().any(
            |event| matches!(event, RadioEvent::TunerStatus(Some(0xFC))),
        )
    });

    // A fault must initiate recovery, not normal tuning or bypass.
    engine.cmd_tx.send(Command::Ah4Tune).unwrap();

    wait_until("recovery request", Duration::from_secs(2), || {
        tuner_recovers.load(Ordering::SeqCst) == 1
    });

    assert_eq!(tuner_starts.load(Ordering::SeqCst), 0);
    assert_eq!(tuner_bypasses.load(Ordering::SeqCst), 0);
    assert!(!keyed.load(Ordering::SeqCst));

    // Presence is a separate discovery result and must survive a temporary
    // loss of status while recovery is pending.
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerPresence(Some(true)));
    updates
        .lock()
        .unwrap()
        .push_back(ControlUpdate::TunerStatus(None));

    let mut busy = false;
    let mut saw_busy = false;
    let mut saw_unavailable = false;
    let mut saw_presence = false;

    wait_until(
        "unavailable status while recovery is busy",
        Duration::from_secs(2),
        || {
            for event in engine.event_rx.try_iter() {
                match event {
                    RadioEvent::Ah4Busy(value) => {
                        busy = value;
                        saw_busy |= value;
                    }
                    RadioEvent::TunerStatus(None) => {
                        saw_unavailable = true;
                    }
                    RadioEvent::TunerPresence(Some(true)) => {
                        saw_presence = true;
                    }
                    _ => {}
                }
            }

            saw_busy && saw_unavailable && saw_presence
        },
    );

    assert!(
        busy,
        "unavailable status must not end the recovery operation"
    );
    assert!(
        !keyed.load(Ordering::SeqCst),
        "recovery must never key the transmitter"
    );

    // Without a definitive result, the recovery timeout must eventually
    // clear the busy state, while leaving RF disabled.
    let mut saw_recovery_end = false;

    wait_until(
        "recovery timeout to clear busy state",
        Duration::from_secs(6),
        || {
            for event in engine.event_rx.try_iter() {
                if let RadioEvent::Ah4Busy(false) = event {
                    saw_recovery_end = true;
                }
            }
            saw_recovery_end
        },
    );

    assert!(
        !keyed.load(Ordering::SeqCst),
        "recovery timeout must not key the transmitter"
    );
    assert_eq!(tuner_starts.load(Ordering::SeqCst), 0);
    assert_eq!(tuner_bypasses.load(Ordering::SeqCst), 0);

    drop(engine.cmd_tx);
    if let Some(thread) = thread {
        thread.join().expect("engine thread panicked");
    }
}
