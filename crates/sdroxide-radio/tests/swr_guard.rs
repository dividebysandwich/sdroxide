//! The SWR guard: stop transmitting into a fault, and stay stopped.
//!
//! Every test here is a bug that reached a live radio. The guard was written on
//! 17 August 2026 and failed its first two on-air tests in a row, both times by
//! sitting silent through a genuine 7.5:1 while the operator watched. Neither
//! failure was in the threshold or the debounce; both were in what the guard
//! demanded before it would believe a reading at all.
//!
//! 1. It required forward power of at least 5 W. A rig with SWR protection
//!    folds its power back BECAUSE the SWR is bad — the IC-7300 dropped to 3 W —
//!    so the floor gated out precisely the emergency it was guarding against.
//! 2. It required forward power to be reported at all. The CI-V driver sends
//!    `TxTelemetry { fwd_w: None, swr: Some(v) }`, so on every Icom the match
//!    arm could never fire and the whole feature was dead code.
//!
//! `trips_with_no_forward_power_reported` is the regression for both, and is
//! the shape of a real Icom: SWR present, forward power absent.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioEvent, TxTelemetry, Vfo};

/// A transmit-capable stand-in that reports whatever telemetry the test sets,
/// and records whether the transmitter is currently running.
struct MockRig {
    rate: f64,
    center: f64,
    telem: Arc<Mutex<Option<TxTelemetry>>>,
    keyed: Arc<Mutex<bool>>,
}

impl IqSource for MockRig {
    fn sample_rate(&self) -> f64 {
        self.rate
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
        "mock rig".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        *self.keyed.lock().unwrap() = true;
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        *self.keyed.lock().unwrap() = false;
        Ok(())
    }
    /// Latched, exactly as `AudioCatSource` latches the ~5 Hz CI-V reading so
    /// the engine's meter tick always has a value.
    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        *self.telem.lock().unwrap()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![600_000.0],
        freq_ranges_rx: vec![(1.0e6, 60.0e6)],
        freq_ranges_tx: vec![(1.0e6, 60.0e6)],
        ..DeviceCaps::default()
    }
}

struct Outcome {
    tripped_at: Option<f32>,
    /// How long after key-down the trip arrived, which is the only way to see a
    /// grace period from outside: the tune window is measured in seconds and an
    /// over's in a fifth of one.
    tripped_after: Option<Duration>,
    /// Whether the transmitter was still running when the test gave up.
    still_keyed: bool,
    /// Whether a key-up attempted AFTER the trip was allowed through.
    keyed_again: bool,
}

/// How the over is keyed. A tune is not an ordinary over: feeding a mismatch is
/// the point of it, so it is held to its own limit and its own grace period.
#[derive(Clone, Copy)]
enum Key {
    Ptt,
    Tune,
}

impl Key {
    fn cmd(self, on: bool) -> Command {
        match self {
            Key::Ptt => Command::SetPtt(on),
            Key::Tune => Command::SetTune(on),
        }
    }
}

/// An ordinary over, waited on long enough for the settle window plus the
/// debounce several times over, so a pass is not a race.
fn run(telem: TxTelemetry, limit: f32, guard: bool) -> Outcome {
    run_keyed(telem, limit, guard, Key::Ptt, Duration::from_secs(3))
}

