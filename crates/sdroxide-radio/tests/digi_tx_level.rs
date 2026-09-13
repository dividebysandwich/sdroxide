//! What level a digital over reaches the radio at.
//!
//! Field report ([issue #131]): a Kenwood on its I/Q output made 100 % of its
//! power under TUNE and a quarter of it on FT8, with the Drive slider moving
//! nothing. The rig modulates the audio we send it, and the modems synthesise
//! their modulating signal at half scale — 6 dB of headroom that belongs to the
//! modem's own arithmetic, not to the transmitter. Nothing divided it out
//! again, so the radio was asked for a quarter of the power a TUNE at the same
//! slider setting asks for, and no power command could make up the difference:
//! the output was riding on the audio, not on the power register.
//!
//! So the two levels are compared against each other rather than against a
//! constant. Whatever a tune tone is worth to a rig, a digital over has to be
//! worth the same.
//!
//! [issue #131]: https://github.com/dividebysandwich/sdroxide/issues/131

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, DigiConfig, Mode, RxId};

const DIAL_HZ: f64 = 14_090_000.0;

/// Point the whole process at a config directory of its own, once.
///
/// `SDROXIDE_CONFIG_DIR` is process-global, and an engine that comes up without
/// it reads the *operator's* live configuration — including the transmit-audio
/// level, which is one of the two numbers this file multiplies together. A
/// station that had turned its data input down to 40 % for FM packet failed
/// this test at 0.4 against the tune tone's 1.0, and the failure looked exactly
/// like the regression it exists to catch. It was measuring the operator's
/// settings, not the code.
fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root =
            std::env::temp_dir().join(format!("sdroxide-digi-tx-level-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    });
}

/// The station this test transmits from: stock, and explicitly at a full-scale
/// data input.
///
/// Sent rather than left to the defaults because it is the premise of the
/// comparison and not an incidental setting. The transmit-audio levels are the
/// operator's own, into a radio that modulates what we send it — deviation on
/// FM, drive into the modulator on sideband — and a digital over only reaches
/// the rig at the level a tune does when the one in play is wide open. Turned
/// down, the over is *supposed* to arrive quieter, which
/// [`the_sideband_level_is_the_one_a_data_mode_uses`] and
/// `aprs_tx::the_transmit_audio_level_scales_what_the_radio_is_given` pin from
/// the other side.
fn full_scale_station() -> DigiConfig {
    DigiConfig { tx_audio_level_fm: 1.0, tx_audio_level_ssb: 1.0, ..DigiConfig::default() }
}

/// The loudest sample the rig's sound card was given, and how many blocks it
/// took.
#[derive(Default)]
struct Heard {
    peak: f32,
    blocks: usize,
}

/// A CAT rig on a sound card: it modulates the audio we hand it and has its own
/// power control, which is what nearly every rig this backend drives looks
/// like — a Kenwood among them.
struct SoundCardRig {
    heard: Arc<Mutex<Heard>>,
}

impl IqSource for SoundCardRig {
    fn sample_rate(&self) -> f64 {
        48_000.0
    }
    fn center_hz(&self) -> f64 {
        DIAL_HZ
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn center_is_dial(&self) -> bool {
        true
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock CAT rig on a sound card".into()
    }
    /// The rig's power register is the level control; the audio is only the
    /// modulating signal. Exactly the case the report came from.
    fn commands_tx_power(&self) -> bool {
        true
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        let mut h = self.heard.lock().unwrap();
        h.blocks += 1;
        for &a in audio {
            h.peak = h.peak.max(a.abs());
        }
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock-cat".into(),
        label: "mock CAT rig".into(),
        rx_channels: 1,
        tx_channels: 1,
        // Quadrature receive, audio transmit — the shape of the rig in the
        // report.
        audio_mode: false,
        tx_audio: true,
        freq_ranges_rx: vec![(10_000.0, 148_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Run an over built by `cmds` and report the loudest audio the rig was given.
/// Gives up once the transmission is plainly under way, since the level is
/// settled by then and a keyboard mode would otherwise send until stopped.
fn peak_of(cmds: Vec<Command>) -> f32 {
    isolate_config();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let src = SoundCardRig { heard: Arc::clone(&heard) };
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let mut h = start_engine(Box::new(src), caps(), cfg);
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(200));
    for c in std::iter::once(Command::SetDigiConfig(full_scale_station())).chain(cmds) {
        h.cmd_tx.send(c).expect("engine gone");
        std::thread::sleep(Duration::from_millis(60));
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while h.event_rx.try_recv().is_ok() {}
        if heard.lock().unwrap().blocks > 20 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = std::mem::take(&mut *heard.lock().unwrap());

    drop(h.cmd_tx);
    drop(h.event_rx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    assert!(out.blocks > 0, "nothing ever reached the rig's sound card");
    out.peak
}

/// The fix. A keyboard-mode over and a tune tone are both modulating signals
/// for the same transmitter, and they arrive at the same level; before this the
/// over arrived 6 dB down, which is the quarter of the power the report
/// measured.
#[test]
fn a_digital_over_reaches_the_rig_at_the_level_a_tune_does() {
    let tune = peak_of(vec![Command::SetTune(true)]);
    let over = peak_of(vec![
        Command::SetMode { rx: RxId::Main, mode: Mode::Rtty },
        Command::DigiTxActive(true),
        Command::DigiTxText("cq cq de w1aw w1aw k".into()),
    ]);
    assert!(tune > 0.9, "the tune tone is full scale: {tune}");
    assert!(
        (over - tune).abs() < 0.1,
        "a digital over is {over} against the tune tone's {tune} — the rig is being asked for \
         {:.0}% of the power",
        (over / tune).powi(2) * 100.0
    );
}

/// Issue #419: TUNE in a data mode is how an operator sets the transmit-audio
/// level against the rig's ALC, so the tune tone follows that mode's level. It
/// used to go out at full scale whatever the slider said, and the slider only
/// did anything once a real over started.
///
/// `Ft4` is used because nothing else in this file writes its level.
#[test]
fn a_tune_in_a_data_mode_goes_out_at_that_modes_level() {
    let tune = peak_of(vec![
        Command::SetDigiTxLevel { mode: Mode::Ft4, level: 0.3 },
        Command::SetMode { rx: RxId::Main, mode: Mode::Ft4 },
        Command::SetTune(true),
    ]);
    assert!((tune - 0.3).abs() < 0.03, "a tune in FT4 at a 0.3 level reached the rig at {tune}");
}

/// RTTY is audio on a sideband, so it takes the sideband level and never the FM
/// one.
///
/// The two were a single number until a station set 40 % for 1200 baud packet
/// deviation and found it had taken 8 dB off their FT8 as well — issue #131's
/// symptom by another road, and invisible, because the control was only drawn
/// in the APRS panel. So the level that applies follows the carrier the mode
/// goes out on: this is the sideband end of it, and
/// `aprs_tx::the_transmit_audio_level_scales_what_the_radio_is_given` is the FM
/// end.
#[test]
fn the_sideband_level_is_the_one_a_data_mode_uses() {
    let rtty_at = |fm: f32, ssb: f32| {
        peak_of(vec![
            Command::SetDigiConfig(DigiConfig {
                tx_audio_level_fm: fm,
                tx_audio_level_ssb: ssb,
                ..DigiConfig::default()
            }),
            Command::SetMode { rx: RxId::Main, mode: Mode::Rtty },
            Command::DigiTxActive(true),
            Command::DigiTxText("cq cq de w1aw w1aw k".into()),
        ])
    };
    // Turning the *FM* level down does nothing to a sideband over...
    let fm_down = rtty_at(0.4, 1.0);
    assert!(fm_down > 0.9, "an FM deviation setting reached an SSB over: {fm_down}");
    // ...and turning the sideband level down scales it, because on sideband
    // this is the only drive control there is.
    let ssb_down = rtty_at(1.0, 0.5);
    assert!(
        (ssb_down / fm_down - 0.5).abs() < 0.08,
        "half the sideband level should be half the amplitude: {ssb_down} against {fm_down}"
    );
}

// ── one level per mode (issue #186) ─────────────────────────────────────────
//
// A note on why these pick the modes they pick. `SetDigiTxLevel` *persists*,
// and `isolate_config` gives the whole process one config directory — so an
// entry written by one test is still in `digi.json` when the next one's engine
// loads it, and there is no command that clears one again. Every measurement
// below therefore sets the level it is measuring, rather than relying on a mode
// having none; the single test that genuinely needs an absent entry owns a mode
// (`Thor`) that nothing else here writes, as `the_sideband_level_is_the_one_a_
// data_mode_uses` owns `Rtty`.

/// Issue #186, reduced to an assertion: a level set for one mode applies to
/// that mode and to nothing else.
///
/// The report is from a TS-590SG operator who found FT8, RTTY, MCW and PSK each
/// wanting a different figure into the same rig, and one number serving all of
/// them. What is being set is where the waveform sits against the radio's ALC,
/// and that is a property of the waveform.
///
/// Both halves are asserted, because either alone is worthless: that PSK
/// followed would pass on a build that scaled every mode together, and that
/// Olivia held would pass on one where the per-mode level did nothing at all.
/// Two continuous modes on the same carrier, so the only thing that can
/// separate them is the map.
#[test]
fn a_level_set_for_one_mode_applies_to_it_and_stays_there() {
    // One over, with both modes' levels stated outright.
    let over = |mode: Mode, psk: f32, olivia: f32| {
        peak_of(vec![
            Command::SetDigiConfig(full_scale_station()),
            Command::SetDigiTxLevel { mode: Mode::Psk, level: psk },
            Command::SetDigiTxLevel { mode: Mode::Olivia, level: olivia },
            Command::SetMode { rx: RxId::Main, mode },
            Command::DigiTxActive(true),
            Command::DigiTxText("cq cq de w1aw w1aw k".into()),
        ])
    };
    let (psk_open, olivia_open) = (over(Mode::Psk, 1.0, 1.0), over(Mode::Olivia, 1.0, 1.0));
    assert!(psk_open > 0.9, "PSK did not start wide open: {psk_open}");
    assert!(olivia_open > 0.9, "Olivia did not start wide open: {olivia_open}");

    // Turn PSK down. PSK follows...
    let psk_down = over(Mode::Psk, 0.5, 1.0);
    assert!(
        (psk_down / psk_open - 0.5).abs() < 0.08,
        "PSK's own level did not reach its over: {psk_down} against {psk_open}"
    );
    // ...and Olivia, on the same carrier, does not.
    let olivia_held = over(Mode::Olivia, 0.5, 1.0);
    assert!(
        (olivia_held / olivia_open - 1.0).abs() < 0.08,
        "a PSK level reached an Olivia over: {olivia_held} against {olivia_open}"
    );
}

/// A mode the operator has never set takes its carrier's level, rather than
/// springing back to full scale.
///
/// The property the whole shape rests on. It is what lets the map arrive
/// without a migration — an empty one is exactly the old behaviour — and what
/// stops a mode appended in a later release from putting a station's first over
/// on the air at full scale into a rig set 8 dB lower.
///
/// THOR, because nothing else in this file writes a THOR entry: this is the one
/// test here that measures an absence.
#[test]
fn a_mode_with_no_level_of_its_own_takes_the_carrier_default() {
    let thor_at = |ssb: f32| {
        peak_of(vec![
            Command::SetDigiConfig(DigiConfig { tx_audio_level_ssb: ssb, ..DigiConfig::default() }),
            Command::SetMode { rx: RxId::Main, mode: Mode::Thor },
            Command::DigiTxActive(true),
            Command::DigiTxText("cq cq de w1aw w1aw k".into()),
        ])
    };
    let open = thor_at(1.0);
    let half = thor_at(0.5);
    assert!(open > 0.9, "the carrier default did not reach a mode with no entry: {open}");
    assert!(
        (half / open - 0.5).abs() < 0.08,
        "half the carrier default should be half the amplitude: {half} against {open}"
    );
}

/// Where a mode does have an entry, that entry is what transmits — the carrier
/// default underneath it is not consulted.
#[test]
fn a_mode_with_a_level_of_its_own_transmits_at_it() {
    let psk_at = |ssb: f32, own: f32| {
        peak_of(vec![
            Command::SetDigiConfig(DigiConfig { tx_audio_level_ssb: ssb, ..DigiConfig::default() }),
            Command::SetDigiTxLevel { mode: Mode::Psk, level: own },
            Command::SetMode { rx: RxId::Main, mode: Mode::Psk },
            Command::DigiTxActive(true),
            Command::DigiTxText("cq cq de w1aw w1aw k".into()),
        ])
    };
    let open = psk_at(1.0, 1.0);
    let own_down = psk_at(1.0, 0.5);
    assert!(
        (own_down / open - 0.5).abs() < 0.08,
        "PSK's own level did not override the carrier default: {own_down} against {open}"
    );
    // The other way round: a wide-open entry over a turned-down default.
    let own_over_quiet_default = psk_at(0.25, 1.0);
    assert!(
        own_over_quiet_default > 0.9,
        "the carrier default was still being applied under an entry: {own_over_quiet_default}"
    );
}

/// The floor leaves a transmitter that radiates.
///
/// A rail dragged to the bottom must not key a dead transmitter: that looks
/// exactly like a broken rig, and is diagnosed by everything except the control
/// that caused it.
#[test]
fn the_level_floor_still_puts_something_on_the_air() {
    let peak = peak_of(vec![
        Command::SetDigiConfig(full_scale_station()),
        Command::SetDigiTxLevel { mode: Mode::Psk, level: 0.0 },
        Command::SetMode { rx: RxId::Main, mode: Mode::Psk },
        Command::DigiTxActive(true),
        Command::DigiTxText("cq cq de w1aw w1aw k".into()),
    ]);
    assert!(peak > 0.0, "the transmitter went silent at the bottom of the rail");
    assert!(peak < 0.1, "the floor is not attenuating: {peak}");
}

// ── the same over on a radio we modulate ourselves ───────────────────────────

/// An I/Q transmitter: no sound card, no power register of its own, so Drive is
/// the level and the modulated baseband is what goes on the air.
struct IqRadio {
    heard: Arc<Mutex<Heard>>,
}

impl IqSource for IqRadio {
    fn sample_rate(&self) -> f64 {
        48_000.0
    }
    fn center_hz(&self) -> f64 {
        DIAL_HZ
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
        "mock I/Q transmitter".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, iq: &[Complex32]) -> Result<()> {
        let mut h = self.heard.lock().unwrap();
        h.blocks += 1;
        for z in iq {
            h.peak = h.peak.max(z.norm());
        }
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

/// As [`peak_of`], for the modulated-I/Q path: the strongest baseband magnitude
/// the converter was handed.
fn iq_peak_of(cmds: Vec<Command>) -> f32 {
    isolate_config();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let src = IqRadio { heard: Arc::clone(&heard) };
    let cfg = EngineConfig { tx_ham_only: false, ..Default::default() };
    let mut h = start_engine(Box::new(src), DeviceCaps { tx_audio: false, ..caps() }, cfg);
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(200));
    for c in cmds {
        h.cmd_tx.send(c).expect("engine gone");
        std::thread::sleep(Duration::from_millis(60));
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while h.event_rx.try_recv().is_ok() {}
        if heard.lock().unwrap().blocks > 20 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = std::mem::take(&mut *heard.lock().unwrap());

    drop(h.cmd_tx);
    drop(h.event_rx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    assert!(out.blocks > 0, "nothing was ever transmitted");
    out.peak
}

/// On a radio sdroxide modulates itself, Drive is the level — so a hundred
/// percent of it has to be a hundred percent of the transmitter, the same as a
/// tune at a hundred percent. The modem's headroom used to eat two thirds of
/// the power the slider was promising, with nothing to show it.
#[test]
fn drive_spends_the_whole_transmitter_on_a_digital_over() {
    let tune = iq_peak_of(vec![Command::SetTuneDrive(1.0), Command::SetTune(true)]);
    let over = iq_peak_of(vec![
        Command::SetTxDrive(1.0),
        Command::SetMode { rx: RxId::Main, mode: Mode::Rtty },
        Command::DigiTxActive(true),
        Command::DigiTxText("cq cq de w1aw w1aw k".into()),
    ]);
    assert!(tune > 0.9, "a full tune is a full-scale carrier: {tune}");
    assert!(
        (over - tune).abs() < 0.1,
        "an over at full Drive is {over} against the full tune's {tune} — {:.0}% of the power",
        (over / tune).powi(2) * 100.0
    );
}
