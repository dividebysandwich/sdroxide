//! A dial that has come to rest gets the I/Q centred on it — on a front end
//! where a retune costs nothing.
//!
//! A KiwiSDR streams 12 kHz of I/Q. `keep_vfo_in_span` moves that window only
//! once the dial is 45 % of it off centre, so a station tuned 4 kHz from the
//! centre keeps one sideband against the window's edge for as long as it is
//! listened to. After the dial has stood still for five seconds the engine
//! centres the I/Q on it, once; on hardware that does not opt in nothing moves.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Vfo};

const RATE: f64 = 12_000.0;
const START: f64 = 7_350_000.0;
/// 4 kHz up: inside the 45 % (5.4 kHz) that `keep_vfo_in_span` leaves alone.
const DIAL: f64 = 7_354_000.0;

/// A KiwiSDR stand-in: 12 kHz of silence, a centre the test can read from
/// outside the engine, and `cheap` as its answer to `recentre_on_dwell`.
struct Kiwiish {
    /// The I/Q centre in Hz, as `f64` bits, shared with the test.
    center: Arc<AtomicU64>,
    cheap: bool,
    /// What it answers to `center_is_dial`: a transceiver's I/Q.
    dial: bool,
    /// The highest centre it can reach, as a downconverter clamps its own
    /// (an RX-888 at the end of its half-spectrum).
    top: f64,
    /// How many times the engine asked for a centre.
    asked: Arc<AtomicU64>,
}

impl IqSource for Kiwiish {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        f64::from_bits(self.center.load(Ordering::Relaxed))
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.asked.fetch_add(1, Ordering::Relaxed);
        let hz = hz.min(self.top);
        self.center.store(hz.to_bits(), Ordering::Relaxed);
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(20));
        let n = buf.len().min(240);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock KiwiSDR".into()
    }
    fn recentre_on_dwell(&self) -> bool {
        self.cheap
    }
    fn center_is_dial(&self) -> bool {
        self.dial
    }
}

