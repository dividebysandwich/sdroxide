//! An HF upconverter must not make the operator do the arithmetic.
//!
//! With a Ham It Up in the antenna line, 10.1008 MHz on the air arrives at the
//! receiver as 135.1008 MHz. Everything above `ConvertedSource` — the dial, the
//! band edges, memories, spots, what gets uploaded to PSK Reporter — has to say
//! 10.1008 MHz, and only the hardware may hear about the other number.
//!
//! No upconverter was available while this was written, so these tests are the
//! acceptance criterion for the arithmetic; the hardware remains unproven.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use sdroxide_radio::{
    Complex32, ConvertedSource, EngineConfig, EngineSwap, IqSource, ReopenFn, Result,
    converter_open_hz, shift_caps, start_engine,
};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, Vfo};

/// A Ham It Up / SpyVerter.
const OFFSET: f64 = 125_000_000.0;
/// 30 m WSPR, the frequency the operator wants to see on the dial.
const DIAL: f64 = 10_100_800.0;
const RATE: f64 = 2_400_000.0;

/// A front end that accepts every tune and records where it was sent. Everything
/// it sees is in the *hardware* domain.
struct Hardware {
    center_hz: f64,
    rate_hz: f64,
    lo_offset_hz: f64,
    landed: Arc<Mutex<f64>>,
    /// Where the transmitter was keyed, in the hardware's own domain.
    keyed: Arc<Mutex<Option<f64>>>,
    band_dial: Arc<Mutex<Option<f64>>>,
    tx_freq: Arc<Mutex<Option<f64>>>,
}

impl Hardware {
    fn new(center_hz: f64) -> Self {
        Hardware {
            center_hz,
            rate_hz: RATE,
            lo_offset_hz: 0.0,
            landed: Arc::new(Mutex::new(center_hz)),
            keyed: Arc::new(Mutex::new(None)),
            band_dial: Arc::new(Mutex::new(None)),
            tx_freq: Arc::new(Mutex::new(None)),
        }
    }
}

impl IqSource for Hardware {
    fn sample_rate(&self) -> f64 {
        self.rate_hz
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        *self.landed.lock().unwrap() = hz;
        Ok(())
    }
    fn set_dial_hz(&mut self, hz: f64) {
        *self.band_dial.lock().unwrap() = Some(hz);
    }
    fn set_tx_freq_hz(&mut self, hz: f64) {
        *self.tx_freq.lock().unwrap() = Some(hz);
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "test front end".into()
    }
    fn tx_begin(&mut self, center_hz: f64, rate: f64) -> Result<f64> {
        *self.keyed.lock().unwrap() = Some(center_hz);
        Ok(rate)
    }
    /// Accepted and dropped. The default refuses, which ends the engine's loop
    /// — a transmit test would then measure a dead engine.
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A universal Ku-band LNB's low-band oscillator, the QO-100 station's receive
/// converter: the 10489.550 MHz downlink arrives at the radio as 739.550 MHz.
const LNB: f64 = -9_750_000_000.0;
/// The QO-100 narrowband transponder, in the numbers on the dial: an SSB
/// downlink and the uplink 8089.5 MHz below it.
const DOWNLINK: f64 = 10_489_550_000.0;
const UPLINK: f64 = 2_400_050_000.0;

/// A device that can hear the whole VHF range the converter's output lands in,
/// and transmit on it — so the transmit withdrawal is a real change, not a
/// device that could never key anyway.
fn hardware_caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(24_000_000.0, 1_766_000_000.0)],
        freq_ranges_tx: vec![(24_000_000.0, 1_766_000_000.0)],
        antennas_tx: vec!["TX/RX".into()],
        ..DeviceCaps::default()
    }
}

/// An AD9361 board — a Pluto, a LibreSDR — which is what a QO-100 station has
/// at both ends of the transponder: it hears the LNB's 739 MHz output and it
/// transmits at 2.4 GHz, and neither of those is 10 GHz.
fn qo100_caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(70_000_000.0, 6_000_000_000.0)],
        freq_ranges_tx: vec![(70_000_000.0, 6_000_000_000.0)],
        antennas_tx: vec!["TX/RX".into()],
        ..DeviceCaps::default()
    }
}

