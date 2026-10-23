//! The link button opens the radio where it last tuned (#72).
//!
//! The report that produced this: a start frequency the tuner cannot reach —
//! an I.F. offset saved as a frequency, so the dongle was asked for 6 GHz —
//! meant the dongle never opened and the operator had to correct the number by
//! hand **every time**, including after pressing the link button. Two things
//! had to be true for that, and both are pinned here:
//!
//! - the program's own "last frequency that tuned" record held the frequency it
//!   had merely been *asked* for, so the fallback was falling back to the same
//!   wrong number; and
//! - the button and the background reconnect opened at the dial, which is that
//!   same wrong number — the first fix lived in the start-up path alone.
//!
//! The second is the one this file exists for: `EngineSwap::ReopenSource` is
//! the route the link button takes, and it must ask for a frequency this radio
//! has tuned.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_radio::{
    Complex32, EngineConfig, EngineSwap, IqSource, ReopenFn, Result, start_engine,
};
use sdroxide_types::DeviceCaps;

/// The start frequency that could not be tuned: an I.F. offset saved as one.
const IMPOSSIBLE: f64 = 6_000_000_000.0;
/// Where the radio was actually left — a QO-100 satellite downlink.
const QO100: f64 = 144_150_000.0;
const RATE: f64 = 48_000.0;

/// Every centre the factory was asked to open, in order.
type Asked = Arc<Mutex<Vec<f64>>>;

/// A source that comes up and tunes, at whatever centre it is opened on.
struct Real {
    center: f64,
}

impl IqSource for Real {
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
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "the radio".into()
    }
}

/// The stand-in the program substitutes when the radio would not open: it
/// reports the centre that was **asked for**, and asks to be reopened.
struct StandIn {
    center: f64,
}

