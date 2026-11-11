//! Switching between the aviation modes and the radiosonde channel with the
//! sonde decoder on must neither hang nor panic the engine, and tuning to a
//! sonde has to be able to leave ADS-B for its own channel.
//!
//! In the fork this came from, tuning to a sonde from ADS-B used to send only a
//! retune: the radio stayed in ADS-B, whose lane holds the front end on
//! 1090 MHz, and going back to ADS-B / HFDL / VDL2 with the sonde lane still
//! mixing from a window that no longer held its channel froze the program.

use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RxId, SondeSettings, Vfo};

const RATE: f64 = 2_500_000.0;

/// A Pluto-like front end: 2.5 Msps, tunes anywhere, noise-free.
struct Rig {
    center: f64,
}

impl IqSource for Rig {
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
        let n = buf.len().min(8192);
        buf[..n].fill(Complex32::new(0.01, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock Pluto".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench front end".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(70_000_000.0, 6_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

struct Run {
    h: sdroxide_radio::EngineHandles,
    _audio: rtrb::Consumer<f32>,
    mode: Mode,
    vfo_hz: f64,
    states: u64,
}

/// An engine test that saves anything (`SetSondeConfig` writes `sonde.json`)
/// writes the operator's real config directory unless this is set.
fn isolate_config() {
    let root = std::env::temp_dir().join(format!("sdroxide-sonde-switch-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
}

impl Run {
    fn new() -> Self {
        isolate_config();
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(1 << 16);
        let h = start_engine(
            Box::new(Rig { center: 100_000_000.0 }),
            caps(),
            EngineConfig {
                remember_session: false,
                audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
                ..Default::default()
            },
        );
        Self { h, _audio: consumer, mode: Mode::Usb, vfo_hz: 0.0, states: 0 }
    }

    fn send(&self, c: Command) {
        self.h.cmd_tx.send(c).unwrap();
    }

    /// Let the engine run for `ms` and fail if it stops publishing its state.
    fn settle(&mut self, ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let before = self.states;
        while Instant::now() < deadline {
            while let Ok(ev) = self.h.event_rx.try_recv() {
                if let RadioEvent::State(s) = ev {
                    self.mode = s.rx[0].mode;
                    self.vfo_hz = s.active_freq_hz();
                    self.states += 1;
                }
            }
            if self._audio.slots() > 0 {
                while self._audio.pop().is_ok() {}
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            self.states > before || ms < 400,
            "the engine published nothing for {ms} ms: it is stuck"
        );
        if let Some(t) = self.h.thread.as_ref() {
            assert!(!t.is_finished(), "the engine thread has died (panic)");
        }
    }
}

#[test]
fn aviation_modes_and_sonde_can_follow_each_other() {
    let mut r = Run::new();
    r.settle(500);
    let sonde = SondeSettings { enabled: true, frequency_hz: 403_000_000.0, scan: true };
    r.send(Command::SetSondeConfig(sonde));
    r.send(Command::SetMode { rx: RxId::Main, mode: Mode::Adsb });
    r.settle(600);
    assert_eq!(r.mode, Mode::Adsb);

    // Tuning to a sonde from ADS-B: leave the mode, then go to the channel.
    r.send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    r.send(Command::SetVfo { vfo: Vfo::A, hz: 403_000_000.0 });
    r.settle(600);
    assert_ne!(r.mode, Mode::Adsb, "SONDE left the radio in ADS-B");
    assert!((r.vfo_hz - 403_000_000.0).abs() < 1.0, "the dial stayed on {}", r.vfo_hz);

    for mode in [Mode::Adsb, Mode::Hfdl, Mode::Vdl2, Mode::Adsb, Mode::Acars] {
        r.send(Command::SetMode { rx: RxId::Main, mode });
        r.settle(600);
        assert_eq!(r.mode, mode);
        r.send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
        r.send(Command::SetVfo { vfo: Vfo::A, hz: 403_000_000.0 });
        r.settle(600);
        assert!((r.vfo_hz - 403_000_000.0).abs() < 1.0, "after {mode:?} the dial is {}", r.vfo_hz);
    }

    let thread = r.h.thread.take();
    drop(r.h.cmd_tx);
    if let Some(t) = thread {
        t.join().expect("the engine thread panicked");
    }
}

/// Listen-mode switches between the aviation modes and plain retunes onto the
/// sonde channel without leaving the mode, with the sonde decoder running
/// throughout.
#[test]
fn direct_hops_between_the_aviation_modes_do_not_freeze_the_engine() {
    let mut r = Run::new();
    r.settle(500);
    r.send(Command::SetSondeConfig(SondeSettings {
        enabled: true,
        frequency_hz: 403_000_000.0,
        scan: true,
    }));
    for step in 0..3 {
        for mode in [Mode::Adsb, Mode::Hfdl, Mode::Vdl2, Mode::Adsb] {
            r.send(Command::SetMode { rx: RxId::Main, mode });
            r.settle(500);
            assert_eq!(r.mode, mode, "round {step}");
        }
        // The sonde channel without leaving the mode.
        r.send(Command::SetVfo { vfo: Vfo::A, hz: 403_000_000.0 });
        r.settle(500);
    }
    let thread = r.h.thread.take();
    drop(r.h.cmd_tx);
    if let Some(t) = thread {
        t.join().expect("the engine thread panicked");
    }
}