/// Collect events for `secs`, returning the notices seen and where the dial and
/// the hardware centre ended up (both as the *engine* understands them).
fn drain(rx: &Receiver<RadioEvent>, secs: f64) -> (Vec<String>, f64, f64) {
    let mut notices = Vec::new();
    let (mut dial, mut center) = (f64::NAN, f64::NAN);
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < deadline {
        while let Ok(ev) = rx.try_recv() {
            match ev {
                RadioEvent::Notice(Some(n)) => notices.push(n),
                RadioEvent::State(s) => {
                    dial = s.active_freq_hz();
                    center = s.center_hz;
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    (notices, dial, center)
}

/// The whole point: type 10.1008 and the hardware goes to 135.1008, while every
/// number the engine publishes stays on 10.1008.
#[test]
fn the_dial_stays_on_air_while_the_hardware_takes_the_offset() {
    let inner = Hardware::new(DIAL + OFFSET);
    let landed = Arc::clone(&inner.landed);
    let source = ConvertedSource::new(Box::new(inner), OFFSET, None);
    let mut h = start_engine(
        Box::new(source),
        shift_caps(hardware_caps(), OFFSET, None),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    // Far enough to leave the span, so the engine has to move the hardware.
    let target = DIAL + 3_000_000.0;
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: target }).unwrap();
    let (notices, dial, center) = drain(&h.event_rx, 1.0);

    assert!(notices.is_empty(), "this frequency is reachable, got {notices:?}");
    assert!((dial - target).abs() < 1.0, "the dial should read {target}, not {dial}");
    assert!((center - target).abs() < 1.0, "the centre is operator-domain too, got {center}");
    let hw = *landed.lock().unwrap();
    assert!(
        (hw - (target + OFFSET)).abs() < 1.0,
        "the hardware should have gone to {}, not {hw}",
        target + OFFSET
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Where the source opened is where the dial starts, so a restart with a
/// converter set does not come up 125 MHz out — or, with the subtraction
/// missing, on a negative frequency.
#[test]
fn the_engine_starts_on_the_operator_frequency() {
    let inner = Hardware::new(DIAL + OFFSET);
    let source = ConvertedSource::new(Box::new(inner), OFFSET, None);
    let mut h = start_engine(
        Box::new(source),
        shift_caps(hardware_caps(), OFFSET, None),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    let (_, dial, center) = drain(&h.event_rx, 0.5);
    assert!(dial > 0.0, "the dial must not start below DC, got {dial}");
    assert!((dial - DIAL).abs() < 1.0, "the dial should start on {DIAL}, not {dial}");
    assert!((center - DIAL).abs() < 1.0, "and so should the centre, got {center}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The converter and a zero-IF front end's own LO offset are different things
/// and have to compose: the LO is parked clear of the VFO *and* the whole axis
/// is translated.
#[test]
fn a_converter_composes_with_a_zero_if_lo_offset() {
    const LO: f64 = 600_000.0;
    let inner = Hardware { lo_offset_hz: LO, ..Hardware::new(DIAL + LO + OFFSET) };
    let landed = Arc::clone(&inner.landed);
    let source = ConvertedSource::new(Box::new(inner), OFFSET, None);
    let mut h = start_engine(
        Box::new(source),
        shift_caps(hardware_caps(), OFFSET, None),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    let target = DIAL + 3_000_000.0;
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: target }).unwrap();
    let (notices, dial, center) = drain(&h.event_rx, 1.0);

    assert!(notices.is_empty(), "this frequency is reachable, got {notices:?}");
    assert!((dial - target).abs() < 1.0, "the dial should read {target}, not {dial}");
    // The DDC has to make up the LO offset and nothing else — if the converter
    // leaked into this difference the demodulator would be 125 MHz off.
    assert!(
        (dial - center - -LO).abs() < 1.0,
        "the DDC should see only the LO offset, got {}",
        dial - center
    );
    let hw = *landed.lock().unwrap();
    assert!(
        (hw - (target + LO + OFFSET)).abs() < 1.0,
        "the hardware should have gone to {}, not {hw}",
        target + LO + OFFSET
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A dial the converted front end cannot reach is refused, and the operator is
/// told so in frequencies they recognise — not in the hardware's.
#[test]
fn an_unreachable_dial_is_refused_in_the_operators_own_numbers() {
    let inner = Hardware::new(DIAL + OFFSET);
    let source = ConvertedSource::new(Box::new(inner), OFFSET, None);
    let mut h = start_engine(
        Box::new(source),
        shift_caps(hardware_caps(), OFFSET, None),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    // The hardware tops out at 1766 MHz, which is 1641 MHz on the dial.
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: 1_700_000_000.0 }).unwrap();
    let (notices, dial, _) = drain(&h.event_rx, 1.0);

    assert!(
        notices.iter().any(|n| n.contains("outside") && n.contains("1641.000")),
        "the range should be quoted as the operator sees it, got {notices:?}"
    );
    assert!((dial - DIAL).abs() < 1.0, "the dial should be back on {DIAL}, not {dial}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A converter is not in the transmit path, so by default it takes transmit
/// with it. The alternative is worse than no feature: the licence check passes
/// on the dial and the radio keys 125 MHz up, in an aeronautical band.
#[test]
fn a_converter_withdraws_transmit() {
    let caps = shift_caps(hardware_caps(), OFFSET, None);
    assert!(!caps.is_transmit_capable(), "transmit must be gone entirely");
    assert!(!caps.can_tx_hz(DIAL), "and no frequency may be transmittable");
    assert!(caps.antennas_tx.is_empty());
    // Receive is untouched apart from the translation: HF opens up, and the top
    // of the range comes down by the same amount.
    assert!(caps.can_rx_hz(DIAL), "10.1008 MHz is reachable through the converter");
    assert!(!hardware_caps().can_rx_hz(DIAL), "and was not before it");
    assert!(!caps.can_rx_hz(1_700_000_000.0), "the ceiling came down with everything else");
    assert!(hardware_caps().can_rx_hz(1_700_000_000.0), "though the hardware itself reaches it");
}

/// …but a station that says what is in its transmit line keeps it. The QO-100
/// shape: the 10 GHz downlink arrives through an LNB, the 2.4 GHz uplink leaves
/// the radio direct, so receive is offset by −9750 MHz and transmit is not
/// offset at all.
#[test]
fn a_stated_transmit_line_keeps_transmit() {
    let caps = shift_caps(qo100_caps(), LNB, Some(0.0));
    assert!(caps.is_transmit_capable(), "the transmitter is still there");
    assert_eq!(caps.antennas_tx, vec!["TX/RX".to_string()], "and so is its antenna");
    // The downlink is reachable on receive through the LNB…
    assert!(caps.can_rx_hz(DOWNLINK), "10489.550 MHz through a 9750 MHz LNB");
    // …and the uplink is transmittable, in the radio's own untouched domain.
    assert!(caps.may_tx_hz(UPLINK), "2400.050 MHz straight out of the radio");
    assert!(!caps.may_tx_hz(DOWNLINK), "but not 10 GHz, which nothing here can key");
}

/// One box converting both ways is a transverter, and then the transmit offset
/// *is* the receive one — 1296 MHz on the dial, 144 MHz at the radio, in both
/// directions.
#[test]
fn a_transverter_converts_the_transmit_path_by_the_same_offset() {
    const IF_OFFSET: f64 = -1_152_000_000.0;
    let inner = Hardware::new(1_296_200_000.0 + IF_OFFSET);
    let keyed = Arc::clone(&inner.keyed);
    let mut s = ConvertedSource::new(Box::new(inner), IF_OFFSET, Some(IF_OFFSET));
    s.tx_begin(1_296_200_000.0, RATE).expect("keys up");
    assert_eq!(
        *keyed.lock().unwrap(),
        Some(144_200_000.0),
        "the radio transmits at its IF, not at the dial"
    );
}

/// With no transmit line stated, `tx_begin` refuses rather than guessing. The
/// engine's gate stops a key-down long before this — but every other way in
/// (a CAT client, a digital mode) has to land somewhere safe, and "somewhere
/// safe" is not the dial frequency: a Pluto asked for 10.489 GHz clamps to the
/// top of its range and transmits *there*.
#[test]
fn transmit_through_a_converter_is_refused_when_nothing_is_stated() {
    let inner = Hardware::new(DOWNLINK + LNB);
    let keyed = Arc::clone(&inner.keyed);
    let mut s = ConvertedSource::new(Box::new(inner), LNB, None);
    let err = s.tx_begin(DOWNLINK, RATE).expect_err("transmit is withdrawn");
    assert!(format!("{err}").contains("transmit line"), "the reason should say so, got {err}");
    assert_eq!(*keyed.lock().unwrap(), None, "and nothing reached the radio");
}

/// End to end, the way the operator drives it: dial on the downlink, split with
/// the uplink on VFO B, and PTT. The transmitter has to key on 2400.050 MHz —
/// no "outside this radio's transmit range", no 10 GHz reaching the hardware.
#[test]
fn a_qo100_station_keys_up_on_its_uplink() {
    let inner = Hardware::new(DOWNLINK + LNB);
    let keyed = Arc::clone(&inner.keyed);
    let source = ConvertedSource::new(Box::new(inner), LNB, Some(0.0));
    let mut h = start_engine(
        Box::new(source),
        shift_caps(qo100_caps(), LNB, Some(0.0)),
        EngineConfig::default(),
    );
    let thread = h.thread.take();

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DOWNLINK }).unwrap();
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::B, hz: UPLINK }).unwrap();
    h.cmd_tx.send(Command::SetSplit(true)).unwrap();
    h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    let (notices, dial, _) = drain(&h.event_rx, 1.0);

    assert!(notices.is_empty(), "the uplink is inside the radio's range, got {notices:?}");
    assert!((dial - DOWNLINK).abs() < 1.0, "the dial stays on the downlink, got {dial}");
    match *keyed.lock().unwrap() {
        Some(hz) => assert!((hz - UPLINK).abs() < 1.0, "keyed on {hz}, not on the uplink"),
        None => panic!("the transmitter never keyed"),
    }

    h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Receive edges move into the operator's domain, and never below DC: a
/// negative low edge is not a frequency, and it reads as nonsense in the
/// "outside this radio's receive range" notice and in what the rigctld and TCI
/// servers advertise.
#[test]
fn shifted_receive_ranges_are_clamped_at_dc() {
    let caps = DeviceCaps {
        // An RTL-SDR Blog V4: everything from DC up.
        freq_ranges_rx: vec![(0.0, 1_766_000_000.0), (0.0, 1_000_000.0)],
        ..DeviceCaps::default()
    };
    let shifted = shift_caps(caps, OFFSET, None);
    assert_eq!(shifted.freq_ranges_rx, vec![(0.0, 1_766_000_000.0 - OFFSET)]);
    // The second range ended below the offset, so through this converter there
    // is nothing in it to hear; it is dropped rather than inverted.
    assert!(shifted.freq_ranges_rx.iter().all(|&(lo, hi)| lo >= 0.0 && hi > lo));
}

/// Every call the wrapper makes to the front end underneath it, in the order the
/// trait declares them.
#[derive(Default)]
struct Calls(Arc<Mutex<Vec<&'static str>>>);

impl Calls {
    fn note(&self, what: &'static str) {
        self.0.lock().unwrap().push(what);
    }
    fn seen(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }
}

/// A front end that answers nothing but remembers being asked. Its frequencies
/// are deliberately distinctive so a missing translation shows up as a number,
/// not as a shrug.
struct Recorder {
    calls: Calls,
    hw_center: f64,
    released: Arc<AtomicBool>,
}

impl IqSource for Recorder {
    fn sample_rate(&self) -> f64 {
        self.calls.note("sample_rate");
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.calls.note("center_hz");
        self.hw_center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.calls.note("set_center_hz");
        self.hw_center = hz;
        Ok(())
    }
    fn select_vfo(&mut self, _vfo: Vfo, hz: f64) {
        self.calls.note("select_vfo");
        // A rig selecting a VFO receives on that VFO's frequency, so the
        // centre it reports is the number it was handed.
        self.hw_center = hz;
    }
    fn lo_offset_hz(&self) -> f64 {
        self.calls.note("lo_offset_hz");
        600_000.0
    }
    fn read(&mut self, _buf: &mut [Complex32]) -> Result<usize> {
        self.calls.note("read");
        Ok(0)
    }
    fn describe(&self) -> String {
        self.calls.note("describe");
        "recorder".into()
    }
    fn set_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        self.calls.note("set_gain_element");
        Ok(())
    }
    fn set_antenna(&mut self, _name: &str) -> Result<()> {
        self.calls.note("set_antenna");
        Ok(())
    }
    fn current_gains(&self) -> Vec<(String, f64)> {
        self.calls.note("current_gains");
        vec![("LNA".into(), 12.0)]
    }
    fn current_antenna(&self) -> String {
        self.calls.note("current_antenna");
        "RX".into()
    }
    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        self.calls.note("tx_begin");
        // Handed back so the test can see which domain it arrived in.
        Ok(center_hz)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        self.calls.note("tx_write");
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        self.calls.note("tx_end");
        Ok(())
    }
    fn set_tx_gain_element(&mut self, _name: &str, _db: f64) -> Result<()> {
        self.calls.note("set_tx_gain_element");
        Ok(())
    }
    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        self.calls.note("current_tx_gains");
        vec![("PA".into(), 0.0)]
    }
    fn set_tx_antenna(&mut self, _name: &str) -> Result<()> {
        self.calls.note("set_tx_antenna");
        Ok(())
    }
    fn current_tx_antenna(&self) -> String {
        self.calls.note("current_tx_antenna");
        "TX".into()
    }
    fn set_tx_drive(&mut self, _frac: f64) {
        self.calls.note("set_tx_drive");
    }
    fn set_tune_drive(&mut self, _frac: f64) {
        self.calls.note("set_tune_drive");
    }
    fn commands_tx_power(&self) -> bool {
        self.calls.note("commands_tx_power");
        true
    }
    fn tx_telemetry(&mut self) -> Option<sdroxide_types::TxTelemetry> {
        self.calls.note("tx_telemetry");
        Some(sdroxide_types::TxTelemetry { fwd_w: Some(5.0), swr: Some(1.2), alc: None, po: None })
    }
    fn set_if_offset(&mut self, _hz: f64) {
        self.calls.note("set_if_offset");
    }
    fn display_bandwidth(&self) -> Option<f64> {
        self.calls.note("display_bandwidth");
        Some(3000.0)
    }
    fn wide_spectrum_db(&mut self, out: &mut Vec<f32>) -> Option<(f64, f64)> {
        self.calls.note("wide_spectrum_db");
        out.push(-90.0);
        Some((self.hw_center, 32_000_000.0))
    }
    fn poll_control(&mut self) -> Vec<sdroxide_radio::ControlUpdate> {
        self.calls.note("poll_control");
        vec![
            sdroxide_radio::ControlUpdate::Freq(self.hw_center),
            sdroxide_radio::ControlUpdate::Mode(Mode::Usb),
        ]
    }
    fn set_control_mode(&mut self, _mode: Mode) -> Result<()> {
        self.calls.note("set_control_mode");
        Ok(())
    }
    fn set_rit_hz(&mut self, _hz: f64) {
        self.calls.note("set_rit_hz");
    }
    fn tx_write_audio(&mut self, _audio: &[f32]) -> Result<()> {
        self.calls.note("tx_write_audio");
        Ok(())
    }
    fn tx_drain(&mut self) {
        self.calls.note("tx_drain");
    }
    fn open_status(&self) -> Option<String> {
        self.calls.note("open_status");
        Some("bias tee is off".into())
    }
    fn needs_reopen(&self) -> bool {
        self.calls.note("needs_reopen");
        true
    }
    fn release(&mut self) {
        self.calls.note("release");
        self.released.store(true, Ordering::SeqCst);
    }
}

/// The wrapper must be transparent. Every method has a default, so one that is
/// not forwarded compiles happily and then quietly reverts to it — which is how
/// a converter would take away the RX-888's full-band strip, the reconnect
/// loop, or the ability to release a USB dongle on Apply.
#[test]
fn every_call_reaches_the_front_end_underneath() {
    let calls = Calls::default();
    let seen = Calls(Arc::clone(&calls.0));
    let released = Arc::new(AtomicBool::new(false));
    let inner = Recorder { calls, hw_center: DIAL + OFFSET, released: Arc::clone(&released) };
    // With a transmit line stated — otherwise transmit is withdrawn and half
    // these calls have nowhere to go, which is a different test.
    let mut s = ConvertedSource::new(Box::new(inner), OFFSET, Some(0.0));

    // Translated, and checked for the direction of the translation.
    assert!((s.center_hz() - DIAL).abs() < 1.0);
    s.set_center_hz(DIAL + 1000.0).unwrap();
    assert!((s.center_hz() - (DIAL + 1000.0)).abs() < 1.0, "round trip through the wrapper");
    // The other VFO is on the operator's side of the converter too, and reaches
    // the rig with the offset added, not taken away.
    s.select_vfo(Vfo::B, DIAL + 2000.0);
    assert!(
        (s.center_hz() - (DIAL + 2000.0)).abs() < 1.0,
        "select_vfo converts the way set_center_hz does, got {}",
        s.center_hz()
    );
    s.set_center_hz(DIAL + 1000.0).unwrap();
    assert!(s.describe().contains("recorder"), "the inner description survives");
    assert!(s.describe().contains("converter"), "and says a converter is in circuit");
    let mut bins = Vec::new();
    let (wide_center, wide_span) = s.wide_spectrum_db(&mut bins).expect("a wide frame");
    assert!(
        (wide_center - (DIAL + 1000.0)).abs() < 1.0,
        "the full-band axis is operator-domain too, got {wide_center}"
    );
    assert_eq!(wide_span, 32_000_000.0, "the span is a width, not a frequency");
    assert_eq!(bins, vec![-90.0]);
    match s.poll_control().as_slice() {
        [
            sdroxide_radio::ControlUpdate::Freq(hz),
            sdroxide_radio::ControlUpdate::Mode(Mode::Usb),
        ] => {
            assert!((hz - (DIAL + 1000.0)).abs() < 1.0, "a rig-reported dial comes back translated")
        }
        other => panic!("unexpected control updates: {other:?}"),
    }
    // Translated by the transmit offset, which here is nought: a receive
    // converter with the transmitter on its own antenna.
    assert_eq!(s.tx_begin(DIAL, RATE).unwrap(), DIAL);

    // Forwarded verbatim — the answers prove they came from the recorder and
    // not from the trait's default.
    assert_eq!(s.sample_rate(), RATE);
    assert_eq!(s.lo_offset_hz(), 600_000.0);
    assert_eq!(s.read(&mut []).unwrap(), 0);
    s.set_gain_element("LNA", 12.0).unwrap();
    s.set_antenna("RX").unwrap();
    assert_eq!(s.current_gains(), vec![("LNA".to_string(), 12.0)]);
    assert_eq!(s.current_antenna(), "RX");
    s.tx_write(&[]).unwrap();
    s.tx_end().unwrap();
    s.set_tx_gain_element("PA", 0.0).unwrap();
    assert_eq!(s.current_tx_gains(), vec![("PA".to_string(), 0.0)]);
    s.set_tx_antenna("TX").unwrap();
    assert_eq!(s.current_tx_antenna(), "TX");
    s.set_tx_drive(0.5);
    s.set_tune_drive(0.05);
    assert!(s.commands_tx_power());
    assert_eq!(s.tx_telemetry().and_then(|t| t.swr), Some(1.2));
    s.set_if_offset(1000.0);
    assert_eq!(s.display_bandwidth(), Some(3000.0));
    s.set_control_mode(Mode::Usb).unwrap();
    s.set_rit_hz(-50.0);
    s.tx_write_audio(&[]).unwrap();
    s.tx_drain();
    assert_eq!(s.open_status().as_deref(), Some("bias tee is off"));
    assert!(s.needs_reopen());
    s.release();
    assert!(released.load(Ordering::SeqCst));

    // And nothing was answered from the wrapper's own head.
    let missed: Vec<_> = [
        "sample_rate",
        "center_hz",
        "set_center_hz",
        "select_vfo",
        "lo_offset_hz",
        "read",
        "describe",
        "set_gain_element",
        "set_antenna",
        "current_gains",
        "current_antenna",
        "tx_begin",
        "tx_write",
        "tx_end",
        "set_tx_gain_element",
        "current_tx_gains",
        "set_tx_antenna",
        "current_tx_antenna",
        "set_tx_drive",
        "set_tune_drive",
        "commands_tx_power",
        "tx_telemetry",
        "set_if_offset",
        "display_bandwidth",
        "wide_spectrum_db",
        "poll_control",
        "set_control_mode",
        "set_rit_hz",
        "tx_write_audio",
        "tx_drain",
        "open_status",
        "needs_reopen",
        "release",
    ]
    .into_iter()
    .filter(|m| !seen.seen().contains(m))
    .collect();
    assert!(missed.is_empty(), "these never reached the front end: {missed:?}");
}

/// Three back ends answer `Ok` to any frequency and then clamp inside their own
/// DDC, so a request that lands below DC has to be stopped here — otherwise the
/// engine believes the tune worked while the receiver sits somewhere else.
#[test]
fn a_frequency_below_dc_after_the_offset_is_refused() {
    let calls = Calls::default();
    let inner =
        Recorder { calls, hw_center: 28_000_000.0, released: Arc::new(AtomicBool::new(false)) };
    // A 2 m transverter with a 28 MHz IF.
    let mut s = ConvertedSource::new(Box::new(inner), -116_000_000.0, Some(-116_000_000.0));
    assert!(s.set_center_hz(144_000_000.0).is_ok(), "in range");
    let err = s.set_center_hz(10_000_000.0).expect_err("below DC once translated");
    assert!(format!("{err}").contains("below DC"), "the reason should say so, got {err}");
}

/// Where the operator's dial sits before they tell sdroxide about the LNB:
/// on the converter's output, which is what the receiver is actually hearing.
const IF_HZ: f64 = DOWNLINK - 55_420.0 + LNB;

/// The arithmetic on its own. A dial that is already in the operator's domain
/// takes the offset; one that predates the converter cannot, and is opened on
/// as the hardware frequency it is.
#[test]
fn the_open_frequency_is_the_dial_plus_the_offset_until_that_is_below_dc() {
    // An upconverter: the dial is the operator's and the hardware goes up.
    assert_eq!(converter_open_hz(DIAL, OFFSET), DIAL + OFFSET);
    // An LNB, with the dial already on the downlink: likewise.
    assert_eq!(converter_open_hz(DOWNLINK, LNB), DOWNLINK + LNB);
    // …and with the dial still on the LNB's output, which is where every
    // operator is at the moment they set the converter up. Below DC is not a
    // frequency, so the number is read as the hardware's own and opened on.
    assert_eq!(converter_open_hz(IF_HZ, LNB), IF_HZ);
}

/// The bug, as the operator meets it: listening to the LNB's output on
/// 739.494 MHz, they open Settings → Radio, pick "LNB, Ku low" and press Apply.
///
/// What must not happen is the radio going deaf — the front end asked for
/// −9010 MHz, clamping to the bottom of its range, and the dial left outside
/// the converted receive range where every tune the operator makes is refused
/// as "outside this radio's receive range" and put back where it already was.
/// What must happen instead is a re-labelling: the receiver does not move, and
/// the signal it is on gets the number the operator meant when they said there
/// was a 9750 MHz LNB in front of it.
#[test]
fn a_converter_switched_on_under_a_running_radio_relabels_the_dial() {
    let landed = Arc::new(Mutex::new(0.0));
    let hw = Arc::clone(&landed);
    // Settings → Radio → Apply, the way `open_converted_source` does it.
    let reopen: ReopenFn = Box::new(move |dial: f64| {
        let open_at = converter_open_hz(dial, LNB);
        *hw.lock().unwrap() = open_at;
        let inner = Hardware { landed: Arc::clone(&hw), ..Hardware::new(open_at) };
        Ok((
            Box::new(ConvertedSource::new(Box::new(inner), LNB, Some(0.0))) as Box<dyn IqSource>,
            shift_caps(qo100_caps(), LNB, Some(0.0)),
        ))
    });

    // Before Apply: no converter, the dial on the 739 MHz the LNB is delivering.
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(Hardware::new(IF_HZ)), qo100_caps(), cfg);
    let thread = h.thread.take();
    h.swap_tx.send(EngineSwap::ReopenSource).expect("the engine is running");
    let (notices, dial, _) = drain(&h.event_rx, 1.5);

    let want = IF_HZ - LNB;
    assert!(
        !notices.iter().any(|n| n.contains("outside")),
        "nothing here is out of range, got {notices:?}"
    );
    assert!((dial - want).abs() < 1.0, "the dial should read {want}, not {dial}");
    assert!(
        (*landed.lock().unwrap() - IF_HZ).abs() < 1.0,
        "the receiver was already on the signal and must not have moved, it is on {}",
        *landed.lock().unwrap()
    );

    // And the radio still tunes: the whole complaint was that it stopped. The
    // downlink is a few tens of kHz away, so this one is the DDC's to make…
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DOWNLINK }).unwrap();
    let (notices, dial, _) = drain(&h.event_rx, 1.0);
    assert!(notices.is_empty(), "the downlink is reachable through the LNB, got {notices:?}");
    assert!((dial - DOWNLINK).abs() < 1.0, "the dial should read {DOWNLINK}, not {dial}");

    // …and one right out of the span is the receiver's, which has to land on the
    // I.F. the LNB delivers rather than on the 10 GHz number the dial shows.
    let far = DOWNLINK + 3_000_000.0;
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: far }).unwrap();
    let (notices, dial, _) = drain(&h.event_rx, 1.0);
    assert!(notices.is_empty(), "still inside the LNB's reach, got {notices:?}");
    assert!((dial - far).abs() < 1.0, "the dial should read {far}, not {dial}");
    assert!(
        (*landed.lock().unwrap() - (far + LNB)).abs() < 1.0,
        "the hardware should be on the I.F., it is on {}",
        *landed.lock().unwrap()
    );

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// The same trap the other way round: the converter is switched *off* while the
/// dial is on 10489 MHz, and the radio behind it hears to 6 GHz. A front end
/// that cannot reach the dial takes it along to where it can — a radio one tune
/// away from useful, rather than one that refuses every tune it is given and
/// has to be typed a frequency out of thin air.
#[test]
fn a_swap_to_a_front_end_that_cannot_reach_the_dial_moves_it() {
    // No converter this time, and a front end that answers with the frequency
    // it was handed rather than the one it clamped to — which is what a Pluto,
    // an RTL-SDR and an HPSDR all do.
    let reopen: ReopenFn = Box::new(|dial: f64| {
        Ok((Box::new(Hardware::new(dial)) as Box<dyn IqSource>, qo100_caps()))
    });

    let inner = Hardware::new(DOWNLINK + LNB);
    let source = ConvertedSource::new(Box::new(inner), LNB, Some(0.0));
    let cfg = EngineConfig { reopen: Some(reopen), ..Default::default() };
    let mut h = start_engine(Box::new(source), shift_caps(qo100_caps(), LNB, Some(0.0)), cfg);
    let thread = h.thread.take();
    h.swap_tx.send(EngineSwap::ReopenSource).expect("the engine is running");
    let (notices, dial, center) = drain(&h.event_rx, 1.5);

    let top = qo100_caps().freq_ranges_rx[0].1;
    assert!(
        notices.iter().any(|n| n.contains("the dial moved to")),
        "the operator has to be told their dial moved, got {notices:?}"
    );
    assert!((dial - top).abs() < 1.0, "the dial should be at the top of the range, got {dial}");
    assert!((center - top).abs() < 1.0, "and the hardware with it, got {center}");

    // The point of moving it: the radio tunes again.
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: 1_000_000_000.0 }).unwrap();
    let (notices, dial, center) = drain(&h.event_rx, 1.0);
    assert!(notices.is_empty(), "1 GHz is inside the range, got {notices:?}");
    assert!((dial - 1_000_000_000.0).abs() < 1.0, "the dial should read 1 GHz, got {dial}");
    assert!((center - 1_000_000_000.0).abs() < 1.0, "and the receiver with it, got {center}");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// A Hermes Lite 2 behind a 2 m transverter: the radio covers DC to 38.4 MHz,
