//! Which sideband SSTV rides, and that it follows the band.
//!
//! Analog SSTV is a phone emission and follows phone practice rather than the
//! digital modes' fixed USB: LSB on 160, 80 and 40 m, USB on 20 m and up. The
//! sideband lives entirely in the sign of the filter edges, so what these hold
//! is that the passband mirrors — on entering the mode, and on tuning across
//! the boundary while already in it.

use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RxId, Vfo};

const RATE: f64 = 2_400_000.0;
/// The 20 m SSTV calling frequency — USB territory.
const SSTV_20M: f64 = 14_230_000.0;
/// The Region 2/3 40 m SSTV calling frequency — LSB territory.
const SSTV_40M: f64 = 7_171_000.0;
/// The Region 1 80 m SSTV calling frequency.
const SSTV_80M: f64 = 3_730_000.0;

struct MockSource {
    center: f64,
}

impl IqSource for MockSource {
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
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock rx source".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Run `cmds`, then return the main receiver's passband from the last state the
/// engine published.
fn filter_after(cmds: &[Command]) -> (f32, f32) {
    let mut h = start_engine(
        Box::new(MockSource { center: SSTV_20M }),
        caps(),
        EngineConfig { tx_ham_only: false, ..Default::default() },
    );
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(150));
    for c in cmds {
        h.cmd_tx.send(c.clone()).unwrap();
    }
    std::thread::sleep(Duration::from_millis(300));

    let mut last = None;
    while let Ok(ev) = h.event_rx.try_recv() {
        if let RadioEvent::State(s) = ev {
            last = Some((s.rx[0].filter_lo, s.rx[0].filter_hi));
        }
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("the engine should publish state")
}

fn tune(hz: f64) -> Command {
    Command::SetVfo { vfo: Vfo::A, hz }
}

fn sstv() -> Command {
    Command::SetMode { rx: RxId::Main, mode: Mode::Sstv }
}

#[test]
fn sstv_on_20m_is_upper_sideband() {
    let (lo, hi) = filter_after(&[tune(SSTV_20M), sstv()]);
    assert!(lo >= 0.0 && hi > 0.0, "20 m SSTV should be USB, got {lo}..{hi} Hz");
}

#[test]
fn sstv_on_40m_and_80m_is_lower_sideband() {
    for dial in [SSTV_40M, SSTV_80M, 1_890_000.0] {
        let (lo, hi) = filter_after(&[tune(dial), sstv()]);
        assert!(
            lo < 0.0 && hi <= 0.0,
            "SSTV at {:.3} MHz should be LSB, got {lo}..{hi} Hz",
            dial / 1e6
        );
    }
}

#[test]
fn tuning_across_the_boundary_flips_the_sideband() {
    // The mode is entered on 20 m and the band changed underneath it — the case
    // the mode-change path cannot answer on its own.
    let (lo, hi) = filter_after(&[tune(SSTV_20M), sstv(), tune(SSTV_40M)]);
    assert!(lo < 0.0 && hi <= 0.0, "tuning down to 40 m should have gone LSB, got {lo}..{hi} Hz");

    let (lo, hi) = filter_after(&[tune(SSTV_40M), sstv(), tune(SSTV_20M)]);
    assert!(lo >= 0.0 && hi > 0.0, "tuning up to 20 m should have gone USB, got {lo}..{hi} Hz");
}

#[test]
fn the_other_digital_modes_stay_on_usb_everywhere() {
    // Only SSTV's sideband follows the band; FT8 on 40 m is still USB.
    let (lo, hi) =
        filter_after(&[tune(7_074_000.0), Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 }]);
    assert!(lo >= 0.0 && hi > 0.0, "FT8 on 40 m should stay USB, got {lo}..{hi} Hz");
}

