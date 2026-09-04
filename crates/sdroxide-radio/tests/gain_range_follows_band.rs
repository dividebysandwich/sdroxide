//! A front end whose gain ladder belongs to the band has to re-publish it
//! when the band moves.
//!
//! An RSP's RF gain is a step into a table, and how long that table is depends
//! on where the dial is: an RSPdx has 28 states at 250–420 MHz and 19 below
//! 12 MHz. Published once when the device opened, the range is wrong for most
//! of the spectrum — and wrong in the direction that hurts, because a slider
//! offering states the service refuses spends its top third moving nothing and
//! snapping back as the driver clamps.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, GainElement, GainUnit, RadioEvent};

const RATE: f64 = 2_000_000.0;
const HF: f64 = 7_100_000.0;
const VHF: f64 = 300_000_000.0;

/// The stand-in ladder: short on HF, long in the band an RSPdx has all of.
/// The exact numbers are the RSPdx's own, so a reader can check them against
/// the API specification's table rather than against a fiction.
fn states_at(hz: f64) -> u8 {
    if hz < 12e6 { 18 } else { 27 }
}

struct Rig {
    center: Arc<AtomicU64>,
}

impl Rig {
    fn hz(&self) -> f64 {
        f64::from_bits(self.center.load(Ordering::Relaxed))
    }
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.hz()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center.store(hz.to_bits(), Ordering::Relaxed);
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.001, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock banded front end".into()
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        // Clamped to the band, the way the driver reports what it kept.
        vec![("LNA".to_string(), -f64::from(states_at(self.hz())))]
    }
    fn learned_rx_gains(&self) -> Option<Vec<GainElement>> {
        Some(vec![GainElement::steps(
            "LNA",
            Direction::Rx,
            -f64::from(states_at(self.hz())),
            0.0,
            1.0,
        )])
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench front end".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 2_000_000_000.0)],
        // Deliberately the model-wide ceiling, as a real backend publishes it
        // before the engine has asked: the point is that it gets corrected.
        //
        // The transmit stage is here to be left alone. Nothing on this mock
        // transmits; it stands in for the front end that both answers
        // `learned_rx_gains` and has a transmitter, which is a combination the
        // trait allows and no backend has yet.
        gains: vec![
            GainElement::steps("LNA", Direction::Rx, -27.0, 0.0, 1.0),
            GainElement::db("DRIVE", Direction::Tx, 0.0, 47.0, 1.0),
        ],
        ..DeviceCaps::default()
    }
}

/// Run an engine, tune it where `tune_to` says, and return the last set of
/// gain stages it published — with whether that last word arrived as a
/// *revision* ([`RadioEvent::CapabilitiesUpdated`]) rather than as a whole new
/// front end. The distinction is the client's: a new front end makes it throw
/// away everything it holds about the old one.
fn gains_after(tune_to: Option<f64>) -> (Vec<GainElement>, bool) {
    let center = Arc::new(AtomicU64::new(HF.to_bits()));
    let mut h = start_engine(
        Box::new(Rig { center: Arc::clone(&center) }),
        caps(),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    let mut last: Option<(Vec<GainElement>, bool)> = None;
    let mut sent = tune_to.is_none();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            let (caps, update) = match ev {
                RadioEvent::Capabilities(c) => (c, false),
                RadioEvent::CapabilitiesUpdated(c) => (c, true),
                _ => continue,
            };
            if caps.gains.iter().any(|g| g.name == "LNA") {
                last = Some((caps.gains.clone(), update));
            }
        }
        if !sent {
            let _ = h.cmd_tx.send(Command::SetVfo {
                vfo: sdroxide_types::Vfo::A,
                hz: tune_to.expect("a frequency to tune to"),
            });
            sent = true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("the engine must publish an LNA range")
}

/// The LNA stage out of a published set.
fn lna(gains: &[GainElement]) -> &GainElement {
    gains.iter().find(|g| g.name == "LNA").expect("the LNA stage")
}

/// On the band the session came up on, the published range is that band's —
/// not the wider one the device was opened with.
#[test]
fn the_opening_range_is_narrowed_to_the_band_it_came_up_on() {
    let (gains, _) = gains_after(None);
    let g = lna(&gains);
    assert_eq!(g.min_db, -18.0, "HF has 19 states, not the model's 28");
    assert_eq!(g.unit, GainUnit::Step, "a state index is not decibels");
}

/// And it follows the dial: tuning to a band with a longer ladder re-publishes
/// the longer one — as a *revision* of the same front end, not as a new one.
/// That second half is not pedantry: a client reading it as a new radio blanks
/// its wideband waterfall, re-reads every image store and silences its
/// announcer, and would do all three every time the dial crossed a band edge.
#[test]
fn the_range_follows_a_retune() {
    let (gains, update) = gains_after(Some(VHF));
    assert_eq!(lna(&gains).min_db, -27.0, "250-420 MHz has all 28 states");
    assert!(update, "a band's ladder is a revision of this radio, not a different radio");
}

/// And it revises the *receive* stages only. `learned_rx_gains` answers for
/// those; a transmitter's stages are not its to answer for, and a front end
/// that had both would otherwise lose its drive control the first time the
/// dial crossed a band edge.
#[test]
fn a_retune_leaves_the_transmit_stages_alone() {
    let (gains, _) = gains_after(Some(VHF));
    let tx: Vec<&GainElement> = gains.iter().filter(|g| g.direction == Direction::Tx).collect();
    assert_eq!(tx.len(), 1, "the transmit stage went missing when the receive ladder moved");
    assert_eq!(tx[0].name, "DRIVE");
    assert_eq!((tx[0].min_db, tx[0].max_db), (0.0, 47.0));
}