/// the transverter brings 144-148 MHz down to a 28 MHz I.F., and the operator
/// types the band they tune into the RX range box.
///
/// Issue #279: that range used to be read as a *hardware* one and shifted by
/// the offset along with the device's, so 144-148 with a −116 MHz transverter
/// came out as 260-264 MHz and the radio refused every frequency in the band it
/// had just been told about.
#[test]
fn a_stated_range_is_on_the_dial_not_the_intermediate_frequency() {
    /// 28 MHz I.F. for 144 MHz on the air.
    const XVTR: f64 = -116_000_000.0;
    let hl2 = DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 38_400_000.0)],
        freq_ranges_tx: vec![(0.0, 38_400_000.0)],
        antennas_tx: vec!["TX/RX".into()],
        ..DeviceCaps::default()
    };
    let band = [(144_000_000.0, 148_000_000.0)];

    let caps = sdroxide_radio::converted_caps(hl2.clone(), XVTR, Some(XVTR), &band, &band);
    assert_eq!(
        caps.freq_ranges_rx, band,
        "the band the operator typed is the band they get, offset or no offset"
    );
    assert_eq!(caps.freq_ranges_tx, band, "and the same on transmit, through the same transverter");

    // Nothing stated: the device's own answer is the one that moves, because
    // that one really is in the hardware's domain. 0-38.4 MHz through a
    // −116 MHz transverter is 116-154.4 MHz on the dial.
    let device_only = sdroxide_radio::converted_caps(hl2.clone(), XVTR, Some(XVTR), &[], &[]);
    assert_eq!(device_only.freq_ranges_rx, vec![(116_000_000.0, 154_400_000.0)]);

    // And a stated transmit range cannot hand a transmitter back to an operator
    // who said there is nothing in the transmit line.
    let rx_only = sdroxide_radio::converted_caps(hl2, XVTR, None, &band, &band);
    assert_eq!(rx_only.freq_ranges_rx, band);
    assert!(rx_only.freq_ranges_tx.is_empty(), "transmit was withdrawn and must stay withdrawn");
}