/// A configuration store of this test's own: a radio id nobody else uses,
/// under a temporary root set once for the whole binary. `SetRadioConfig`
/// writes `radio.json`, and the default store is the station's real one —
/// a test must neither change the operator's radio nor see another test's.
fn private_store() -> sdroxide_config::Store {
    static ROOT: std::sync::Once = std::sync::Once::new();
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(9000);
    ROOT.call_once(|| {
        let root = std::env::temp_dir().join(format!("sdroxide-dwell-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // SAFETY: set once, before the first engine of this binary starts, and
        // every test here opens its engine through this function.
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    });
    sdroxide_config::Store::radio(NEXT.fetch_add(1, Ordering::Relaxed))
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "kiwisdr".into(),
        label: "mock KiwiSDR".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 30_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Open at [`START`], tune to [`DIAL`], and return the I/Q centre after `early`
/// and after `late` of rest.
fn centres(cheap: bool, early: Duration, late: Duration) -> (f64, f64) {
    let (a, b, _) = run(Kiwiish::new(cheap), early, late);
    (a, b)
}

impl Kiwiish {
    fn new(cheap: bool) -> Self {
        Kiwiish {
            center: Arc::new(AtomicU64::new(START.to_bits())),
            cheap,
            dial: false,
            top: f64::INFINITY,
            asked: Arc::new(AtomicU64::new(0)),
        }
    }
}

/// [`centres`] for a source built by the test; also how often a centre was
/// asked for over the whole run.
fn run(src: Kiwiish, early: Duration, late: Duration) -> (f64, f64, u64) {
    run_with(src, None, Hole::Shown, early, late)
}

/// What the client tells the engine about a hole in the picture.
#[derive(Clone, Copy)]
enum Hole {
    /// From the start: the view runs off the I/Q window.
    Shown,
    /// Never: the view lies inside the window.
    None,
    /// After this long: a pan with the right button opens one later.
    After(Duration),
}

/// [`run`], with a radio configuration sent before the dial is tuned.
fn run_with(
    src: Kiwiish,
    cfg: Option<sdroxide_types::RadioConfig>,
    hole: Hole,
    early: Duration,
    late: Duration,
) -> (f64, f64, u64) {
    let (center, asked) = (src.center.clone(), src.asked.clone());
    let store = private_store();
    let mut h = start_engine(
        Box::new(src),
        caps(),
        EngineConfig { remember_session: false, store, ..Default::default() },
    );
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(250));
    if let Some(cfg) = cfg {
        h.cmd_tx.send(Command::SetRadioConfig { cfg: Box::new(cfg), reopen: false }).unwrap();
    }
    if matches!(hole, Hole::Shown) {
        h.cmd_tx.send(Command::SetViewUncovered(true)).unwrap();
    }
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL }).unwrap();
    if let Hole::After(d) = hole {
        std::thread::sleep(d);
        h.cmd_tx.send(Command::SetViewUncovered(true)).unwrap();
    }
    let drain = |h: &mut sdroxide_radio::EngineHandles, d: Duration| {
        let until = std::time::Instant::now() + d;
        while std::time::Instant::now() < until {
            while h.event_rx.try_recv().is_ok() {}
            let _ = h.spectrum_out.update();
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    drain(&mut h, early);
    let a = f64::from_bits(center.load(Ordering::Relaxed));
    drain(&mut h, late - early);
    let b = f64::from_bits(center.load(Ordering::Relaxed));
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    (a, b, asked.load(Ordering::Relaxed))
}

/// Proves: on a front end that opts in, the I/Q stays put while the dial has
/// rested 2 s and is centred on it after 6.5 s.
#[test]
fn a_resting_dial_gets_the_iq_centred_on_it() {
    let (early, late) = centres(true, Duration::from_secs(2), Duration::from_millis(6_500));
    // Still moving (well, just set): the window stays where it was.
    assert_eq!(early, START, "the I/Q moved before the dial had rested");
    // Five seconds on, it has come round the dial.
    assert_eq!(late, DIAL, "the I/Q was not centred on the resting dial");
}

/// Proves: on a front end that does not opt in, nothing moves however long
/// the dial rests.
#[test]
fn hardware_that_does_not_opt_in_is_left_alone() {
    let (_, late) = centres(false, Duration::from_secs(1), Duration::from_millis(6_500));
    assert_eq!(late, START);
}

/// Proves: on a radio whose centre is its dial the rest adds nothing, even
/// when it says a retune is cheap. Its centre follows the dial anyway (the
/// engine moves it with every tune); a recentre on top would turn the
/// transceiver's own frequency, so the opt-in must not change a thing.
#[test]
fn a_radio_whose_centre_is_its_dial_is_left_alone() {
    let d = (Duration::from_secs(1), Duration::from_millis(6_500));
    let opted = run(Kiwiish { dial: true, ..Kiwiish::new(true) }, d.0, d.1);
    let plain = run(Kiwiish { dial: true, ..Kiwiish::new(false) }, d.0, d.1);
    assert_eq!(opted, plain, "the rest moved a radio whose centre is its dial");
}

/// Proves: at an end stop the recentre goes as far as the front end can and
/// is not asked again for the same resting place — no retune per block
/// against a centre that cannot move further.
#[test]
fn a_resting_dial_past_the_centre_limit_is_asked_for_once() {
    let top = 7_352_000.0;
    let src = Kiwiish { top, ..Kiwiish::new(true) };
    let (_, late, asked) = run(src, Duration::from_secs(1), Duration::from_millis(8_000));
    assert_eq!(late, top, "the centre did not go as far as it could");
    assert!(asked <= 2, "the centre was asked for {asked} times for one resting place");
}

/// Proves: switched off in the radio's settings, centring on a resting dial
/// does not happen, however long the dial rests; the switch applies live.
#[test]
fn switched_off_in_the_settings_nothing_moves() {
    let off = sdroxide_types::RadioConfig { dwell_recentre_off: true, ..Default::default() };
    let (_, late, _) = run_with(
        Kiwiish::new(true),
        Some(off),
        Hole::Shown,
        Duration::from_secs(1),
        Duration::from_millis(6_500),
    );
    assert_eq!(late, START, "the window moved with centring switched off");
}

/// Proves: with nothing missing from the picture — the view inside the I/Q
/// window — a resting dial off centre is left where it is.
#[test]
fn a_view_inside_the_window_is_left_alone() {
    let (_, late, _) = run_with(
        Kiwiish::new(true),
        None,
        Hole::None,
        Duration::from_secs(1),
        Duration::from_millis(6_500),
    );
    assert_eq!(late, START, "the window moved without a hole in the picture");
}

/// Proves: a hole that opens after the dial's rest has had its turn (a pan
/// with the right button) starts a rest of its own and is filled five
/// seconds later.
#[test]
fn a_hole_opened_later_is_filled_after_its_own_rest() {
    let (early, late, _) = run_with(
        Kiwiish::new(true),
        None,
        Hole::After(Duration::from_secs(6)),
        Duration::from_secs(3),
        Duration::from_millis(6_500),
    );
    // Six seconds of rest without a hole, then three with one: too young.
    assert_eq!(early, START, "the window moved before the hole had rested");
    // Six and a half seconds after the hole opened: filled.
    assert_eq!(late, DIAL, "the hole was not filled after its own rest");
}
