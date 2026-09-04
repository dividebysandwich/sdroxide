//! Antenna selection: a multi-port front end (a LimeSDR receives on
//! LNAH/LNAL/LNAW and transmits on BAND1/BAND2) must end up on the port the
//! operator asked for — at start, when they switch it, and again after a
//! reconnect, which reopens the device on whatever its driver defaults to.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, ControlUpdate, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Direction, RadioEvent};

const RATE: f64 = 48_000.0;
const CENTER: f64 = 14_100_000.0;

/// The ports a front end is on, shared with the test so it can see what the
/// engine actually asked the hardware for — as opposed to what the published
/// state says, which is only the readback of it.
#[derive(Clone)]
struct Ports {
    rx: Arc<Mutex<String>>,
    tx: Arc<Mutex<String>>,
    /// Every selection the engine made, in order, tagged with its direction.
    asked: Arc<Mutex<Vec<(Direction, String)>>>,
}

impl Ports {
    /// A device that powers up on its first port, as a driver does.
    fn fresh() -> Self {
        Ports {
            rx: Arc::new(Mutex::new("LNAH".into())),
            tx: Arc::new(Mutex::new("BAND1".into())),
            asked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn rx(&self) -> String {
        self.rx.lock().unwrap().clone()
    }

    fn tx(&self) -> String {
        self.tx.lock().unwrap().clone()
    }

    fn asked(&self) -> Vec<(Direction, String)> {
        self.asked.lock().unwrap().clone()
    }
}

struct Rig {
    ports: Ports,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "multi-port rig".into()
    }
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Rx, name.into()));
        *self.ports.rx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_antenna(&self) -> String {
        self.ports.rx()
    }
    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Tx, name.into()));
        *self.ports.tx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_tx_antenna(&self) -> String {
        self.ports.tx()
    }
}

/// A front end whose antenna is a setting in the *radio* rather than a switch
/// in the driver — an ELAD FDM-DUO's `AN`, which lives in the transceiver, keeps
/// its value across a power cycle, and is read back when the control link opens.
/// Such a rig announces its port instead of being asked for one.
struct ReportingRig {
    ports: Ports,
    /// The port the radio reports, once, the first time it is polled.
    report: Option<&'static str>,
}

impl IqSource for ReportingRig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "rig that reports its own port".into()
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        match self.report.take() {
            // Recorded as where the hardware is, but not as something that was
            // *asked* of it: the operator set this at the front panel, and all
            // that has happened here is finding out.
            //
            // Unless something *has* been asked of it since, in which case this
            // is a stale answer that crossed that command on the wire — the
            // radio has moved on, and the report says where it used to be.
            Some(name) => {
                if self.ports.asked.lock().unwrap().is_empty() {
                    *self.ports.rx.lock().unwrap() = name.into();
                }
                vec![ControlUpdate::Antenna(name)]
            }
            None => Vec::new(),
        }
    }
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Rx, name.into()));
        *self.ports.rx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_antenna(&self) -> String {
        self.ports.rx()
    }
}

/// A LimeSDR with a LimeRFE bolted in front of it: the front end is one coaxial
/// cable into one of the receive sockets, so which socket to listen on is a fact
/// about the cabling and the source says so ([`IqSource::owns_rx_antenna`]).
struct CabledRig {
    ports: Ports,
}

impl IqSource for CabledRig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "LimeSDR behind a LimeRFE".into()
    }
    fn owns_rx_antenna(&self) -> bool {
        true
    }
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Rx, name.into()));
        *self.ports.rx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_antenna(&self) -> String {
        self.ports.rx()
    }
    fn set_tx_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Tx, name.into()));
        *self.ports.tx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_tx_antenna(&self) -> String {
        self.ports.tx()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "lime".into(),
        label: "LimeSDR".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 60_000_000.0)],
        freq_ranges_tx: vec![(0.0, 60_000_000.0)],
        antennas_rx: vec!["LNAH".into(), "LNAL".into(), "LNAW".into()],
        antennas_tx: vec!["BAND1".into(), "BAND2".into()],
        ..DeviceCaps::default()
    }
}

/// Wait for a published state that satisfies `want`, so the assertions are
/// about what every UI sees rather than about internal timing.
fn wait_for_state(
    rx: &crossbeam_channel::Receiver<RadioEvent>,
    want: impl Fn(&sdroxide_types::RadioState) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(RadioEvent::State(s)) if want(&s) => return true,
            Ok(_) => {}
            Err(_) => {}
        }
    }
    false
}

