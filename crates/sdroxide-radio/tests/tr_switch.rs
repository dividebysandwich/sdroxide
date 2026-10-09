//! The external T/R switch against the engine, with a fake relay board.
//!
//! Every claim here is one an operator is trusting with their receiver's front
//! end, so each is measured rather than assumed: that the contacts close
//! *before* `tx_begin` by at least the lead they asked for, that they open
//! *after* `tx_end` by at least the hold, that a refused key-down leaves them
//! exactly where they were, and that a module follows only the radio it
//! belongs to.
//!
//! The board is a `RelayTransport` that records `(when, mask)`; the radio is an
//! `IqSource` that records when it was keyed and unkeyed. Both clocks are the
//! same `Instant`, which is what makes "before" and "after" checkable.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{
    Complex32, ControlUpdate, EngineConfig, IqSource, RadioError, Result, TrSwitch, start_engine,
};
use sdroxide_relay::{ChannelMask, RelayTransport};
use sdroxide_types::{
    Command, DeviceCaps, RadioEvent, RelayChannel, RelayConfig, RelayLink, RelayRole,
};

const RATE: f64 = 48_000.0;
const DIAL: f64 = 14_074_000.0;

/// Long enough to measure across a thread hand-off without being so long the
/// test is slow. Real values are a tenth of these.
const LEAD_MS: u16 = 60;
const HOLD_MS: u16 = 80;
/// How much of the lead the assertions insist on actually seeing. The engine
/// sleeps the whole of it, but the worker's wake-up and this process's
/// scheduling both come out of the middle, so the margin is deliberately
/// generous — the claim under test is "the contacts led the RF by most of what
/// was asked", not "by exactly 60.000 ms".
const SLACK_MS: u64 = 20;

/// The T/R switch's own configuration for these tests: one contact, grounding
/// the SDR's antenna, active-high.
fn relay_cfg() -> RelayConfig {
    RelayConfig {
        link: RelayLink::Serial,
        channels: vec![RelayChannel {
            index: 1,
            role: RelayRole::SdrAntenna,
            label: "SDR".into(),
            active_high: true,
            lead_ms: LEAD_MS,
            hold_ms: HOLD_MS,
        }],
        ..RelayConfig::default()
    }
}

// ── the fake board ──────────────────────────────────────────────────────────

#[derive(Default)]
struct BoardLog {
    /// Every state the contacts were put into, and when.
    changes: Vec<(Instant, ChannelMask)>,
    /// Whether the next write should fail, standing in for a pulled cable.
    fail: bool,
}

struct FakeBoard {
    log: Arc<Mutex<BoardLog>>,
    last: Option<ChannelMask>,
}

impl RelayTransport for FakeBoard {
    fn apply(&mut self, want: ChannelMask) -> sdroxide_relay::Result<()> {
        let mut l = self.log.lock().unwrap();
        if l.fail {
            return Err(sdroxide_relay::Error::NoAnswer { path: "fake".into() });
        }
        if self.last != Some(want) {
            self.last = Some(want);
            l.changes.push((Instant::now(), want));
        }
        Ok(())
    }
    fn round_trip(&self) -> Duration {
        Duration::from_millis(1)
    }
    fn describe(&self) -> String {
        "fake relay board".into()
    }
}

// ── the fake radio ──────────────────────────────────────────────────────────

#[derive(Default)]
struct RigLog {
    keyed: Vec<Instant>,
    unkeyed: Vec<Instant>,
    local_ptt: Option<bool>,
    /// Whether `tx_begin` should refuse — a radio that will not key.
    refuse: bool,
}

struct MockTrx {
    log: Arc<Mutex<RigLog>>,
}