/// A station with two transverters on one I.F., which is what issue #278 asked
/// for: 2 m and 6 m from two boxes into the same 28 MHz receiver, HF straight
/// in, and each band with its own offset, transmit rule and drive limit.
#[test]
fn a_transverter_table_gives_each_band_its_own_offset() {
    use sdroxide_radio::{ConverterPlan, ConverterStep};

    let plan = ConverterPlan::from_steps([
        // 6 m into 28 MHz: 28 − 50 = −22.
        ConverterStep {
            band: Some((50_000_000.0, 54_000_000.0)),
            rx_offset_hz: -22_000_000.0,
            tx_offset_hz: Some(-22_000_000.0),
            tx_drive: Some(0.05),
        },
        // 2 m into the same I.F.: 28 − 144 = −116.
        ConverterStep {
            band: Some((144_000_000.0, 148_000_000.0)),
            rx_offset_hz: -116_000_000.0,
            tx_offset_hz: Some(-116_000_000.0),
            tx_drive: Some(0.02),
        },
    ]);

    // Each band goes to the same I.F. through a different offset.
    assert_eq!(plan.offset_for(50_150_000.0), -22_000_000.0);
    assert_eq!(plan.offset_for(144_300_000.0), -116_000_000.0);
    // ...and HF is the radio, untouched, which is the whole reason the table
    // is bands rather than one number.
    assert_eq!(plan.offset_for(14_200_000.0), 0.0);
    assert_eq!(plan.step_for(14_200_000.0).tx_offset_hz, Some(0.0), "HF still transmits");
    assert_eq!(plan.step_for(14_200_000.0).tx_drive, None, "and at whatever drive was set");

    // A drive ceiling belongs to the band, not to the station.
    assert_eq!(plan.step_for(144_300_000.0).tx_drive, Some(0.02));

    // The published ranges are the union: the radio's own HF, plus each
    // transverter's band as far as the radio can reach through it.
    let hl2 = DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 38_400_000.0)],
        freq_ranges_tx: vec![(0.0, 38_400_000.0)],
        antennas_tx: vec!["TX/RX".into()],
        ..DeviceCaps::default()
    };
    let caps = sdroxide_radio::plan_caps(hl2, &plan, &[], &[]);
    for hz in [1_840_000.0, 14_200_000.0, 50_150_000.0, 144_300_000.0] {
        assert!(caps.may_rx_hz(hz), "{hz} should be reachable");
        assert!(caps.may_tx_hz(hz), "{hz} should be transmittable");
    }
    // 70 cm is reachable through neither: no transverter names it and the
    // radio stops at 38.4 MHz.
    assert!(!caps.may_rx_hz(435_000_000.0));
    // Nor is the *I.F.* still on the dial where a transverter has taken the
    // band over — 28 MHz is 6 m's I.F., but the radio reaches it directly too,
    // so it stays; what must not appear is 2 m's band through the 6 m box.
    assert!(caps.may_rx_hz(28_500_000.0), "the radio's own 10 m is still there");
}

