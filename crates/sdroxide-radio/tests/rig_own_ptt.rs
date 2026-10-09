//! An over the operator started at the radio, not in sdroxide.
//!
//! A transceiver has its own microphone, and pressing the button on it is a
//! transmission sdroxide never asked for. Two things have to follow from that,
//! and they pull in opposite directions.
//!
//! It has to be *noticed*: the meter belongs to whatever is on the air, so it
//! goes to transmit and shows the SWR of that over rather than sitting on a
//! receive signal that stopped existing when the transmitter came up.
//!
//! And it must not be *joined*. [`ControlUpdate::RigTx`] is not
//! [`ControlUpdate::Ptt`] — there is no PTT line here to follow, there is a
//! radio already modulating somebody's voice. Running the transmit chain into
//! it would put the computer's audio on the air underneath theirs, and on a rig
//! whose key-down doubles as an audio-source switch (a Kenwood's `TX1;` is DATA
//! SEND) would cut them off mid-word.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, ControlUpdate, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioEvent, TxTelemetry};

const RATE: f64 = 48_000.0;
const DIAL: f64 = 14_074_000.0;
/// What the rig's SWR bridge reads during the operator's over.
const SWR: f32 = 1.7;

#[derive(Default)]
struct Rig {
    /// Queued for the next `poll_control`, standing in for the CAT thread
    /// noticing the rig's transmit flag.
    report: Option<bool>,
    /// Whether the rig says it is transmitting right now.
    keyed: bool,
    /// Every frequency `tx_begin` was called with: the transmit chain running.
    keyed_by_us: Vec<f64>,
    unkeyed_by_us: usize,
    lan_input: bool,
    audio: Vec<f32>,
}

struct MockRig {
    rig: Arc<Mutex<Rig>>,
}

impl IqSource for MockRig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        DIAL
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
    /// The rig's own S-meter, as every CAT family that has one reports it.
    /// Here only so there is always a *receive* meter to be replaced: what the
    /// test watches is which meter the engine publishes, not what it reads.
    fn rx_signal_dbm(&mut self) -> Option<f32> {
        Some(-93.0)
    }
    fn describe(&self) -> String {
        "mock rig with its own microphone".into()
    }
    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        match self.rig.lock().unwrap().report.take() {
            Some(on) => vec![ControlUpdate::RigTx(on)],
            None => Vec::new(),
        }
    }
    /// The rig's SWR bridge — it reads whoever is keying it, which during the
    /// operator's own over is the operator.
    fn tx_telemetry(&mut self) -> Option<TxTelemetry> {
        // An SWR bridge and nothing else: this rig reports no ALC meter, so
        // `alc` stays absent rather than being invented as a zero reading.
        let t = TxTelemetry { fwd_w: None, swr: Some(SWR), alc: None, po: None };
        self.rig.lock().unwrap().keyed.then_some(t)
    }
    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        self.rig.lock().unwrap().keyed_by_us.push(center_hz);
        Ok(RATE)
    }
    /// Take the blocks, and do nothing with them.
    ///
    /// Not optional, and not decoration. This rig declares `tx_channels: 1`, so
    /// a source without these is one that says it can transmit and then refuses
    /// the first block — which the engine treats, correctly and deliberately,
    /// as the link having gone: it reports `ConnectionLost`, unkeys and stops.
    /// The test would then be racing that shutdown for its next command, and
    /// lost about one run in three. What it is measuring is which frequencies
    /// reached `tx_begin`; the blocks after it are not its business, but they
    /// have to be accepted for the engine to still be there to ask.
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        Ok(())
    }
    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        self.rig.lock().unwrap().audio.extend_from_slice(audio);
        Ok(())
    }
    fn feeds_audio_on_rig_tx(&self) -> bool {
        self.rig.lock().unwrap().lan_input
    }
    fn tx_end(&mut self) -> Result<()> {
        self.rig.lock().unwrap().unkeyed_by_us += 1;
        Ok(())
    }
}

#[test]
fn local_ptt_feeds_host_voice_only_when_the_radio_selects_lan_audio() {
    use sdroxide_radio::{MicParams, rtrb::RingBuffer};

    let rig = Arc::new(Mutex::new(Rig::default()));
    let (mut mic, consumer) = RingBuffer::new(4096);
    let mut c = caps();
    c.tx_audio = true;
    let h = start_engine(
        Box::new(MockRig { rig: Arc::clone(&rig) }),
        c,
        EngineConfig {
            tx_ham_only: false,
            mic: Some(MicParams { consumer, rate: RATE }),
            ..Default::default()
        },
    );
    let t = Harness { h, rig };
    t.h.cmd_tx
        .send(Command::SetMode { rx: sdroxide_types::RxId::Main, mode: sdroxide_types::Mode::Usb })
        .unwrap();
    t.wait("receive meter", |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_none()));
    t.rig_keys(true);
    t.wait("local transmit meter", |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_some()));
    assert!(t.rig.lock().unwrap().audio.is_empty(), "host audio leaked into radio mic mode");

    t.rig.lock().unwrap().lan_input = true;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while mic.push(0.2).is_ok() {}
        if t.rig.lock().unwrap().audio.iter().any(|a| a.abs() > 0.01) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    {
        let r = t.rig.lock().unwrap();
        assert!(r.audio.iter().any(|a| a.abs() > 0.01), "LAN mode never received computer voice");
        assert!(r.keyed_by_us.is_empty(), "audio following sent radio key-down");
    }
    t.rig.lock().unwrap().lan_input = false;
    std::thread::sleep(Duration::from_millis(80));
    let samples = t.rig.lock().unwrap().audio.len();
    std::thread::sleep(Duration::from_millis(80));
    assert_eq!(t.rig.lock().unwrap().audio.len(), samples, "voice continued in radio mic mode");
    t.rig.lock().unwrap().lan_input = true;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && t.rig.lock().unwrap().audio.len() == samples {
        while mic.push(0.2).is_ok() {}
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(t.rig.lock().unwrap().audio.len() > samples, "voice did not resume in LAN mode");
    t.rig_keys(false);
    t.wait(
        "receive after local release",
        |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_none()),
    );
    let samples = t.rig.lock().unwrap().audio.len();
    std::thread::sleep(Duration::from_millis(80));
    let r = t.rig.lock().unwrap();
    assert_eq!(r.audio.len(), samples, "voice continued after local release");
    assert_eq!(r.unkeyed_by_us, 0, "audio following sent radio key-up");
    drop(r);
    t.shutdown();
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

