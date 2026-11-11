//! Issue #762: a band left without a band button — a digital mode's band chip
//! tunes straight to the next band's calling frequency with a dial move — has
//! to be recorded on its stack all the same. It was not, so 6 m left in FT8
//! came back on whatever older entry its stack held.
//!
//! One test function on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! this one writes a real `bandstacks.json` under it.

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Band, Command, DeviceCaps, Mode, RadioEvent, RadioState, RxId, Vfo};

const RATE: f64 = 192_000.0;

struct Rx {
    center: f64,
}

impl IqSource for Rx {
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
        let n = buf.len().min(1024);
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
        freq_ranges_rx: vec![(1_000_000.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Drain states until one satisfies `pred`; each predicate below can only be
/// met by the command just sent, so no stale state stands in for it.
fn wait_for(
    rx: &crossbeam_channel::Receiver<RadioEvent>,
    what: &str,
    pred: impl Fn(&RadioState) -> bool,
) -> RadioState {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(RadioEvent::State(s)) = rx.recv_timeout(Duration::from_millis(100))
            && pred(&s)
        {
            return s;
        }
    }
    panic!("never saw: {what}");
}

#[test]
fn a_band_left_by_a_dial_move_is_recorded_on_its_stack_and_recalled_only_in_its_mode() {
    let root = std::env::temp_dir().join(format!("sdroxide-band-stack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let mut h = start_engine(
        Box::new(Rx { center: 14_200_000.0 }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // 6 m in FT8, on its calling frequency.
    send(Command::SetVfo { vfo: Vfo::A, hz: 50_313_000.0 });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 });
    wait_for(&h.event_rx, "6 m FT8", |s| {
        s.rx[0].mode == Mode::Ft8 && (s.vfo_a_hz - 50_313_000.0).abs() < 1.0
    });

    // The FT8 band chip for 2 m: a dial move, not a band change.
    send(Command::SetVfo { vfo: Vfo::A, hz: 144_174_000.0 });
    wait_for(&h.event_rx, "2 m FT8", |s| (s.vfo_a_hz - 144_174_000.0).abs() < 1.0);

    // Still in FT8, the 6 m band button: 6 m comes back where it was left.
    send(Command::SetBand(Band::M6));
    let s = wait_for(&h.event_rx, "back on 6 m", |s| s.band == Band::M6);
    assert!(
        (s.vfo_a_hz - 50_313_000.0).abs() < 1.0 && s.rx[0].mode == Mode::Ft8,
        "6 m recalled {} Hz {:?}, not 50.313 MHz FT8",
        s.vfo_a_hz,
        s.rx[0].mode
    );

    // Out of FT8 on 2 m, then the 6 m band button: a band button keeps the
    // operator out of the digital modes, as tuning out of a band does. The
    // FT8 entry is passed over and 6 m opens on its own default.
    send(Command::SetBand(Band::M2));
    wait_for(&h.event_rx, "on 2 m", |s| s.band == Band::M2);
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    wait_for(&h.event_rx, "2 m NFM", |s| s.rx[0].mode == Mode::Nfm);
    send(Command::SetBand(Band::M6));
    let s = wait_for(&h.event_rx, "6 m out of FT8", |s| s.band == Band::M6);
    let (default_hz, default_mode) = Band::M6.default_entry();
    assert!(
        (s.vfo_a_hz - default_hz).abs() < 1.0 && s.rx[0].mode == default_mode,
        "6 m recalled {} Hz {:?}, not its default {default_hz} Hz {default_mode:?}",
        s.vfo_a_hz,
        s.rx[0].mode
    );

    // A second press cycles the 6 m stack and still finds no FT8 on it.
    send(Command::SetBand(Band::M6));
    std::thread::sleep(Duration::from_millis(300));
    let s = wait_for(&h.event_rx, "6 m still voice", |s| s.band == Band::M6);
    assert_ne!(s.rx[0].mode, Mode::Ft8, "the second press recalled FT8");

    drop(h);
    let _ = thread.map(|t| t.join());
    let _ = std::fs::remove_dir_all(&root);
}
