//! Starting a contest session must not rewrite the digital configuration.
//!
//! The contest logger opens from its own panel, not the digi panel, and the
//! only thing it needs to tell the engine is which FT8 contest layout to send.
//! It was saying that by pushing a whole `DigiConfig` — its own copy, with one
//! field changed — through `Command::SetDigiConfig`.
//!
//! That makes the panel's copy authoritative over the engine's, and the
//! consequence is not confined to the contest. `DigiConfig` rides whole and
//! positionally, so any field the panel's copy is stale on is rolled back to
//! whatever the panel happened to be holding. Two failures follow:
//!
//! 1. **An FT8 contest layout the operator had already set is cleared.** Every
//!    contest except EU VHF has no FT8 layout, so the logger maps all of them
//!    to `ContestMode::None`. Opening a session therefore wrote `None` over a
//!    layout that was working, silently turning the feature off — and the
//!    symptom appears in the digi panel, far from the button that caused it.
//! 2. **Every field a build adds to `DigiConfig` is exposed to the same
//!    clobber.** A panel that predates a field, or was simply not re-synced
//!    after it was added elsewhere, writes that field's default over the
//!    operator's setting. The fix is structural: a setting that needs writing
//!    one field gets a command that carries one field, so the blast radius of
//!    the write is the field.
//!
//! So these tests are about what survives, not about the command's own effect.
//! The contest mode changing is covered elsewhere; what could have been lost is
//! what is at stake.

use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, ContestMode, DeviceCaps, DigiConfig, Mode, RadioEvent, RxId};