/// The band-selected offset has to reach the source, not just the plan: a dial
/// moved from one transverter's band to another retunes the same radio to a
/// different intermediate frequency.
#[test]
fn moving_between_transverters_retunes_the_same_radio() {
    use sdroxide_radio::{ConverterPlan, ConverterStep};

    let inner = Hardware::new(28_150_000.0);
    let landed = Arc::clone(&inner.landed);
    let plan = ConverterPlan::from_steps([
        ConverterStep {
            band: Some((50_000_000.0, 54_000_000.0)),
            rx_offset_hz: -22_000_000.0,
            tx_offset_hz: Some(-22_000_000.0),
            tx_drive: Some(0.05),
        },
        ConverterStep {
            band: Some((144_000_000.0, 148_000_000.0)),
            rx_offset_hz: -116_000_000.0,
            tx_offset_hz: Some(-116_000_000.0),
            tx_drive: Some(0.02),
        },
    ]);
    let mut source = ConvertedSource::with_plan(Box::new(inner), plan);

    source.set_center_hz(50_150_000.0).expect("6 m");
    assert!((*landed.lock().unwrap() - 28_150_000.0).abs() < 1.0);
    assert!((source.center_hz() - 50_150_000.0).abs() < 1.0, "and reads back as 6 m");
    assert_eq!(source.tx_drive_ceiling(), Some(0.05));

    source.set_center_hz(144_300_000.0).expect("2 m");
    assert!((*landed.lock().unwrap() - 28_300_000.0).abs() < 1.0);
    assert!((source.center_hz() - 144_300_000.0).abs() < 1.0, "and reads back as 2 m");
    assert_eq!(source.tx_drive_ceiling(), Some(0.02));

    // HF is the radio itself: no offset, no ceiling.
    source.set_center_hz(14_200_000.0).expect("20 m");
    assert!((*landed.lock().unwrap() - 14_200_000.0).abs() < 1.0);
    assert_eq!(source.tx_drive_ceiling(), None);
}