/// `--antenna` / `--tx-antenna` on a headless server: nobody is at the machine
/// to pick a port, so the one named on the command line has to be selected as
/// the engine comes up.
#[test]
fn the_startup_antenna_is_selected_on_the_front_end() {
    let ports = Ports::fresh();
    let cfg = EngineConfig {
        initial_antenna: (Some("LNAW".into()), Some("BAND2".into())),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { ports: ports.clone() }), caps(), cfg);
    let thread = h.thread.take();

    assert!(
        wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW" && s.antenna_tx == "BAND2"),
        "the opening state must show the ports that were asked for, not the driver's defaults"
    );
    assert_eq!(ports.rx(), "LNAW", "the RX port reached the hardware");
    assert_eq!(ports.tx(), "BAND2", "the TX port reached the hardware");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// A port the front end does not have is somebody else's radio — the preference
/// outlived an interface switch. Asking for it would only make the driver log an
/// error, so it is skipped and the device is left where it opened.
#[test]
fn an_antenna_this_device_does_not_have_is_left_alone() {
    let ports = Ports::fresh();
    let cfg = EngineConfig {
        initial_antenna: (Some("VERTICAL".into()), Some("ATU".into())),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { ports: ports.clone() }), caps(), cfg);
    let thread = h.thread.take();

    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAH"));
    assert!(ports.asked().is_empty(), "nothing should have been asked of the device");
    assert_eq!(ports.rx(), "LNAH");
    assert_eq!(ports.tx(), "BAND1");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// The TX port used to be recorded in the state and never sent anywhere. A
/// LimeSDR that transmits on the wrong band port puts the power into a filter
/// that isn't for this band, so the command has to reach the hardware.
#[test]
fn selecting_the_tx_antenna_reaches_the_hardware() {
    let ports = Ports::fresh();
    let mut h =
        start_engine(Box::new(Rig { ports: ports.clone() }), caps(), EngineConfig::default());
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetAntenna { dir: Direction::Tx, name: "BAND2".into() }).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_tx == "BAND2"));
    assert_eq!(ports.tx(), "BAND2");
    assert_eq!(ports.asked(), vec![(Direction::Tx, "BAND2".to_string())]);

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// A station with more than one antenna has one *per band*: the beam on 2 m and
/// the wire on 40. Switching bands has to bring the socket with it, or the
/// operator reaches for the antenna control every time the dial crosses a band
/// edge — which is an antenna selector not doing its job (issues #235, #238).
#[test]
fn the_antenna_follows_the_band_it_was_chosen_on() {
    let ports = Ports::fresh();
    let mut h =
        start_engine(Box::new(Rig { ports: ports.clone() }), caps(), EngineConfig::default());
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // 40 m on LNAL…
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 7_100_000.0 });
    send(Command::SetAntenna { dir: Direction::Rx, name: "LNAL".into() });
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAL"));

    // …20 m on LNAW.
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 14_100_000.0 });
    send(Command::SetAntenna { dir: Direction::Rx, name: "LNAW".into() });
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"));

    // Back to 40 m: the socket comes back on its own.
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 7_050_000.0 });
    assert!(
        wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAL"),
        "40 m was left on LNAL and has to come back to it"
    );
    assert_eq!(ports.rx(), "LNAL", "and the socket reached the hardware");

    // A band nobody has chosen on is left exactly where the radio already is:
    // no preference means no assertion, the same rule a restored session
    // follows.
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 28_100_000.0 });
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(ports.rx(), "LNAL", "10 m has no remembered socket, so nothing moved");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// A rig that drops out and comes back is reopened by the engine, and a freshly
/// opened device is on its driver's default port. The antenna belongs to the
/// station's coax rather than to the front end, so it has to be restored — an
/// operator who came back to a working waterfall would have no reason to suspect
/// they were listening on the wrong feedline.
#[test]
fn a_reconnect_returns_to_the_selected_antenna() {
    // The replacement the factory hands back: a fresh device on LNAH/BAND1.
    let after = Ports::fresh();
    let handed = after.clone();
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        Ok((Box::new(Rig { ports: handed.clone() }) as Box<dyn IqSource>, caps()))
    });

    let before = Ports::fresh();
    let cfg = EngineConfig {
        initial_antenna: (Some("LNAL".into()), None),
        reopen: Some(reopen),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(Rig { ports: before.clone() }), caps(), cfg);
    let thread = h.thread.take();
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAL"));

    // The operator moves to a third port, then the interface is rebuilt under
    // them: what they chose by hand is what must come back, not what they
    // started on.
    h.cmd_tx.send(Command::SetAntenna { dir: Direction::Rx, name: "LNAW".into() }).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"));
    assert_eq!(before.rx(), "LNAW");

    h.swap_tx.send(sdroxide_radio::EngineSwap::ReopenSource).unwrap();
    assert!(
        wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"),
        "the reopened front end must be put back on the operator's port"
    );
    assert_eq!(after.rx(), "LNAW", "and the new device is the one that was asked");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// A radio that carries its own antenna switch is *asked* which port it is on,
