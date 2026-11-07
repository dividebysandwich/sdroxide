//! Issue #762: a band left without a band button — a digital mode's band chip
//! tunes straight to the next band's calling frequency with a dial move — has
//! to be recorded on its stack all the same. It was not, so 6 m left in FT8
//! came back on whatever older entry its stack held.
//!
//! One test function on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! this one writes a real `bandstacks.json` under it.

use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{
    Band, BandStackEntry, Command, DeviceCaps, Mode, RadioEvent, RadioState, RxId, Vfo,
};

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
fn a_band_left_by_a_dial_move_is_recorded_on_its_stack() {
    let root = std::env::temp_dir().join(format!("sdroxide-band-stack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let entry = |freq_hz, mode| {
        let (filter_lo, filter_hi) = Mode::default_filter(mode);
        BandStackEntry { freq_hz, mode, filter_lo, filter_hi }
    };
    let mut legacy = sdroxide_config::BandStacks::default();
    legacy.insert(
        Band::M6,
        vec![
            entry(50_081_530.0, Mode::Cw),
            entry(50_125_000.0, Mode::Usb),
            entry(50_313_000.0, Mode::Ft8),
        ],
    );
    sdroxide_config::save_bandstacks(&legacy).unwrap();
    let store = sdroxide_config::Store::radio(5);
    let other = sdroxide_config::Store::radio(0);
    assert_eq!(store.load_bandstacks(), legacy);
    assert_eq!(other.load_bandstacks(), legacy);

    let mut h = start_engine(
        Box::new(Rx { center: 14_200_000.0 }),
        caps(),
        EngineConfig { remember_session: false, store: store.clone(), ..Default::default() },
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

    // Out of FT8 on 2 m, then the 6 m band button: 6 m comes back where it
    // was left.
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    send(Command::SetBand(Band::M6));
    let s = wait_for(&h.event_rx, "back on 6 m", |s| s.band == Band::M6);
    assert!(
        (s.vfo_a_hz - 50_313_000.0).abs() < 1.0 && s.rx[0].mode == Mode::Ft8,
        "6 m recalled {} Hz {:?}, not 50.313 MHz FT8",
        s.vfo_a_hz,
        s.rx[0].mode
    );

    // FT8 must not intercept SetBand. Edit the recalled register, then cycle
    // through USB and CW and back to the edited FT8 register.
    send(Command::SetVfo { vfo: Vfo::A, hz: 50_320_000.0 });
    send(Command::SetFilter { rx: RxId::Main, lo: 200.0, hi: 2800.0 });
    send(Command::SetBand(Band::M6));
    wait_for(&h.event_rx, "6 m CW stack", |s| {
        s.vfo_a_hz == 50_081_530.0 && s.rx[0].mode == Mode::Cw
    });
    send(Command::SetVfo { vfo: Vfo::A, hz: 50_090_000.0 });
    send(Command::SetBand(Band::M6));
    wait_for(&h.event_rx, "6 m USB stack", |s| {
        s.vfo_a_hz == 50_125_000.0 && s.rx[0].mode == Mode::Usb
    });
    send(Command::SetBand(Band::M6));
    wait_for(&h.event_rx, "edited FT8 stack", |s| {
        s.vfo_a_hz == 50_320_000.0
            && s.rx[0].mode == Mode::Ft8
            && s.rx[0].filter_lo == 200.0
            && s.rx[0].filter_hi == 2800.0
    });
    send(Command::SetBand(Band::M2));
    wait_for(&h.event_rx, "leave edited 6 m", |s| s.band == Band::M2);
    send(Command::SetBand(Band::M6));
    wait_for(&h.event_rx, "edited 6 m recalled from FT8", |s| {
        s.vfo_a_hz == 50_320_000.0 && s.rx[0].mode == Mode::Ft8
    });
    drop(h);
    let _ = thread.map(|t| t.join());
    let saved = store.load_bandstacks();
    assert_eq!(saved[&Band::M6][0].freq_hz, 50_320_000.0);
    assert!(saved[&Band::M6].iter().any(|e| e.freq_hz == 50_090_000.0));
    assert!(!saved[&Band::M6].iter().any(|e| e.freq_hz == 50_081_530.0));
    assert_eq!(other.load_bandstacks(), legacy, "other radios must not change");
    assert_eq!(sdroxide_config::load_bandstacks(), legacy, "legacy file stays intact");
    let mut h = start_engine(
        Box::new(Rx { center: 144_174_000.0 }),
        caps(),
        EngineConfig { remember_session: false, store, ..Default::default() },
    );
    let thread = h.thread.take();
    h.cmd_tx.send(Command::SetBand(Band::M6)).unwrap();
    wait_for(&h.event_rx, "6 m survives restart", |s| {
        s.vfo_a_hz == 50_320_000.0 && s.rx[0].mode == Mode::Ft8
    });
    drop(h);
    let _ = thread.map(|t| t.join());
    let _ = std::fs::remove_dir_all(&root);
}
