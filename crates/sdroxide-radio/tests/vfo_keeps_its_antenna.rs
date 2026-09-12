//! Each VFO remembers the antenna socket it was left on.
//!
//! The per-band memory cannot stand in for this. It holds one socket per band,
//! so an RSPdx operator listening on Antenna A with VFO A and on Antenna B with
//! VFO B — both at the same end of the same band — writes both choices into the
//! same entry, and the second one wins for both. Switching back gave the
//! frequency and the mode correctly and left the front end on the wrong socket
//! (issue #404).
//!
//! The antenna rides in the same shelf as the mode and the passband, written on
//! the way out of a VFO and read on the way into one, so a swap and an A=B copy
//! carry it for free.
//!
//! The two memories divide the work by what the operator just did. An A/B press
//! that stays inside one band is the VFO's: the band has one socket and this is
//! two. Crossing a band edge — by the dial, or by an A/B press onto a VFO parked
//! on another band — is the band's, because which aerial hears 2 m is a fact
//! about the station rather than about a VFO. Choosing a socket by hand writes
//! the band's entry and nothing else does.

use std::time::Duration;

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, Mode, RadioEvent, RadioState, RxId, Vfo};

const RATE: f64 = 2_400_000.0;

/// A front end with two sockets, like an RSPdx's Antenna A and Antenna B.
struct MockSource {
    center: f64,
    antenna: String,
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
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.antenna = name.to_string();
        Ok(())
    }
    fn current_antenna(&self) -> String {
        self.antenna.clone()
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock two-socket source".into()
    }
}

/// Point the config at a scratch directory, once for the whole binary — the
/// engine reads and seeds files as it starts, and no test may write to the
/// operator's own.
fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let root = std::env::temp_dir().join(format!("sdroxide-vfo-ant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    });
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        antennas_rx: vec!["Antenna A".into(), "Antenna B".into()],
        ..DeviceCaps::default()
    }
}

/// Run `cmds` against a fresh engine and report the last state it published.
fn after(cmds: &[Command]) -> RadioState {
    isolate_config();
    let mut h = start_engine(
        Box::new(MockSource { center: 14_200_000.0, antenna: "Antenna A".into() }),
        caps(),
        EngineConfig { tx_ham_only: false, ..Default::default() },
    );
    let thread = h.thread.take();

    std::thread::sleep(Duration::from_millis(150));
    for c in cmds {
        h.cmd_tx.send(c.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(60));
    }
    std::thread::sleep(Duration::from_millis(200));

    let mut last = None;
    while let Ok(ev) = h.event_rx.try_recv() {
        if let RadioEvent::State(s) = ev {
            last = Some(s);
        }
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
    last.expect("the engine should publish state")
}

fn on(vfo: Vfo, hz: f64, antenna: &str) -> Vec<Command> {
    vec![
        Command::SelectVfo(vfo),
        Command::SetVfo { vfo, hz },
        Command::SetAntenna { dir: Direction::Rx, name: antenna.into() },
    ]
}

/// The whole of issue #404, and the reporter's own steps: two VFOs on one band,
/// a socket each. The band memory holds whichever was chosen last and cannot
/// tell them apart, so this is the VFO's shelf answering.
#[test]
fn each_vfo_comes_back_on_the_socket_it_was_left_on() {
    let mut cmds = on(Vfo::A, 7_100_000.0, "Antenna A");
    cmds.extend(on(Vfo::B, 7_150_000.0, "Antenna B"));
    cmds.push(Command::SelectVfo(Vfo::A));
    let s = after(&cmds);
    assert_eq!(s.antenna_rx, "Antenna A", "A was left on Antenna A");

    // And over to B again, which was left on the other one.
    let mut cmds = on(Vfo::A, 7_100_000.0, "Antenna A");
    cmds.extend(on(Vfo::B, 7_150_000.0, "Antenna B"));
    cmds.push(Command::SelectVfo(Vfo::A));
    cmds.push(Command::SelectVfo(Vfo::B));
    let s = after(&cmds);
    assert_eq!(s.antenna_rx, "Antenna B", "B was left on Antenna B");
}

/// Tuning across a band edge is the band memory's business, unchanged: the
/// socket chosen on the band being entered is the one that comes back.
#[test]
fn tuning_across_a_band_edge_recalls_the_band() {
    let mut cmds = on(Vfo::A, 14_200_000.0, "Antenna B");
    // Down to 40 m, where the other socket is chosen, and back up.
    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: 7_100_000.0 });
    cmds.push(Command::SetAntenna { dir: Direction::Rx, name: "Antenna A".into() });
    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: 14_200_000.0 });
    let s = after(&cmds);
    assert_eq!(s.antenna_rx, "Antenna B", "20 m was left on Antenna B");
}

