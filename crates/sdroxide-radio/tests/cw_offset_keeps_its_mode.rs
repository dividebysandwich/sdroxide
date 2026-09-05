//! The CW offset is the sidetone pitch, and it belongs to CW (issue #336).
//!
//! There are two memories in play and they used to be one. `DigiConfig`'s
//! `tx_audio_hz` remembers, per **band**, where in the passband a slotted mode
//! last transmitted — a figure worth keeping because a licence edge belongs to
//! the band. `cw_pitch_hz` remembers, for the **station**, the tone the
//! operator copies CW at: 700 Hz for most people, and the frequency the
//! passband is centred on as well as the one the keyer sends at.
//!
//! Letting CW into the band memory broke both. Clicking a signal in PSK wrote
//! 2069 Hz against the band; switching back to CW restored it over the pitch,
//! and the keyer went out on 2069 Hz while the operator listened at 700 —
//! outside the passband, which is what was reported. It went the other way too:
//! a pitch written against the band was handed to FT8 as a transmit offset the
//! next time that band came round.
//!
//! Its own test binary because it reads `digi.json` back — see the note in
//! `digi_tx_level_persist.rs` for why that cannot share a process.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RxId, Vfo};

/// The 15 m CW end of the band the report came from, and the PSK spot on the
/// same band that overwrote it.
const DIAL_CW: f64 = 21_028_000.0;
const DIAL_PSK: f64 = 21_076_000.0;
const RATE: f64 = 48_000.0;

/// The pitch the operator copies at, and the offset a click in PSK produced.
const PITCH_HZ: f32 = 700.0;
const PSK_HZ: f32 = 2_069.0;

struct Silent {
    center: Arc<Mutex<f64>>,
}

impl IqSource for Silent {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "silent stand-in".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(10_000.0, 60_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Wait for a digi status that satisfies `f`, and hand it back.
fn digi_settles(
    h: &sdroxide_radio::EngineHandles,
    mut f: impl FnMut(&sdroxide_types::DigiStatus) -> bool,
) -> Option<sdroxide_types::DigiStatus> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Ft8Status(st) = ev
                && f(&st)
            {
                return Some(st);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

/// The *last* status of the mode named, after everything the engine does on a
/// mode change has had time to happen.
///
/// The first one is no good here. A new controller publishes as soon as it
/// exists, and the band memory is applied on the poll after that — so the
/// figure under test arrives second, and a test that stopped at the first
/// status passed against the very bug it was written for.
fn digi_after_settling(
    h: &sdroxide_radio::EngineHandles,
    mode: Mode,
) -> sdroxide_types::DigiStatus {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut last = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Ft8Status(st) = ev
                && st.mode == mode
            {
                last = Some(st);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    last.unwrap_or_else(|| panic!("no {mode:?} status in two seconds"))
}

fn send(h: &sdroxide_radio::EngineHandles, c: Command) {
    h.cmd_tx.send(c).expect("engine gone");
    std::thread::sleep(Duration::from_millis(80));
}

#[test]
fn a_cw_pitch_survives_a_trip_through_another_mode() {
    let root = std::env::temp_dir().join(format!("sdroxide-cw-offset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // SAFETY: no engine exists yet in this process.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let center = Arc::new(Mutex::new(DIAL_CW));
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let h = start_engine(Box::new(Silent { center }), caps(), cfg);

    // CW, copying at 700 Hz.
    send(&h, Command::SetVfo { vfo: Vfo::A, hz: DIAL_CW });
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
    send(&h, Command::SetDigiAudioFreq(PITCH_HZ));
    assert!(
        digi_settles(&h, |s| (s.audio_hz - PITCH_HZ).abs() < 1.0).is_some(),
        "the pitch was not taken in the first place"
    );

    // Over to PSK on the same band, and click a signal — which is all
    // `SetDigiAudioFreq` is.
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Psk });
    send(&h, Command::SetVfo { vfo: Vfo::A, hz: DIAL_PSK });
    send(&h, Command::SetDigiAudioFreq(PSK_HZ));
    assert!(
        digi_settles(&h, |s| (s.audio_hz - PSK_HZ).abs() < 1.0).is_some(),
        "PSK did not move to the signal that was clicked"
    );

    // And back to CW. The keyer has to be where the operator is listening.
    send(&h, Command::SetVfo { vfo: Vfo::A, hz: DIAL_CW });
    send(&h, Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
    let st = digi_after_settling(&h, Mode::Cw);
    assert!(
        (st.audio_hz - PITCH_HZ).abs() < 1.0,
        "CW came back on {} Hz, not the {PITCH_HZ} Hz it was left on",
        st.audio_hz
    );

    // The band memory is for the modes that choose an offset on a band. CW's
    // pitch is not one of those and must not have been written there — the next
    // slotted mode on this band would inherit it.
    let saved = sdroxide_config::load_digi_config();
    assert_eq!(
        saved.tx_audio_hz.get(&sdroxide_types::Band::M15).copied(),
        Some(PSK_HZ),
        "the band should carry PSK's offset and nothing else: {:?}",
        saved.tx_audio_hz
    );
    // …and the pitch is on disk in its own right, so the next session starts
    // where this one left off.
    assert_eq!(saved.cw_pitch_hz, PITCH_HZ, "the pitch did not reach the station configuration");

    drop(h.cmd_tx);
    drop(h.event_rx);
    let _ = std::fs::remove_dir_all(&root);
}