impl IqSource for MockTrx {
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
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock transceiver".into()
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.log.lock().unwrap().local_ptt.take().map(ControlUpdate::RigTx).into_iter().collect()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        let mut l = self.log.lock().unwrap();
        if l.refuse {
            return Err(RadioError::Msg("the amplifier interlock is open".into()));
        }
        l.keyed.push(Instant::now());
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        self.log.lock().unwrap().unkeyed.push(Instant::now());
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The engine writes `session.json` and reads `relay.json`, and
/// `SDROXIDE_CONFIG_DIR` is process-global — so without this the test would
/// read the operator's real T/R switch configuration and try to open their
/// serial port.
fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root = std::env::temp_dir().join(format!("sdroxide-tr-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    });
}

struct Station {
    engines: Vec<sdroxide_radio::EngineHandles>,
    rigs: Vec<Arc<Mutex<RigLog>>>,
    board: Arc<Mutex<BoardLog>>,
    /// The switch of the radio that owns the fake board.
    hub: Arc<TrSwitch>,
}

/// Bring up `n` engines, each with its own T/R switch, and the fake board
/// installed on the first radio's.
///
/// No `TxGate`: the interlock is a separate rule with its own test, and here it
/// would stop a second radio ever reaching the air — which is the case the
/// ownership tests are about.
fn station(n: u32) -> Station {
    station_with(n, relay_cfg(), caps())
}

/// [`station`], with the switch's configuration and the radio's capabilities
/// given rather than the defaults.
fn station_with(n: u32, cfg: RelayConfig, caps: DeviceCaps) -> Station {
    station_owned(n, 0, cfg, caps)
}

/// [`station_with`], with the fake board belonging to radio `owner` rather
/// than the first.
fn station_owned(n: u32, owner: usize, cfg: RelayConfig, caps: DeviceCaps) -> Station {
    isolate_config();
    let board = Arc::new(Mutex::new(BoardLog::default()));
    let mut engines = Vec::new();
    let mut rigs = Vec::new();
    let mut hubs = Vec::new();
    for i in 0..n {
        let log = Arc::new(Mutex::new(RigLog::default()));
        let hub = Arc::new(TrSwitch::new());
        let h = start_engine(
            Box::new(MockTrx { log: Arc::clone(&log) }),
            caps.clone(),
            EngineConfig {
                tx_ham_only: false,
                instance: i,
                // Nothing opens hardware here: the installed handle below
                // replaces whatever `sync_relay` decided at boot.
                primary: i == 0,
                tr_switch: Some(Arc::clone(&hub)),
                ..Default::default()
            },
        );
        h.cmd_tx.send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: DIAL }).unwrap();
        engines.push(h);
        rigs.push(log);
        hubs.push(hub);
    }
    let hub = Arc::clone(&hubs[owner]);
    let st = Station { engines, rigs, board, hub };
    // Wait for the owning engine's *own* `sync_relay` to have run before
    // installing the board over the top of it.
    //
    // `RadioEvent::RelayStatus` and not `State`: the state goes out early in
    // the boot sequence and `sync_relay` runs later, so on a loaded machine the
    // install below landed first and the engine then replaced the fake board
    // with the `None` its (empty) `relay.json` asked for. Three of these tests
    // failed that way, and only when the whole suite was running.
    st.wait(owner, "the  engine's own T/R switch to settle", |ev| {
        matches!(ev, RadioEvent::RelayStatus(_))
    });
    st.hub.install(
        Some(sdroxide_relay::spawn(
            Box::new(FakeBoard { log: Arc::clone(&st.board), last: None }),
            cfg.clone(),
        )),
        &cfg,
    );
    st
}