/// A radio opened on a transverter's I.F. has to come up on the *band*, not on
/// the I.F.: the engine reads `center_hz()` before it has commanded anything,
/// and a source that answered 28 MHz would put the dial there.
#[test]
fn a_source_opened_on_an_intermediate_frequency_comes_up_on_the_band() {
    use sdroxide_radio::{ConverterPlan, ConverterStep};

    let plan = ConverterPlan::from_steps([ConverterStep {
        band: Some((144_000_000.0, 148_000_000.0)),
        rx_offset_hz: -116_000_000.0,
        tx_offset_hz: Some(-116_000_000.0),
        tx_drive: Some(0.02),
    }]);
    // Opened where `converter_open_hz` would have put it for a 144.300 dial.
    let inner = Hardware::new(28_300_000.0);
    let band_dial = Arc::clone(&inner.band_dial);
    let source = ConvertedSource::with_plan(Box::new(inner), plan.clone());
    assert_eq!(*band_dial.lock().unwrap(), Some(144_300_000.0));
    assert!(
        (source.center_hz() - 144_300_000.0).abs() < 1.0,
        "came up on {} rather than on 2 m",
        source.center_hz()
    );

    // An I.F. no transverter can account for is the radio's own frequency and
    // is left alone — that is 10 m, not a mis-seeded 2 m.
    let source = ConvertedSource::with_plan(Box::new(Hardware::new(14_200_000.0)), plan);
    assert!((source.center_hz() - 14_200_000.0).abs() < 1.0);
}

