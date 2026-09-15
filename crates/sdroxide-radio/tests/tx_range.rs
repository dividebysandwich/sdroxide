//! What decides whether a key-down is allowed to reach the antenna.
//!
//! The case that prompted these: a driver that implements neither of
//! SoapySDR's frequency-range calls (SoapySX among others) hands over empty
//! range lists, and an empty list read as "this radio transmits nowhere" left a
//! working transceiver unable to key up at all — "TX refused: frequency outside
//! the device TX range", on a radio that receives perfectly well on the same
//! frequency. Silence from a driver is "it didn't say", and the operator can
//! say it instead.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioEvent, Vfo};

/// A transmit-capable stand-in that records the frequencies it was keyed on.
struct MockRig {
    rate: f64,
    center: f64,
    keyed: Arc<Mutex<Vec<f64>>>,
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
        // Pace like hardware; the engine is otherwise free-running here.
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock rig".into()
    }
    fn tx_begin(&mut self, center_hz: f64, rate: f64) -> Result<f64> {
        self.keyed.lock().unwrap().push(center_hz);
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Key up on `hz` with these capabilities, and report whether the transmitter
/// was actually started.
fn keys_up_on(caps: DeviceCaps, hz: f64) -> bool {
    keys_up_on_with(caps, hz, false, false)
}

/// The same, but with the amateur-band licence gate armed (`tx_ham_only`) and,
/// when wanted, the 11 m opt-in. The gate tests below need it armed: its whole
/// point is that it refuses every non-amateur band.
fn keys_up_on_with(caps: DeviceCaps, hz: f64, tx_ham_only: bool, cb_tx_allowed: bool) -> bool {
    let keyed = Arc::new(Mutex::new(Vec::new()));
    let src = MockRig { rate: 600_000.0, center: hz, keyed: Arc::clone(&keyed) };
    let cfg = EngineConfig { tx_ham_only, cb_tx_allowed, ..Default::default() };
    let mut h = start_engine(Box::new(src), caps, cfg);
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz }).unwrap();
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();

    // TX begins on the loop pass after the command, so give it a moment — and
    // stop as soon as the answer is in, either way.
    let mut ptt = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !ptt {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev {
                ptt |= s.tx.ptt;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    let started = !keyed.lock().unwrap().is_empty();
    assert_eq!(ptt, started, "PTT and the transmitter itself must agree");
    started
}

fn caps(rx: Vec<(f64, f64)>, tx: Vec<(f64, f64)>) -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![600_000.0],
        freq_ranges_rx: rx,
        freq_ranges_tx: tx,
        ..DeviceCaps::default()
    }
}

/// The regression: a transceiver whose driver publishes no ranges must key up.
#[test]
fn a_driver_that_publishes_no_ranges_can_still_transmit() {
    assert!(
        keys_up_on(caps(vec![], vec![]), 435_000_000.0),
        "a transmit-capable device that publishes no TX range must still key up"
    );
}

/// A device that only publishes its receive range is in the same position on
/// the transmit side — nothing was said, so nothing is enforced.
#[test]
fn an_unpublished_tx_range_is_not_a_published_empty_one() {
    assert!(keys_up_on(caps(vec![(1_000_000.0, 1_000_000_000.0)], vec![]), 435_000_000.0));
}

/// And a device that does publish a transmit range is still held to it, which
/// is the whole point of asking.
#[test]
fn a_published_tx_range_is_enforced() {
    let hf_only = caps(vec![(100_000.0, 1_000_000_000.0)], vec![(1_800_000.0, 54_000_000.0)]);
    assert!(keys_up_on(hf_only.clone(), 14_200_000.0), "inside the range");
    assert!(!keys_up_on(hf_only, 435_000_000.0), "outside it");
}

/// The operator's own limit, as `radio.json` states it, replaces whatever the
/// device said — including the nothing that started all this.
#[test]
fn a_stated_range_becomes_the_limit() {
    let stated = sdroxide_radio::override_caps_ranges(
        caps(vec![], vec![]),
        &[(430_000_000.0, 440_000_000.0)],
        &[(430_000_000.0, 440_000_000.0)],
    );
    assert!(keys_up_on(stated.clone(), 435_000_000.0), "inside the range the operator stated");
    assert!(!keys_up_on(stated, 145_500_000.0), "outside it — the operator's limit is a limit");
}

/// 11 m is a transmitting service but not an amateur allocation, so the licence
/// gate refuses it by default even on a radio that covers it — and opens it only
/// when the station has deliberately opted in (`cb_tx_allowed`, the switch
/// behind the one-time warning). The opt-in opens 11 m and nothing else: the
/// broadcast and general-coverage bands stay receive-only.
#[test]
fn eleven_metres_needs_the_cb_opt_in() {
    let wide = caps(vec![(500_000.0, 60_000_000.0)], vec![(500_000.0, 60_000_000.0)]);
    let cb = 27_265_000.0;
    let bc = 6_000_000.0; // 49 m broadcast, not an amateur band either

    assert!(!keys_up_on_with(wide.clone(), cb, true, false), "11 m must be locked by default");
    assert!(
        !keys_up_on_with(wide.clone(), bc, true, false),
        "broadcast stays locked with the opt-in off"
    );

    assert!(keys_up_on_with(wide.clone(), cb, true, true), "11 m must key once opted in");
    assert!(!keys_up_on_with(wide, bc, true, true), "the opt-in opens 11 m only");
}