impl Station {
    fn wait(&self, engine: usize, what: &str, mut f: impl FnMut(&RadioEvent) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            while let Ok(ev) = self.engines[engine].event_rx.try_recv() {
                if f(&ev) {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("timed out waiting for {what}");
    }

    /// Wait until the contacts are in `mask`, or give up.
    ///
    /// By state rather than by count on purpose: the driver writes every
    /// managed contact once at startup — a board and this end have no agreement
    /// to diff against until it has — so a change *count* is one larger than
    /// the number of things that have actually happened, and a test that waited
    /// on it would sail past the event it was waiting for. It did.
    fn wait_state(&self, mask: ChannelMask) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.board.lock().unwrap().changes.last().map(|(_, m)| *m) == Some(mask) {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn key(&self, engine: usize, on: bool) {
        self.engines[engine].cmd_tx.send(Command::SetPtt(on)).unwrap();
    }

    fn changes(&self) -> Vec<(Instant, ChannelMask)> {
        self.board.lock().unwrap().changes.clone()
    }

    /// When the contacts first went to transmit, and when they came back.
    fn closed_opened(&self) -> (Option<Instant>, Option<Instant>) {
        let c = self.changes();
        let closed = c.iter().find(|(_, m)| *m != 0).map(|(t, _)| *t);
        let opened = closed.and_then(|_| {
            c.iter().skip_while(|(_, m)| *m == 0).find(|(_, m)| *m == 0).map(|(t, _)| *t)
        });
        (closed, opened)
    }

    fn shutdown(mut self) {
        // Drop the handle first, so the worker stands the contacts down while
        // the engines are still alive to be blamed if it does not.
        self.hub.install(None, &RelayConfig::default());
        for h in self.engines.iter_mut() {
            let thread = h.thread.take();
            let (cmd, _) = crossbeam_channel::unbounded::<Command>();
            let dead = std::mem::replace(&mut h.cmd_tx, cmd);
            drop(dead);
            if let Some(t) = thread {
                let _ = t.join();
            }
        }
    }
}

/// The claim the whole subsystem exists to make: the antenna relay is closed
/// before any RF appears, and does not open until after it has stopped.
#[test]
fn the_contacts_lead_the_rf_and_trail_it() {
    let st = station(1);
    st.key(0, true);

    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[0].lock().unwrap().keyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let keyed = *st.rigs[0].lock().unwrap().keyed.first().expect("the radio was never keyed");
    let (closed, _) = st.closed_opened();
    let closed = closed.expect("the contacts never closed");
    assert!(closed < keyed, "RF was let out before the contacts closed");
    let lead = keyed.duration_since(closed);
    assert!(
        lead >= Duration::from_millis(u64::from(LEAD_MS) - SLACK_MS),
        "the contacts led the RF by only {lead:?}, not the {LEAD_MS} ms asked for"
    );

    st.key(0, false);
    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[0].lock().unwrap().unkeyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let unkeyed = *st.rigs[0].lock().unwrap().unkeyed.first().expect("the radio never unkeyed");
    st.wait_state(0);
    let (_, opened) = st.closed_opened();
    let opened = opened.expect("the contacts never came back to receive");
    assert!(opened > unkeyed, "the antenna came back before the transmitter stopped");
    let hold = opened.duration_since(unkeyed);
    assert!(
        hold >= Duration::from_millis(u64::from(HOLD_MS) - SLACK_MS),
        "the contacts trailed the RF by only {hold:?}, not the {HOLD_MS} ms asked for"
    );
    st.shutdown();
}

#[test]
fn local_microphone_ptt_switches_the_relay_without_keying_the_radio() {
    let st = station(1);
    st.wait_state(0);
    st.rigs[0].lock().unwrap().local_ptt = Some(true);
    st.wait_state(1);
    assert_eq!(st.changes().last().map(|(_, m)| *m), Some(1), "local PTT did not close the relay");
    assert!(st.rigs[0].lock().unwrap().keyed.is_empty(), "local PTT commanded radio key-down");
    st.rigs[0].lock().unwrap().local_ptt = Some(false);
    st.wait_state(0);
    assert_eq!(
        st.changes().last().map(|(_, m)| *m),
        Some(0),
        "local release did not open the relay"
    );
    assert!(st.rigs[0].lock().unwrap().unkeyed.is_empty(), "local release commanded radio unkey");
    st.shutdown();
}

/// A key-down refused by one of the engine's rails must leave the hardware
/// exactly as it was. The relay is thrown after every rail for precisely this
/// reason, so there is nothing to unwind — and this is what proves the ordering
/// did not drift.
#[test]
fn a_refused_key_down_never_touches_the_contacts() {
    let st = station(1);
    // Out of the transmit range this radio declares, which is a refusal from
    // `caps.may_tx_hz` — a rail well before the relay.
    st.engines[0]
        .cmd_tx
        .send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 200_000_000.0 })
        .unwrap();
    st.key(0, true);
    st.wait(0, "the refusal", |ev| matches!(ev, RadioEvent::Notice(Some(_))));
    std::thread::sleep(Duration::from_millis(100));

    assert!(
        st.changes().iter().all(|(_, m)| *m == 0),
        "a refused key-down threw the antenna relay: {:?}",
        st.changes()
    );
    assert!(st.rigs[0].lock().unwrap().keyed.is_empty(), "and it reached the transmitter");
    st.shutdown();
}

/// A radio that accepts the request and then refuses to key. No RF appeared, so
/// the contacts drop at once rather than serving out the hold — a receiver held
/// off for an over that never happened is deaf for nothing.
#[test]
fn a_radio_that_refuses_to_key_drops_the_contacts_without_the_hold() {
    let st = station(1);
    st.rigs[0].lock().unwrap().refuse = true;
    st.key(0, true);
    st.wait(
        0,
        "the refusal",
        |ev| matches!(ev, RadioEvent::Notice(Some(s)) if s.contains("refused to key")),
    );
    st.wait_state(0);

    let (closed, opened) = st.closed_opened();
    let closed = closed.expect("the contacts should have been thrown before the attempt");
    let opened = opened.expect("and dropped again when nothing came of it");
    let held = opened.duration_since(closed);
    assert!(
        held < Duration::from_millis(u64::from(HOLD_MS)),
        "the contacts served out the {HOLD_MS} ms hold ({held:?}) for an over that never happened"
    );
    st.shutdown();
}

/// A module belongs to one radio. Another radio keying — with this radio's
/// module installed and this radio receiving — must not throw it: that radio
/// has its own antenna, and its own module if it needs one.
#[test]
fn another_radio_keying_leaves_this_radios_module_alone() {
    let st = station(2);
    st.key(1, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[1].lock().unwrap().keyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!st.rigs[1].lock().unwrap().keyed.is_empty(), "the second radio never keyed");
    std::thread::sleep(Duration::from_millis(u64::from(LEAD_MS) + 120));
    assert!(
        st.changes().iter().all(|(_, m)| *m == 0),
        "the second radio keying threw the first radio's module: {:?}",
        st.changes()
    );
    st.key(1, false);

    // ...and the owner keying still does.
    st.key(0, true);
    st.wait_state(1);
    assert_eq!(st.changes().last().map(|(_, m)| *m), Some(1), "the owner's over never threw it");
    st.key(0, false);
    st.wait_state(0);
    st.shutdown();
}

/// The path a dropped remote session takes — `session.rs` sends `SetPtt(false)`
/// when a client disappears. Covered by construction, and asserted so it stays
/// that way.
#[test]
fn an_unkey_from_a_disconnecting_client_releases_the_contacts() {
    let st = station(1);
    st.key(0, true);
    st.wait_state(1);
    // Exactly what the server sends on a lost session.
    st.engines[0].cmd_tx.send(Command::SetPtt(false)).unwrap();
    st.engines[0].cmd_tx.send(Command::SetTune(false)).unwrap();
    st.wait_state(0);
    let (_, opened) = st.closed_opened();
    assert!(opened.is_some(), "a dropped client left the antenna grounded");
    st.shutdown();
}

/// The fail-safe an operator is actually relying on: a switch that will not
/// open refuses the over, the way the SWR guard does, rather than letting RF
/// out into an unprotected receiver.
#[test]
fn a_switch_that_cannot_be_opened_refuses_the_over() {
    let st = station(1);
    // No driver, and a reason — exactly the state `sync_relay` leaves the hub in
    // when the port is not there.
    let cfg = RelayConfig { fail_safe: sdroxide_types::FailSafe::RefuseTx, ..relay_cfg() };
    st.hub.install(None, &cfg);
    st.hub.set_open_error(Some("no T/R switch found at /dev/ttyUSB9".into()));

    st.key(0, true);
    st.wait(
        0,
        "the refusal",
        |ev| matches!(ev, RadioEvent::Notice(Some(s)) if s.contains("ttyUSB9")),
    );
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        st.rigs[0].lock().unwrap().keyed.is_empty(),
        "the radio was keyed into an unprotected receiver"
    );