/// And an A/B press onto a VFO parked on another band is a band change like any
/// other, so the band wins there even against the VFO's own shelf.
///
/// A is left on 20 m on Antenna A. The band's own choice for 20 m is then moved
/// to Antenna B from the other VFO, so that the two records disagree — and
/// coming back to A has to give the band's answer, not A's.
#[test]
fn a_vfo_switch_onto_another_band_defers_to_the_band() {
    let mut cmds = on(Vfo::A, 14_200_000.0, "Antenna A");
    cmds.extend(on(Vfo::B, 7_100_000.0, "Antenna B"));
    // B visits 20 m, changes its mind about the band, and goes back to 40 m.
    cmds.push(Command::SetVfo { vfo: Vfo::B, hz: 14_200_000.0 });
    cmds.push(Command::SetAntenna { dir: Direction::Rx, name: "Antenna B".into() });
    cmds.push(Command::SetVfo { vfo: Vfo::B, hz: 7_100_000.0 });
    cmds.push(Command::SelectVfo(Vfo::A));
    let s = after(&cmds);
    assert!((s.active_freq_hz() - 14_200_000.0).abs() < 1.0);
    assert_eq!(s.antenna_rx, "Antenna B", "20 m's own socket, not the one A was left on");
}

/// A swap exchanges the listening positions, so the active VFO — which does not
/// change — is holding the other one's socket afterwards.
#[test]
fn swapping_exchanges_the_sockets_as_well_as_the_dials() {
    let mut cmds = on(Vfo::B, 7_150_000.0, "Antenna B");
    cmds.extend(on(Vfo::A, 7_100_000.0, "Antenna A"));
    cmds.push(Command::SwapVfos);
    let s = after(&cmds);
    assert_eq!(s.active_vfo, Vfo::A, "a swap does not change which VFO is in use");
    assert_eq!(s.antenna_rx, "Antenna B", "A is holding what B had");
}

/// A=B copies the whole position, the socket included.
#[test]
fn copying_a_to_b_copies_the_socket_with_it() {
    let mut cmds = on(Vfo::B, 7_150_000.0, "Antenna B");
    cmds.extend(on(Vfo::A, 7_100_000.0, "Antenna A"));
    cmds.push(Command::CopyAtoB);
    cmds.push(Command::SelectVfo(Vfo::B));
    let s = after(&cmds);
    assert_eq!(s.antenna_rx, "Antenna A", "B is a copy of A now, socket included");
}

/// Re-selecting the VFO already in use must not undo a socket the operator has
/// just chosen: the shelf is written on the way out, so a redundant select
/// shelves what is in force and puts it straight back.
#[test]
fn selecting_the_vfo_already_in_use_changes_nothing() {
    let mut cmds = on(Vfo::A, 7_100_000.0, "Antenna A");
    cmds.push(Command::SetAntenna { dir: Direction::Rx, name: "Antenna B".into() });
    cmds.push(Command::SelectVfo(Vfo::A));
    let s = after(&cmds);
    assert_eq!(s.antenna_rx, "Antenna B");
}

/// The mode memory is untouched by all this — the two ride the same shelf.
#[test]
fn the_mode_still_goes_with_the_vfo() {
    let mut cmds = on(Vfo::A, 7_100_000.0, "Antenna A");
    cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
    cmds.extend(on(Vfo::B, 7_150_000.0, "Antenna B"));
    cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Lsb });
    cmds.push(Command::SelectVfo(Vfo::A));
    let s = after(&cmds);
    assert_eq!(s.rx[0].mode, Mode::Cw);
    assert_eq!(s.antenna_rx, "Antenna A");
}