/// The VHF twin has no sideband at all. `Mode::SstvFm` frequency-modulates the
/// carrier, so its passband straddles the dial and nothing about it follows the
/// band it is on (issue #192).
///
/// Driven across the very boundary that flips `Mode::Sstv` above — entered on
/// 20 m, tuned down to 40 m — because that is the move a shared `is_sstv()`
/// test would get wrong, and it is one engine rather than a band's worth.
#[test]
fn sstv_on_fm_keeps_its_channel_across_the_sideband_boundary() {
    let want = Mode::SstvFm.default_filter();
    assert!(want.0 < 0.0 && want.1 > 0.0, "an FM channel straddles the dial: {want:?}");

    let sstv_fm = Command::SetMode { rx: RxId::Main, mode: Mode::SstvFm };
    let (lo, hi) = filter_after(&[tune(SSTV_20M), sstv_fm, tune(SSTV_40M)]);
    assert_eq!((lo, hi), want, "an FM channel followed a sideband rule");

    // And the table underneath says the same on every band the sideband mode
    // does mirror on, plus the VHF/UHF image channels it is actually for.
    for dial in [1_890_000.0, SSTV_80M, SSTV_40M, 50_510_000.0, 144_500_000.0, 433_400_000.0] {
        assert_eq!(Mode::SstvFm.default_filter_at(dial), want, "at {:.3} MHz", dial / 1e6);
    }
}

// ── What the rig is told, and by whom ────────────────────────────────────────

/// A front end that is the rig too, and records every mode commanded to it.
/// `resolves` is [`IqSource::resolves_sstv_sideband`] — the whole question these
/// last tests ask.
struct ModeRecordingRig {
    center: f64,
    resolves: bool,
    modes: std::sync::Arc<std::sync::Mutex<Vec<Mode>>>,
}

impl IqSource for ModeRecordingRig {
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
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock rig".into()
    }
    fn commands_rx_mode(&self) -> bool {
        true
    }
    fn resolves_sstv_sideband(&self) -> bool {
        self.resolves
    }
    fn set_control_mode(&mut self, mode: Mode) -> Result<()> {
        self.modes.lock().unwrap().push(mode);
        Ok(())
    }
}

/// Enter SSTV at `dial` and return every mode the radio was commanded.
fn modes_commanded_for_sstv(resolves: bool, dial: f64) -> Vec<Mode> {
    let modes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut h = start_engine(
        Box::new(ModeRecordingRig { center: dial, resolves, modes: std::sync::Arc::clone(&modes) }),
        caps(),
        EngineConfig { tx_ham_only: false, ..Default::default() },
    );
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(150));
    h.cmd_tx.send(tune(dial)).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    modes.lock().unwrap().clear();
    h.cmd_tx.send(sstv()).unwrap();
    std::thread::sleep(Duration::from_millis(300));

    let out = modes.lock().unwrap().clone();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    out
}

/// A radio whose control layer is a mode table with no dial in it is still sent
/// plain LSB on the bands where SSTV rides the lower sideband: the engine is the
/// only place that can work that out for it.
#[test]
fn a_rig_that_cannot_work_the_sideband_out_is_sent_lsb() {
    let seen = modes_commanded_for_sstv(false, SSTV_40M);
    assert!(seen.contains(&Mode::Lsb), "40 m SSTV should reach the rig as LSB: {seen:?}");
    let seen = modes_commanded_for_sstv(false, SSTV_20M);
    assert!(seen.contains(&Mode::Sstv), "20 m SSTV should reach the rig as SSTV: {seen:?}");
}

/// A CAT rig gets SSTV by name on *both* sidebands (issue #313). Translating it
/// to plain LSB first is what spent the operator's `digi_mode` choice and put
/// the picture on the microphone input: the mode policy at the other end takes
/// the dial and answers DIGL there.
#[test]
fn a_cat_rig_is_sent_sstv_by_name_on_either_sideband() {
    for dial in [SSTV_20M, SSTV_40M, SSTV_80M] {
        let seen = modes_commanded_for_sstv(true, dial);
        assert!(
            seen.contains(&Mode::Sstv) && !seen.contains(&Mode::Lsb),
            "at {:.3} MHz the rig should have been told SSTV, not a bare sideband: {seen:?}",
            dial / 1e6
        );
    }
}