    // ...and it is a refusal, not a lock. The operator chooses to go on without
    // the switch, and the very next key-down works.
    let cfg = RelayConfig { fail_safe: sdroxide_types::FailSafe::WarnOnly, ..cfg };
    st.hub.install(None, &cfg);
    st.key(0, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[0].lock().unwrap().keyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !st.rigs[0].lock().unwrap().keyed.is_empty(),
        "\"transmit anyway, warn\" refused anyway"
    );
    st.shutdown();
}

/// Under a satellite lock the transmitter is on the transponder's uplink, not
/// on the dial — so the band decoder's TX word has to be the uplink band's.
/// AO-7's mode B, a real inverting transponder: the dial on the 2 m downlink,
/// the transmitter on 70 cm. Taken from the dial, the key-down would have put
/// the 2 m filter in line with a 70 cm transmitter (issue #442).
#[test]
fn a_satellite_lock_switches_the_band_decoder_to_the_uplink_band() {
    use sdroxide_types::{Band, RelayBandRow, SatLockConfig, SatUplink};
    const TWO_M: ChannelMask = 0b010;
    const SEVENTY_CM: ChannelMask = 0b100;
    let filter = |index: u8| RelayChannel {
        index,
        role: RelayRole::BandDecoder,
        label: format!("filter {index}"),
        active_high: true,
        lead_ms: LEAD_MS,
        hold_ms: HOLD_MS,
    };
    let mut cfg = relay_cfg();
    cfg.channels.extend([filter(2), filter(3)]);
    cfg.band_table = vec![
        RelayBandRow { band: Band::M2, rx_mask: TWO_M, tx_mask: TWO_M },
        RelayBandRow { band: Band::M70, rx_mask: SEVENTY_CM, tx_mask: SEVENTY_CM },
    ];
    let caps = DeviceCaps { freq_ranges_tx: vec![(144e6, 148e6), (430e6, 440e6)], ..caps() };
    let st = station_with(1, cfg, caps);

    st.engines[0]
        .cmd_tx
        .send(Command::SetSatLock(Some(Box::new(SatLockConfig {
            norad_id: 7530,
            name: "OSCAR 7 (AO-7)".into(),
            tle: Some((
                "1 07530U 74089B   26205.50898980 -.00000033  00000+0  81693-4 0  9992".into(),
                "2 07530 101.9909 219.1448 0012602  61.1135  93.5622 12.53698681365193".into(),
            )),
            observer: Some((48.2, 16.4)),
            downlink_hz: 145_950_000.0,
            uplink: Some(SatUplink {
                up_lo_hz: 432_125_000.0,
                up_hi_hz: 432_175_000.0,
                down_lo_hz: 145_925_000.0,
                down_hi_hz: 145_975_000.0,
                inverting: true,
            }),
            doppler: false,
            rotator: false,
        }))))
        .unwrap();
    // Receiving on the downlink: the 2 m filter.
    st.wait_state(TWO_M);
    assert_eq!(st.changes().last().map(|(_, m)| *m), Some(TWO_M), "never settled on 2 m");

    st.key(0, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[0].lock().unwrap().keyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!st.rigs[0].lock().unwrap().keyed.is_empty(), "the radio never keyed");
    let on_air = st.changes().last().map(|(_, m)| *m);
    assert_eq!(
        on_air,
        Some(0b001 | SEVENTY_CM),
        "keyed on the 70 cm uplink with the contacts at {on_air:?}, not the 70 cm filter"
    );
    st.key(0, false);
    st.wait_state(TWO_M);
    st.shutdown();
}