/// not told — the setting is in the rig and survived its last power cycle. What
/// it answers has to end up on screen, and has to be remembered: the next time
/// this front end is opened, putting a session file's port back on top of the
/// one the operator left the radio on would move an antenna relay nobody
/// touched.
#[test]
fn the_port_the_radio_reports_is_adopted_and_remembered() {
    // The replacement a reconnect hands back: a fresh device, on its default.
    let after = Ports::fresh();
    let handed = after.clone();
    let reopen: sdroxide_radio::ReopenFn = Box::new(move |_center: f64| {
        Ok((
            Box::new(ReportingRig { ports: handed.clone(), report: None }) as Box<dyn IqSource>,
            caps(),
        ))
    });

    let before = Ports::fresh();
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(
        Box::new(ReportingRig { ports: before.clone(), report: Some("LNAW") }),
        caps(),
        cfg,
    );
    let thread = h.thread.take();

    assert!(
        wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"),
        "the port the radio reported must be the one on screen"
    );
    assert!(
        before.asked().is_empty(),
        "adopting a port must not command the radio back to anything"
    );

    // Rebuild the interface: the new device comes up on its driver default, and
    // the port the *radio* reported is the one that has to be put back.
    h.swap_tx.send(sdroxide_radio::EngineSwap::ReopenSource).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"));
    assert_eq!(after.rx(), "LNAW", "the reported port outlived the device that reported it");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// The other half of the rule above: a port this end has already *asserted*
/// outranks what the radio says about itself.
///
/// Every open re-asserts the remembered port, and the radio is read at the same
/// moment, so the two cross on the wire as a matter of course: the answer
/// describes the socket the rig was on a fraction of a second before it was told
/// to move. Adopting that would show — and then remember — the port the operator
/// had just left, on a radio that is no longer on it.
#[test]
fn a_port_the_operator_asked_for_outranks_what_the_radio_reports() {
    let ports = Ports::fresh();
    let cfg = EngineConfig {
        // What the operator wants, from the command line or the session.
        initial_antenna: (Some("LNAW".into()), None),
        ..Default::default()
    };
    // What the radio answers: where it was before the command reached it.
    let mut h = start_engine(
        Box::new(ReportingRig { ports: ports.clone(), report: Some("LNAH") }),
        caps(),
        cfg,
    );
    let thread = h.thread.take();

    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"));
    assert_eq!(ports.rx(), "LNAW", "the radio was told, and that is where it is");

    // Every state published from here on has to still say LNAW. The report is
    // polled within a loop or two of the engine starting, and adopting it would
    // publish a state saying otherwise — which is precisely what an operator
    // would see on screen.
    let deadline = Instant::now() + Duration::from_millis(400);
    while Instant::now() < deadline {
        if let Ok(RadioEvent::State(s)) = h.event_rx.recv_timeout(Duration::from_millis(50)) {
            assert_eq!(
                s.antenna_rx, "LNAW",
                "a stale report must not move the panel off the port that was asked for"
            );
        }
    }
    assert_eq!(ports.rx(), "LNAW", "and the radio must be left where it was told");
    assert_eq!(
        ports.asked(),
        vec![(Direction::Rx, "LNAW".to_string())],
        "nothing may be re-commanded on the strength of the report either"
    );

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// The field report behind this: a LimeRFE cabled to the wideband socket, a
/// `session.json` written by an earlier run that had put the radio on LNAL, and
/// a receiver listening to an empty connector at every start. Where the source
/// owns the receive socket, a remembered one is not restored over it.
#[test]
fn a_remembered_port_does_not_override_the_cabling() {
    let ports = Ports::fresh();
    *ports.rx.lock().unwrap() = "LNAW".into();
    let cfg = EngineConfig {
        // What an earlier session recorded, which was never a decision.
        initial_antenna: (Some("LNAL".into()), Some("BAND2".into())),
        ..Default::default()
    };
    let mut h = start_engine(Box::new(CabledRig { ports: ports.clone() }), caps(), cfg);
    let thread = h.thread.take();

    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAW"));
    assert_eq!(ports.rx(), "LNAW", "the front end must be left on the socket it is wired to");
    assert!(
        !ports.asked().iter().any(|(d, _)| *d == Direction::Rx),
        "nothing may be asked of the receive path at all: {:?}",
        ports.asked()
    );
    // Transmit is untouched by this — that port really is a preference, and the
    // remembered one still has to reach the hardware.
    assert_eq!(ports.tx(), "BAND2");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// Owning the socket is about not *restoring* one, never about refusing the
/// operator: somebody who wired their front end to LNAL still has to be able to
/// say so and be listened to.
#[test]
fn the_operator_can_still_move_a_cabled_front_end() {
    let ports = Ports::fresh();
    *ports.rx.lock().unwrap() = "LNAW".into();
    let mut h =
        start_engine(Box::new(CabledRig { ports: ports.clone() }), caps(), EngineConfig::default());
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetAntenna { dir: Direction::Rx, name: "LNAL".into() }).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAL"));
    assert_eq!(ports.rx(), "LNAL");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// A transceiver with a separate *receiving* antenna: an extra input switched
/// into the receive path while the aerial on the main socket stays on transmit.
///
/// Nothing like the sockets above, and it is the difference that matters. The
/// radio holds this setting itself, per band, and answers for it — so sdroxide
/// reads it and adopts it, and writes it only when the operator clicks. Every
/// write sdroxide *does* make carries the value it last read, because on an
/// IC-7300MK2 the same command carries both and a zero there means "out of
/// circuit" (issue #229).
struct RxAntRig {
    ports: Ports,
    /// What the radio is set to, as the radio holds it. `None` is a radio with
    /// no such connector, which is how most of them answer.
    rx_ant: Arc<Mutex<Option<bool>>>,
    /// Every write the engine made to it, and every time it went and asked.
    wrote: Arc<Mutex<Vec<bool>>>,
    asked: Arc<Mutex<usize>>,
    /// What the radio will report next time it is polled — the answer to an
    /// ask, or the operator reaching for the button on the radio itself.
    report: Arc<Mutex<Option<bool>>>,
}

impl RxAntRig {
    fn new(rx_ant: Option<bool>) -> RxAntRig {
        RxAntRig {
            ports: Ports::fresh(),
            rx_ant: Arc::new(Mutex::new(rx_ant)),
            wrote: Arc::new(Mutex::new(Vec::new())),
            asked: Arc::new(Mutex::new(0)),
            report: Arc::new(Mutex::new(None)),
        }
    }
}

impl IqSource for RxAntRig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        CENTER
    }
    fn set_center_hz(&mut self, _hz: f64) -> Result<()> {
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "rig with a receiving antenna".into()
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        match self.report.lock().unwrap().take() {
            Some(on) => vec![ControlUpdate::RxAntenna(on)],
            None => Vec::new(),
        }
    }
    fn rx_antenna(&self) -> Option<bool> {
        *self.rx_ant.lock().unwrap()
    }
    fn set_rx_antenna(&mut self, on: bool) -> Result<()> {
        self.wrote.lock().unwrap().push(on);
        *self.rx_ant.lock().unwrap() = Some(on);
        Ok(())
    }
    fn reread_rx_antenna(&mut self) {
        *self.asked.lock().unwrap() += 1;
        // The radio answering, a round trip later.
        *self.report.lock().unwrap() = *self.rx_ant.lock().unwrap();
    }
    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.ports.asked.lock().unwrap().push((Direction::Rx, name.into()));
        *self.ports.rx.lock().unwrap() = name.into();
        Ok(())
    }
    fn current_antenna(&self) -> String {
        self.ports.rx()
    }
}