impl IqSource for StandIn {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "no radio".into()
    }
    fn open_status(&self) -> Option<String> {
        Some("the tuner refused to open".into())
    }
    fn needs_reopen(&self) -> bool {
        true
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "the radio".into(),
        rx_channels: 1,
        tx_channels: 0,
        audio_mode: false,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_800_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// `SDROXIDE_CONFIG_DIR` is process state, and Rust runs the tests in one
/// binary on several threads — so two tests each setting it would race, and
/// the loser would read the winner's session. Held for the length of a test.
static CONFIG_DIR: Mutex<()> = Mutex::new(());

/// An isolated configuration directory, so the session this writes is this
/// test's and not the operator's. `SDROXIDE_CONFIG_DIR` is what every `Store`
/// constructor reads, which is how the other persistence tests do it.
///
/// Callers hold [`CONFIG_DIR`].
fn temp_root(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sdroxide-reopen-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    root
}

/// A factory that behaves like an RTL-SDR: it refuses anything past its tuner
/// and opens anything below, recording what it was asked for each time.
fn tuner(asked: Asked) -> ReopenFn {
    Box::new(move |center: f64| {
        asked.lock().unwrap().push(center);
        // An RTL-SDR's own limits: it stops at 1,766 MHz going up and 24 MHz
        // coming down, which is the floor that made the first channel on his
        // station a poor first guess.
        if center > 1_766_000_000.0 || center < 24_000_000.0 {
            return Err(format!(
                "{center} MHz is outside this RTL-SDR's tuning range, 24 MHz to 1766 MHz"
            ));
        }
        Ok((Box::new(Real { center }) as Box<dyn IqSource>, caps()))
    })
}

/// A session that remembers where the radio was left, which is the only place
/// a fallback can honestly come from.
///
/// `last_good` is what the station recorded. `QO100` is a working station; the
/// impossible frequency is what a station that never managed to tune holds,
/// because the stand-in it ran on answered every tune with "done".
fn session_recalling(store: &sdroxide_config::Store, last_good: f64) {
    let mut s = store.load_session();
    s.last_good_vfo_hz = last_good;
    store.save_session(&s).unwrap();
}

/// The operator's own channel list — the only honest source left once the
/// record is the number that just failed. In the order stored, which is what
/// the walk follows.
fn store_channels(channels: &[(f64, &str)]) {
    let m: Vec<sdroxide_types::MemoryChannel> = channels
        .iter()
        .enumerate()
        .map(|(i, (freq_hz, name))| sdroxide_types::MemoryChannel {
            id: i as u32 + 1,
            name: (*name).into(),
            freq_hz: *freq_hz,
            mode: sdroxide_types::Mode::Usb,
            filter_lo: -2900.0,
            filter_hi: -200.0,
            folder: None,
            rtty: None,
            repeater: None,
            antenna: None,
            color: None,
        })
        .collect();
    sdroxide_config::save_memories(&m).unwrap();
}

fn store_qo100_channel() {
    store_channels(&[(QO100, "QO-100")]);
}

#[test]
fn the_link_button_opens_at_the_last_frequency_that_tuned() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("link");
    let store = sdroxide_config::Store::radio(0);
    session_recalling(&store, QO100);

    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        reopen: Some(tuner(Arc::clone(&asked))),
        ..Default::default()
    };
    // Started on the stand-in at the impossible frequency — a start frequency
    // the tuner cannot reach, which is what the report is.
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);

    // The link button. Its route is a swap, not a command, and going through
    // the real one is the point: this is what the button does.
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if asked.lock().unwrap().iter().any(|&c| (c - QO100).abs() < 1.0) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let asks = asked.lock().unwrap().clone();
    assert!(
        asks.iter().any(|&c| (c - QO100).abs() < 1.0),
        "the link button never asked for the frequency the radio had tuned; it asked {asks:?} \
         — {IMPOSSIBLE} Hz is past an RTL-SDR's tuner, so none of those could open"
    );
    assert!(
        !asks.iter().any(|&c| (c - IMPOSSIBLE).abs() < 1.0),
        "the button repeated the impossible frequency rather than the one that worked: {asks:?}"
    );

    // Stop it, which is what writes the session.
    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_start_that_never_opened_does_not_record_the_frequency_it_was_asked_for() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("seed");
    let store = sdroxide_config::Store::radio(0);
    session_recalling(&store, QO100);

    // No factory at all: nothing can open, so this run never tunes.
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);

    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }

    let recorded = store.load_session().last_good_vfo_hz;
    assert_ne!(
        recorded, IMPOSSIBLE,
        "a front end that never opened recorded {IMPOSSIBLE} Hz as a frequency that tuned — \
         that is what made the fallback fall back to the same wrong number for ever"
    );
    assert_eq!(
        recorded, QO100,
        "nothing tuned on this run, so the record should still be the last frequency that did"
    );

    let _ = std::fs::remove_dir_all(&root);
}
#[test]
fn a_station_whose_record_is_the_unreachable_number_still_opens_on_its_own_channel() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("poisoned");
    let store = sdroxide_config::Store::radio(0);

    // His state exactly: the session holds the unreachable frequency as "the
    // last one that tuned", because the stand-in answered every tune with
    // "done". Nothing is offered but the operator's own channel list.
    session_recalling(&store, IMPOSSIBLE);
    store_qo100_channel();

    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        reopen: Some(tuner(Arc::clone(&asked))),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);

    // Start-up offers the fallback first, before anything is pressed: the
    // program is already running because the radio would not open, so the very
    // first attempt must not repeat it.
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if asked.lock().unwrap().iter().any(|&c| (c - QO100).abs() < 1.0) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let asks = asked.lock().unwrap().clone();
    assert!(
        asks.iter().any(|&c| (c - QO100).abs() < 1.0),
        "with the record holding {IMPOSSIBLE} Hz, the radio never opened on a channel of the \
         operator's own; it asked {asks:?}"
    );

    // And the record repairs itself from there, so the next start does not need
    // the channel list either.
    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let recorded = store.load_session().last_good_vfo_hz;
    assert!(
        (recorded - QO100).abs() < 1.0 || recorded != IMPOSSIBLE,
        "the record is still the unreachable number after a successful open: {recorded}"
    );
    assert_ne!(
        recorded, IMPOSSIBLE,
        "a successful open must replace the unreachable number in the record"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unreachable_channel_does_not_strand_the_station_on_the_next_one() {
    let _dir = CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("walk");
    let store = sdroxide_config::Store::radio(0);

    // His station: the record holds the unreachable number, and the *first*
    // channel is one this radio cannot reach — 6.2 MHz on an RTL-SDR, whose own
    // floor is 24 MHz. It works on the RSP1, which is exactly why a bench test
    // alone would have passed this.
    session_recalling(&store, IMPOSSIBLE);
    store_channels(&[(6_200_000.0, "60 m IF"), (145_000_000.0, "low band")]);

    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let cfg = EngineConfig {
        tx_ham_only: false,
        remember_session: true,
        store: store.clone(),
        reopen: Some(tuner(Arc::clone(&asked))),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(StandIn { center: IMPOSSIBLE }), caps(), cfg);
    h.swap_tx.send(EngineSwap::ReopenSource).unwrap();

    // The tuner this test uses refuses anything under 24 MHz, as an RTL-SDR
    // does, so the walk has to move past the 6.2 MHz channel to reach one.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if asked.lock().unwrap().iter().any(|&c| (c - 145_000_000.0).abs() < 1.0) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let asks = asked.lock().unwrap().clone();
    assert!(
        asks.iter().any(|&c| (c - 145_000_000.0).abs() < 1.0),
        "the walk stopped at a channel this radio cannot reach; it asked {asks:?}"
    );
    assert!(
        asks.iter().any(|&c| (c - 6_200_000.0).abs() < 1.0),
        "the first channel should still have been tried — the point is that the walk moves on, \
         not that it skips; it asked {asks:?}"
    );

    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let _ = std::fs::remove_dir_all(&root);
}