/// The bug this ownership model fixes: a module assigned to a radio that is
/// not the first tab follows *that* radio's receive band, not the first tab's.
/// Radio 0 sits on 20 m; radio 1 owns the module and moves 40 m → 20 m → 40 m,
/// and the decoder's RX word follows it each time. Radio 0 keying moves
/// nothing; radio 1 keying puts its own transmit band's word in line, and its
/// own receive band's word comes back after the over.
#[test]
fn the_band_decoder_follows_its_own_radio() {
    use sdroxide_types::{Band, RelayBandRow};
    const TWENTY: ChannelMask = 0b010;
    const FORTY: ChannelMask = 0b100;
    let filter = |index: u8| RelayChannel {
        index,
        role: RelayRole::BandDecoder,
        label: format!("filter {index}"),
        active_high: true,
        lead_ms: LEAD_MS,
        hold_ms: HOLD_MS,
    };
    let mut cfg = relay_cfg();
    cfg.channels.extend([filter(2), filter(3)]);
    cfg.band_table = vec![
        RelayBandRow { band: Band::M20, rx_mask: TWENTY, tx_mask: TWENTY },
        RelayBandRow { band: Band::M40, rx_mask: FORTY, tx_mask: FORTY },
    ];
    let st = station_owned(2, 1, cfg, caps());
    let tune = |engine: usize, hz: f64| {
        st.engines[engine]
            .cmd_tx
            .send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz })
            .unwrap();
    };
    let settled = |mask: ChannelMask, what: &str| {
        st.wait_state(mask);
        assert_eq!(st.changes().last().map(|(_, m)| *m), Some(mask), "{what}");
    };

    tune(1, 7_074_000.0);
    settled(FORTY, "the owning radio's 40 m receive band never reached the module");

    tune(1, 14_074_000.0);
    settled(TWENTY, "the module stayed put when its own radio moved to 20 m");
    tune(1, 7_074_000.0);
    settled(FORTY, "the module stayed put when its own radio came back to 40 m");

    // The first tab moving and keying is no business of this module's.
    tune(0, 14_200_000.0);
    st.key(0, true);
    let deadline = Instant::now() + Duration::from_secs(5);
    while st.rigs[0].lock().unwrap().keyed.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!st.rigs[0].lock().unwrap().keyed.is_empty(), "the first radio never keyed");
    std::thread::sleep(Duration::from_millis(u64::from(LEAD_MS) + 120));
    assert_eq!(
        st.changes().last().map(|(_, m)| *m),
        Some(FORTY),
        "another radio's dial or over moved this radio's module"
    );
    st.key(0, false);

    // The owner keys on 40 m, then on 20 m after moving.
    st.key(1, true);
    settled(0b001 | FORTY, "the owner keyed without its 40 m filter in line");
    st.key(1, false);
    settled(FORTY, "the 40 m receive word did not come back after the over");
    tune(1, 14_074_000.0);
    settled(TWENTY, "the module did not follow the owner to 20 m after an over");
    st.shutdown();
}