struct Harness {
    h: sdroxide_radio::EngineHandles,
    rig: Arc<Mutex<Rig>>,
}

fn engine() -> Harness {
    let rig = Arc::new(Mutex::new(Rig::default()));
    let h = start_engine(
        Box::new(MockRig { rig: Arc::clone(&rig) }),
        caps(),
        EngineConfig { tx_ham_only: false, ..Default::default() },
    );
    h.cmd_tx.send(Command::SetVfo { vfo: sdroxide_types::Vfo::A, hz: DIAL }).unwrap();
    Harness { h, rig }
}

impl Harness {
    /// The rig keys or unkeys itself, and the source reports it on its next poll.
    fn rig_keys(&self, on: bool) {
        let mut r = self.rig.lock().unwrap();
        r.keyed = on;
        r.report = Some(on);
    }

    /// Wait for an event satisfying `f`, or give up.
    fn wait(&self, what: &str, mut f: impl FnMut(&RadioEvent) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            while let Ok(ev) = self.h.event_rx.try_recv() {
                if f(&ev) {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out waiting for {what}");
    }

    fn drain(&self) {
        while self.h.event_rx.try_recv().is_ok() {}
    }

    fn shutdown(mut self) {
        let thread = self.h.thread.take();
        drop(self.h.cmd_tx);
        if let Some(t) = thread {
            let _ = t.join();
        }
    }
}

/// The meter follows whatever is on the air. An over the operator keyed at the
/// radio is on the air, so the needle goes to transmit and reads its SWR — and
/// comes back to receive when they let go.
#[test]
fn external_ptt_is_published_as_status_without_commanding_ptt() {
    let t = engine();
    t.wait("initial state", |ev| matches!(ev, RadioEvent::State(_)));
    t.rig_keys(true);
    t.wait(
        "external PTT indication",
        |ev| matches!(ev, RadioEvent::State(s) if s.tx.external_ptt && s.tx.ptt_on() && !s.tx.ptt),
    );
    assert!(t.rig.lock().unwrap().keyed_by_us.is_empty());
    t.rig_keys(false);
    t.wait(
        "external PTT indicator release",
        |ev| matches!(ev, RadioEvent::State(s) if !s.tx.external_ptt && !s.tx.ptt_on()),
    );
    assert_eq!(t.rig.lock().unwrap().unkeyed_by_us, 0);
    t.shutdown();
}

#[test]
fn the_meter_follows_an_over_the_operator_keyed_at_the_radio() {
    let t = engine();
    t.wait("a receive meter", |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_none()));

    t.rig_keys(true);
    t.wait("the meter to go to transmit", |ev| match ev {
        RadioEvent::Meters(m) => m.tx.is_some_and(|tx| tx.swr == Some(SWR)),
        _ => false,
    });

    t.rig_keys(false);
    t.wait(
        "the meter to come back to receive",
        |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_none()),
    );
    t.shutdown();
}

/// The other half, and the one with teeth: noticing the over must not join it.
/// Nothing keys, and a PTT arriving meanwhile is refused rather than queued —
/// an over that starts the moment the operator lets go of their microphone is
/// its own surprise.
#[test]
fn sdroxide_neither_keys_along_nor_transmits_into_it() {
    let t = engine();
    t.wait("the first meter", |ev| matches!(ev, RadioEvent::Meters(_)));
    t.rig_keys(true);
    t.wait(
        "the meter to go to transmit",
        |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_some()),
    );
    // Whatever else happened, the transmit chain never ran: noticing the rig's
    // own over is not a route into `tx_begin`.
    assert!(
        t.rig.lock().unwrap().keyed_by_us.is_empty(),
        "sdroxide transmitted into an over it did not key"
    );

    t.drain();
    t.h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    t.wait("the refusal", |ev| matches!(ev, RadioEvent::Notice(Some(s)) if s.contains("its own")));
    assert!(
        t.rig.lock().unwrap().keyed_by_us.is_empty(),
        "a refused key-down still reached the transmitter"
    );

    // And it is a refusal, not a lock: the operator lets go, and sdroxide's own
    // PTT works again.
    t.rig_keys(false);
    t.wait(
        "the meter to come back to receive",
        |ev| matches!(ev, RadioEvent::Meters(m) if m.tx.is_none()),
    );
    t.h.cmd_tx.send(Command::SetPtt(true)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && t.rig.lock().unwrap().keyed_by_us.is_empty() {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        t.rig.lock().unwrap().keyed_by_us.as_slice(),
        &[DIAL],
        "sdroxide could not transmit after the operator's over ended"
    );
    t.h.cmd_tx.send(Command::SetPtt(false)).unwrap();
    t.shutdown();
}