/// Point the process at a config directory of its own, once.
fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root =
            std::env::temp_dir().join(format!("sdroxide-contest-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    });
}

/// A silent rig. Nothing here transmits; the test is about configuration.
struct SilentRig;

impl IqSource for SilentRig {
    fn sample_rate(&self) -> f64 {
        48_000.0
    }
    fn center_hz(&self) -> f64 {
        14_090_000.0
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
        "silent mock rig".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "silent mock rig".into(),
        rx_channels: 1,
        tx_channels: 0,
        freq_ranges_rx: vec![(10_000.0, 148_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The configuration an operator might have set up before a contest: their
/// call, a grid, an FT8 contest layout, and a couple of ordinary settings.
///
/// Distinctive values rather than defaults, because the failure being pinned is
/// a field being written back to its default. A default would survive the
/// clobber and the test would pass for the wrong reason.
fn operators_station() -> DigiConfig {
    DigiConfig {
        my_call: "OE1XYZ".into(),
        my_grid: "JN88".into(),
        contest: ContestMode::EuVhf,
        // A field a newer build adds. On `main` this is `ft8_depth`, and with
        // FST4W landed also `fst4w_period`; on this branch the FSK441 period
        // stands in for the same thing — a setting the engine holds and a panel
        // copy written before it existed cannot carry.
        fsk441_period: sdroxide_types::Fsk441Period::P30,
        tx_audio_level_ssb: 0.42,
        msg_cq: "cq oe1xyz oe1xyz".into(),
        ..DigiConfig::default()
    }
}

/// Run `cmds` against an engine already holding `start`, and report the
/// `DigiConfig` the engine echoes back last.
fn config_after(start: DigiConfig, cmds: Vec<Command>) -> DigiConfig {
    isolate_config();
    let mut h = start_engine(Box::new(SilentRig), caps(), EngineConfig::default());
    let thread = h.thread.take();
    std::thread::sleep(Duration::from_millis(200));
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 }).expect("engine gone");
    std::thread::sleep(Duration::from_millis(80));
    h.cmd_tx.send(Command::SetDigiConfig(start)).expect("engine gone");
    std::thread::sleep(Duration::from_millis(80));
    for c in cmds {
        h.cmd_tx.send(c).expect("engine gone");
        std::thread::sleep(Duration::from_millis(80));
    }

    // Drain for a fixed window and keep the *last* status, not the first: the
    // engine emits one as it comes up, before any of these commands land, and
    // taking that would compare the operator's settings against defaults and
    // blame the code for the startup echo.
    let mut last = None;
    let deadline = std::time::Instant::now() + Duration::from_millis(600);
    while std::time::Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Ft8Status(s) = ev {
                last = Some(s.config);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h.cmd_tx);
    drop(h.event_rx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("the engine never echoed a DigiStatus carrying a config")
}

/// The bug. Opening a contest session that has no FT8 layout of its own writes
/// `ContestMode::None`, because that is the honest answer to "which layout
/// should the FT8 side send for this contest" — and it must write *only* that.
///
/// The layout the operator had set beforehand survives. Everything else in the
/// configuration survives too, which is the general form of the same fix: this
/// command cannot carry a field it does not name, so it cannot roll one back.
#[test]
fn starting_a_session_leaves_the_ft8_contest_layout_and_the_rest_alone() {
    let after = config_after(operators_station(), vec![Command::SetDigiContest(ContestMode::None)]);

    assert_eq!(
        after.contest,
        ContestMode::None,
        "the command's own effect did not land — this test is not measuring what it thinks"
    );

    let before = operators_station();
    for (what, got, want) in [
        ("my_call", &after.my_call, &before.my_call),
        ("my_grid", &after.my_grid, &before.my_grid),
        ("msg_cq", &after.msg_cq, &before.msg_cq),
    ] {
        assert_eq!(got, want, "{what} was rolled back to {:?} by a one-field write", got);
    }
    assert_eq!(
        after.fsk441_period,
        sdroxide_types::Fsk441Period::P30,
        "the FSK441 period was reset by a command that carries a contest mode"
    );
    assert!(
        (after.tx_audio_level_ssb - 0.42).abs() < 1e-6,
        "the sideband transmit level went to {} — a mode-agnostic whole-config write again",
        after.tx_audio_level_ssb
    );
}

/// And the other direction: a session that *does* have an FT8 layout sets it,
/// still without touching anything else. The first test proves the command does
/// not clobber; this one proves it is not inert.
#[test]
fn a_session_with_an_ft8_layout_sets_it_and_touches_nothing_else() {
    let after =
        config_after(operators_station(), vec![Command::SetDigiContest(ContestMode::EuVhf)]);
    assert_eq!(after.contest, ContestMode::EuVhf);
    assert_eq!(after.my_call, "OE1XYZ");
    assert_eq!(after.fsk441_period, sdroxide_types::Fsk441Period::P30);
}

/// Why the narrow command is not a stylistic preference.
///
/// This is the route the contest panel used, and it is kept as a test because
/// `Command::SetDigiConfig` will keep working forever — it is how every other
/// panel writes its settings — so nothing about the type stops anyone reaching
/// for it again. A panel's copy goes stale the moment anything else writes the
/// configuration, and writing it back rolls every field it is stale on back to
/// whatever the panel happened to be holding.
///
/// Asserting the loss is the point: if this ever stops holding, `SetDigiConfig`
/// has learned to merge rather than replace, and the narrow command is no
/// longer load-bearing — at which point this test is telling the truth in the
/// other direction and should be re-read, not deleted.
#[test]
fn a_whole_config_write_from_a_stale_copy_is_what_the_narrow_command_exists_to_avoid() {
    let stale = DigiConfig::default();
    let after = config_after(
        operators_station(),
        // Exactly what the panel sent: its own copy, with one field changed.
        vec![Command::SetDigiConfig(DigiConfig { contest: ContestMode::None, ..stale })],
    );

    assert_eq!(after.contest, ContestMode::None);
    assert_eq!(
        after.my_call, "",
        "a stale whole-config write stopped losing fields — re-read this test before assuming \
         the narrow command still matters"
    );
    assert_eq!(
        after.fsk441_period,
        sdroxide_types::Fsk441Period::default(),
        "the stale write stopped rolling back the FSK441 period"
    );
}