#[test]
fn startup_and_reopen_initialize_the_selected_transverter_without_a_band_change() {
    use sdroxide_radio::{ConverterPlan, ConverterStep, plan_caps};

    const RF: f64 = 144_174_000.0;
    const IF: f64 = 28_174_000.0;
    let plan = ConverterPlan::from_steps([
        ConverterStep {
            band: Some((50_000_000.0, 54_000_000.0)),
            rx_offset_hz: -22_000_000.0,
            tx_offset_hz: None,
            tx_drive: Some(0.05),
        },
        ConverterStep {
            band: Some((144_000_000.0, 148_000_000.0)),
            rx_offset_hz: -116_000_000.0,
            tx_offset_hz: Some(-116_000_000.0),
            tx_drive: Some(0.02),
        },
    ]);
    let inner = Hardware::new(IF);
    let band_dial = Arc::clone(&inner.band_dial);
    let tx_freq = Arc::clone(&inner.tx_freq);
    let keyed = Arc::clone(&inner.keyed);
    let source = ConvertedSource::with_plan_at_dial(Box::new(inner), plan.clone(), RF);
    assert_eq!(source.center_hz(), RF, "the shared IF must not select the first row");
    assert_eq!(source.tx_drive_ceiling(), Some(0.02));
    assert_eq!(*band_dial.lock().unwrap(), Some(RF), "initialize before any tune");

    for dial in [IF, 50_174_000.0] {
        let source =
            ConvertedSource::with_plan_at_dial(Box::new(Hardware::new(IF)), plan.clone(), dial);
        assert_eq!(source.center_hz(), dial, "HF and the first transverter still open correctly");
    }

    let caps = plan_caps(hardware_caps(), &plan, &[], &[]);
    let reopen_caps = caps.clone();
    let reopen_dial = Arc::clone(&band_dial);
    let reopen_tx = Arc::clone(&tx_freq);
    let reopen_keyed = Arc::clone(&keyed);
    let reopen: ReopenFn = Box::new(move |dial| {
        assert_eq!(dial, RF, "Apply must retain the selected transverter");
        *reopen_dial.lock().unwrap() = None;
        *reopen_tx.lock().unwrap() = None;
        *reopen_keyed.lock().unwrap() = None;
        let inner = Hardware {
            band_dial: Arc::clone(&reopen_dial),
            tx_freq: Arc::clone(&reopen_tx),
            keyed: Arc::clone(&reopen_keyed),
            ..Hardware::new(IF)
        };
        Ok((
            Box::new(ConvertedSource::with_plan_at_dial(Box::new(inner), plan.clone(), dial)),
            reopen_caps.clone(),
        ))
    });
    let mut h = start_engine(
        Box::new(source),
        caps,
        EngineConfig { initial_mode: Some(Mode::Cw), reopen: Some(reopen), ..Default::default() },
    );
    let thread = h.thread.take();

    for reopened in [false, true] {
        if reopened {
            h.swap_tx.send(EngineSwap::ReopenSource).unwrap();
        }
        let (notices, dial, _) = drain(&h.event_rx, 0.5);
        assert!(notices.is_empty(), "startup/reopen notices: {notices:?}");
        assert_eq!(dial, RF);
        assert_eq!(*band_dial.lock().unwrap(), Some(RF));
        assert_eq!(*tx_freq.lock().unwrap(), Some(IF));
        // Protocol 1 reconstructs the IO board's TX frequency from this offset.
        assert_eq!(tx_freq.lock().unwrap().unwrap() + band_dial.lock().unwrap().unwrap() - IF, RF);

        h.cmd_tx.send(Command::SetPtt(true)).unwrap();
        let (notices, _, _) = drain(&h.event_rx, 0.5);
        assert!(notices.is_empty(), "PTT must work without leaving the band: {notices:?}");
        assert_eq!(*keyed.lock().unwrap(), Some(IF));
        h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    }

    drop(h.cmd_tx);
    if let Some(t) = thread {
        t.join().unwrap();
    }
}