/// Key up with `telem` reported throughout, wait up to `wait` for a verdict,
/// then try to key up a second time to see whether the latch holds.
fn run_keyed(telem: TxTelemetry, limit: f32, guard: bool, key: Key, wait: Duration) -> Outcome {
    let telem = Arc::new(Mutex::new(Some(telem)));
    let keyed = Arc::new(Mutex::new(false));
    let src = MockRig {
        rate: 600_000.0,
        center: 14.1e6,
        telem: Arc::clone(&telem),
        keyed: Arc::clone(&keyed),
    };
    let cfg = EngineConfig {
        tx_ham_only: false,
        swr_guard: guard,
        swr_limit: limit,
        ..Default::default()
    };
    let mut h = start_engine(Box::new(src), caps(), cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: 14.1e6 }).unwrap();
    let keyed_at = Instant::now();
    h.cmd_tx.send(key.cmd(true)).unwrap();

    let mut tripped_at = None;
    let mut tripped_after = None;
    let deadline = keyed_at + wait;
    while Instant::now() < deadline && tripped_at.is_none() {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev {
                if let Some(swr) = s.tx.swr_tripped {
                    tripped_at = Some(swr);
                    tripped_after.get_or_insert(keyed_at.elapsed());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let still_keyed = *keyed.lock().unwrap();

    // The latch: a second key-up must be refused while it stands.
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    while h.event_rx.try_recv().is_ok() {}
    let keyed_again = *keyed.lock().unwrap();

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    Outcome { tripped_at, tripped_after, still_keyed, keyed_again }
}

/// ⭐ THE REGRESSION. An Icom reports SWR and no forward power at all. This is
/// the exact shape of the telemetry that twice failed on air.
#[test]
fn trips_with_no_forward_power_reported() {
    let o = run(
        TxTelemetry { fwd_w: None, swr: Some(7.5), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, Some(7.5), "a 7.5:1 with no power figure must trip a 2.5:1 guard");
    assert!(!o.still_keyed, "the transmitter must actually stop, not just set a flag");
    assert!(!o.keyed_again, "the latch must refuse the next key-up until acknowledged");
}

/// The other half of the live failure: the rig folds back to a few watts
/// BECAUSE the SWR is bad, so a low power reading must not excuse it.
#[test]
fn trips_when_the_rig_has_folded_its_power_back() {
    let o = run(
        TxTelemetry { fwd_w: Some(3.0), swr: Some(7.5), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, Some(7.5), "3 W at 7.5:1 is the fault, not a reason to ignore it");
    assert!(!o.still_keyed);
}

#[test]
fn leaves_a_good_swr_alone() {
    let o = run(
        TxTelemetry { fwd_w: Some(50.0), swr: Some(1.3), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, None, "1.3:1 must not trip a 2.5:1 guard");
    assert!(o.still_keyed, "a good match must be left transmitting");
}

/// Disarmed is disarmed, however bad the antenna is.
#[test]
fn does_nothing_when_disarmed() {
    let o = run(
        TxTelemetry { fwd_w: None, swr: Some(9.9), alc: None, po: None, rfl_w: None },
        2.5,
        false,
    );
    assert_eq!(o.tripped_at, None);
    assert!(o.still_keyed);
}

/// A rig that reports no SWR at all must be left entirely alone, rather than
/// having its silence read as either safe or dangerous.
#[test]
fn inert_when_the_rig_reports_no_swr() {
    let o = run(
        TxTelemetry { fwd_w: Some(50.0), swr: None, alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, None);
    assert!(o.still_keyed);
}

/// Genuinely no forward power means the transmitter is producing nothing, and
/// the ratio is then noise. This is the only thing the power floor may do.
#[test]
fn ignores_a_reading_taken_at_no_power() {
    let o = run(
        TxTelemetry { fwd_w: Some(0.0), swr: Some(9.9), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, None, "0 W means the reading is meaningless, not that the SWR is bad");
}

/// THE TUNE ALLOWANCE. An antenna tuner is given something to tune on by
/// keying into the mismatch it exists to remove, so a tune runs to double the
/// limit: 4:1 is over the 2.5:1 set for an over and under the 5:1 a tune gets.
///
/// Waited out well past the five-second grace, so what this proves is the
/// raised LIMIT and not merely a longer window.
#[test]
fn a_tune_is_allowed_above_the_on_air_limit() {
    let o = run_keyed(
        TxTelemetry { fwd_w: None, swr: Some(4.0), alc: None, po: None, rfl_w: None },
        2.5,
        true,
        Key::Tune,
        Duration::from_secs(7),
    );
    assert_eq!(o.tripped_at, None, "4:1 is under the 5:1 a 2.5:1 setting allows while tuning");
    assert!(o.still_keyed, "the tune carrier must be left running for the ATU to work on");
}

/// The same reading, keyed as an ordinary over: the difference is the tune, not
/// the figure. Without this the test above would also pass on a guard that had
/// simply stopped working.
#[test]
fn the_same_swr_stops_an_ordinary_over() {
    let o = run(
        TxTelemetry { fwd_w: None, swr: Some(4.0), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    assert_eq!(o.tripped_at, Some(4.0), "4:1 is over a 2.5:1 limit when it is not a tune");
    assert!(!o.still_keyed);
}

/// THE GRACE PERIOD. A disconnected feeder reads at the top of the scale, and
/// a tune must still stop it — but only after the tuner has had its five
/// seconds, since a sweep that starts at 10:1 is what tuning looks like.
#[test]
fn a_tune_still_stops_on_a_dead_feeder() {
    let o = run_keyed(
        TxTelemetry { fwd_w: None, swr: Some(9.9), alc: None, po: None, rfl_w: None },
        2.5,
        true,
        Key::Tune,
        Duration::from_secs(10),
    );
    assert_eq!(o.tripped_at, Some(9.9), "9.9:1 is over the 5:1 a tune is allowed");
    let after = o.tripped_after.expect("a trip has a time");
    assert!(
        after > Duration::from_secs(4),
        "the tuner must get its grace period, not be cut off in a fifth of a second: {after:?}"
    );
    assert!(!o.still_keyed, "the carrier must actually stop once the grace has run out");
    assert!(!o.keyed_again, "and the latch holds afterwards, exactly as for an over");
}

/// The grace period is the tune's alone: an ordinary over is still stopped in a
/// fraction of a second, which is the whole reason the guard exists.
#[test]
fn an_over_is_stopped_without_waiting_seconds() {
    let o = run(
        TxTelemetry { fwd_w: None, swr: Some(9.9), alc: None, po: None, rfl_w: None },
        2.5,
        true,
    );
    let after = o.tripped_after.expect("a trip has a time");
    assert!(after < Duration::from_secs(1), "an over must not inherit the tune grace: {after:?}");
}