/// The control appears only for a radio that has said it has the connector —
/// which it says by answering with the flag at all. Everything else, which is
/// every SDR, publishes nothing and gets no control.
#[test]
fn the_receiving_antenna_is_published_only_where_the_radio_has_one() {
    for (has, want) in [(None, false), (Some(false), true), (Some(true), true)] {
        let rig = RxAntRig::new(has);
        let mut h = start_engine(Box::new(rig), caps(), EngineConfig::default());
        let thread = h.thread.take();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = None;
        while Instant::now() < deadline && seen.is_none() {
            if let Ok(RadioEvent::Capabilities(c) | RadioEvent::CapabilitiesUpdated(c)) =
                h.event_rx.recv_timeout(Duration::from_millis(100))
            {
                seen = Some(c.has_rx_antenna);
            }
        }
        assert_eq!(seen, Some(want), "a radio answering {has:?}");
        drop(h);
        let _ = thread.map(|t| t.join());
    }
}

/// The end-to-end case the issue was about: a band change must *ask* the radio
/// what its receiving antenna is set to, never assert one. sdroxide had been
/// writing the byte as zero on every band change, which switched the operator's
/// receive aerial out of circuit each time they changed band.
#[test]
fn a_band_change_asks_for_the_receiving_antenna_rather_than_asserting_it() {
    let rig = RxAntRig::new(Some(true));
    let (rx_ant, wrote, asked, report) =
        (rig.rx_ant.clone(), rig.wrote.clone(), rig.asked.clone(), rig.report.clone());
    let mut h = start_engine(Box::new(rig), caps(), EngineConfig::default());
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    assert!(wait_for_state(&h.event_rx, |s| s.rx_antenna), "the radio's own setting is adopted");

    // Change band, one crossing at a time — the engine drains its whole
    // command queue before it looks at the band, so two sent together are one
    // change and the test would be counting its own timing.
    let asks = || *asked.lock().unwrap();
    let before = asks();
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 7_100_000.0 });
    assert!(wait_for_state(&h.event_rx, |s| s.band == sdroxide_types::Band::M40));
    std::thread::sleep(Duration::from_millis(100));
    assert!(asks() > before, "a band change asks");

    // A socket change asks too: on an IC-7610 the flag belongs to the socket
    // being selected rather than to the one being left.
    let before = asks();
    send(Command::SetAntenna { dir: Direction::Rx, name: "LNAL".into() });
    assert!(wait_for_state(&h.event_rx, |s| s.antenna_rx == "LNAL"));
    std::thread::sleep(Duration::from_millis(100));
    assert!(asks() > before, "a socket change asks");

    // And back, with the socket now remembered on 40 m so the assertion of
    // issue #258 is in the picture too.
    let before = asks();
    send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: 14_100_000.0 });
    assert!(wait_for_state(&h.event_rx, |s| s.band == sdroxide_types::Band::M20));
    std::thread::sleep(Duration::from_millis(100));
    assert!(asks() > before, "every band change asks, not just the first");

    assert_eq!(*rx_ant.lock().unwrap(), Some(true), "the receive aerial was switched out");
    assert!(wrote.lock().unwrap().is_empty(), "nothing may write it but a click");

    // And what the radio says is adopted, including a change made at the radio
    // itself that the next ask brings back.
    *rx_ant.lock().unwrap() = Some(false);
    *report.lock().unwrap() = Some(false);
    assert!(wait_for_state(&h.event_rx, |s| !s.rx_antenna), "the radio's own change is adopted");

    drop(h);
    let _ = thread.map(|t| t.join());
}

/// The one thing that does move it, and the echo that answers the click before
/// the radio has.
#[test]
fn only_a_click_writes_the_receiving_antenna() {
    let rig = RxAntRig::new(Some(false));
    let wrote = rig.wrote.clone();
    let mut h = start_engine(Box::new(rig), caps(), EngineConfig::default());
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetRxAntenna(true)).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| s.rx_antenna));
    assert_eq!(*wrote.lock().unwrap(), vec![true]);

    h.cmd_tx.send(Command::SetRxAntenna(false)).unwrap();
    assert!(wait_for_state(&h.event_rx, |s| !s.rx_antenna));
    assert_eq!(*wrote.lock().unwrap(), vec![true, false]);

    drop(h);
    let _ = thread.map(|t| t.join());
}
