//! Working a repeater (issue #137): the transmit shift, and the CTCSS tone
//! that has to be under the voice for the repeater to open at all.
//!
//! The shift is checked where it matters — at the *source*, which is what a
//! band-switching accessory and the transmitter itself are told — rather than
//! only in the broadcast state, because a shift that never leaves the state is
//! a number on a screen.
//!
//! The tone is checked by listening to it. The engine's transmit path is a
//! modulator into a DUC, so what the mock front end is handed is real
//! transmitted I/Q; discriminating it back gives the same signal a repeater's
//! decoder sees, and `SubToneDetect` — which was written against off-air
//! signals — says what is in it.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use sdroxide_dsp::SubToneDetect;
use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{
    Command, DeviceCaps, Mode, RadioEvent, RadioState, Region, RepeaterState, RxId, Shift,
    SubTone, ToneMode, Vfo,
};

/// 48 kHz so the transmit chain's DUC is a straight pass-through and what the
/// mock is handed is the modulator's own output rate — which is the rate the
/// detector below has to be built at.
const RATE: f64 = 48_000.0;
/// A Region 1 2 m repeater output: the plan shifts it 600 kHz down.
const RPT_HZ: f64 = 145_712_500.0;
/// The 2 m calling channel, which is simplex in every plan.
const SIMPLEX_HZ: f64 = 145_500_000.0;

/// Each engine adopts the saved station region during startup. These tests
/// share the process-wide region, so serialize engine startup while a test
/// temporarily selects a region. Own the engine thread here too: on panic, the
/// engine must stop before another test can change the process-wide region.
static TEST_REGION_LOCK: Mutex<()> = Mutex::new(());

struct TestRegionGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<Region>,
    engine_thread: Option<std::thread::JoinHandle<()>>,
}

impl TestRegionGuard {
    fn new() -> Self {
        Self {
            _lock: TEST_REGION_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner),
            previous: None,
            engine_thread: None,
        }
    }

    fn preserve_current(&mut self) {
        self.previous = Some(sdroxide_types::region());
    }

    fn set_region(&mut self, region: Region) {
        self.preserve_current();
        sdroxide_types::set_region(region);
    }

    fn own_engine_thread(&mut self, thread: Option<std::thread::JoinHandle<()>>) {
        self.engine_thread = thread;
    }
}

impl Drop for TestRegionGuard {
    fn drop(&mut self) {
        // The engine may still consult the global region. Let it stop before
        // restoring the saved value and releasing the lock, including on panic.
        if let Some(thread) = self.engine_thread.take() {
            let _ = thread.join();
        }
        if let Some(region) = self.previous.take() {
            sdroxide_types::set_region(region);
        }
    }
}

/// What the front end was told and given.
#[derive(Default)]
struct Heard {
    /// The transmit frequency the engine pushed, most recent last.
    tx_freqs: Vec<f64>,
    /// The transmitted baseband, as handed to `tx_write`.
    iq: Vec<Complex32>,
}

/// A transmit-capable I/Q front end that keeps what it was given.
struct Rig {
    center: f64,
    heard: Arc<Mutex<Heard>>,
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
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "repeater bench".into()
    }
    fn set_tx_freq_hz(&mut self, hz: f64) {
        self.heard.lock().unwrap().tx_freqs.push(hz);
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.heard.lock().unwrap().iq.extend_from_slice(samples);
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "repeater bench".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(1_000_000.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The engine's state once `ready` is happy with it, or a panic saying what it
/// last said instead. Every assertion has to wait for the announcement: a
/// command is acted on when the engine gets round to it.
fn state_where(
    rx: &crossbeam_channel::Receiver<RadioEvent>,
    ready: impl Fn(&RadioState) -> bool,
) -> RadioState {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = None;
    while Instant::now() < deadline {
        if let Ok(RadioEvent::State(s)) = rx.recv_timeout(Duration::from_millis(100)) {
            if ready(&s) {
                return s;
            }
            last = Some(s);
        }
    }
    panic!("the engine never reached the expected state; last: {last:#?}");
}

/// A repeater setup: a minus shift with a CTCSS tone under it, which is what
/// most of the world's machines want.
fn minus_600(tone: ToneMode) -> RepeaterState {
    RepeaterState {
        shift: Shift::Minus,
        offset_hz: 600_000,
        tone,
        ctcss_tenths: 885,
        ..RepeaterState::default()
    }
}

/// The shift moves the transmitter and leaves the receiver where it was — and
/// what moves is what the *front end* is told, not just what the state says.
#[test]
fn the_shift_moves_the_transmitter_and_leaves_the_receiver() {
    let mut region_guard = TestRegionGuard::new();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let mut h = start_engine(
        Box::new(Rig { center: RPT_HZ, heard: Arc::clone(&heard) }),
        caps(),
        EngineConfig { remember_session: false, tx_ham_only: false, ..Default::default() },
    );
    region_guard.preserve_current();
    region_guard.own_engine_thread(h.thread.take());
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: RPT_HZ });
    send(Command::SetRepeater(minus_600(ToneMode::Off)));
    let s = state_where(&h.event_rx, |s| s.repeater.shift == Shift::Minus);
    assert_eq!(s.rx_freq_hz(), RPT_HZ, "the receiver stays on the repeater's output");
    assert_eq!(s.tx_freq_hz(), RPT_HZ - 600_000.0, "the transmitter goes to its input");

    // The front end is told before anything is keyed, which is the whole point
    // of pushing it: a band-switching accessory has to be right before there is
    // any RF.
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if heard.lock().unwrap().tx_freqs.last() == Some(&(RPT_HZ - 600_000.0)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        heard.lock().unwrap().tx_freqs.last().copied(),
        Some(RPT_HZ - 600_000.0),
        "the front end was never told where we would transmit",
    );

    // Back to simplex, and the magnitude survives it: switching a repeater off
    // and on again must not cost the offset that was set for it.
    send(Command::SetRepeater(RepeaterState { shift: Shift::Simplex, ..minus_600(ToneMode::Off) }));
    let s = state_where(&h.event_rx, |s| s.repeater.shift == Shift::Simplex);
    assert_eq!(s.tx_freq_hz(), RPT_HZ);
    assert_eq!(s.repeater.offset_hz, 600_000);

    drop(h);
    drop(region_guard);
}

/// AUTO takes the shift from the band plan inside a repeater sub-band, and
/// leaves the radio simplex outside one — which is what keeps it off the
/// calling channels.
#[test]
fn auto_follows_the_band_plan_and_stays_simplex_outside_it() {
    let mut region_guard = TestRegionGuard::new();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let mut h = start_engine(
        Box::new(Rig { center: RPT_HZ, heard }),
        caps(),
        EngineConfig { remember_session: false, tx_ham_only: false, ..Default::default() },
    );
    // The engine thread adopts the saved station region as it starts; wait for
    // its first state so the override below is not overwritten by it.
    state_where(&h.event_rx, |_| true);
    region_guard.set_region(Region::R1);
    region_guard.own_engine_thread(h.thread.take());
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // Region 1 is the default region, and 145.7125 is one of its repeater
    // outputs.
    send(Command::SetVfo { vfo: Vfo::A, hz: RPT_HZ });
    send(Command::SetRepeater(RepeaterState { auto: true, ..RepeaterState::default() }));
    let s = state_where(&h.event_rx, |s| s.repeater.shift == Shift::Minus);
    assert_eq!(s.repeater.offset_hz, 600_000);
    assert_eq!(s.tx_freq_hz(), RPT_HZ - 600_000.0);

    // The calling channel is not a repeater output, so nothing is shifted onto
    // it — the failure this rule exists to prevent.
    send(Command::SetVfo { vfo: Vfo::A, hz: SIMPLEX_HZ });
    let s = state_where(&h.event_rx, |s| {
        s.active_freq_hz() == SIMPLEX_HZ && s.repeater.shift == Shift::Simplex
    });
    assert_eq!(s.tx_freq_hz(), SIMPLEX_HZ, "AUTO invented a shift on a simplex channel");
    assert!(s.repeater.auto, "AUTO switched itself off");

    drop(h);
    drop(region_guard);
}

/// The CTCSS tone is actually on the air: discriminate the transmitted I/Q and
/// the detector finds the tone that was asked for.
#[test]
fn the_transmitted_over_carries_its_ctcss_tone() {
    let mut region_guard = TestRegionGuard::new();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let mut h = start_engine(
        Box::new(Rig { center: RPT_HZ, heard: Arc::clone(&heard) }),
        caps(),
        EngineConfig { remember_session: false, tx_ham_only: false, ..Default::default() },
    );
    region_guard.preserve_current();
    region_guard.own_engine_thread(h.thread.take());
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: RPT_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    send(Command::SetRepeater(minus_600(ToneMode::Ctcss)));
    let _ = state_where(&h.event_rx, |s| {
        s.rx[0].mode == Mode::Nfm && s.repeater.tone == ToneMode::Ctcss
    });

    // No microphone on this bench, so the over is silence with the tone under
    // it — which is exactly the case a repeater's decoder has to work in
    // anyway, between words.
    send(Command::SetPtt(true));
    // The tone needs about a second of signal to be resolved from its
    // neighbours in the table; take three to leave room for the key-up and for
    // the detector's own confirmation runs.
    let want = (3.0 * RATE) as usize;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        while h.event_rx.try_recv().is_ok() {}
        if heard.lock().unwrap().iq.len() >= want {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    send(Command::SetPtt(false));
    let iq = std::mem::take(&mut heard.lock().unwrap().iq);
    drop(h);
    drop(region_guard);

    assert!(iq.len() >= want, "only {} samples were transmitted", iq.len());

    // Discriminate: the phase advance between samples is the instantaneous
    // frequency, and the modulator's full scale is ±5 kHz — the units
    // `SubToneDetect` reads.
    let mut audio = Vec::with_capacity(iq.len());
    for w in iq.windows(2) {
        let d = w[1] * w[0].conj();
        let hz = d.arg() as f64 * RATE / std::f64::consts::TAU;
        audio.push((hz / 5_000.0) as f32);
    }
    let mut det = SubToneDetect::new(RATE);
    for block in audio.chunks(480) {
        det.process(block);
    }
    assert_eq!(
        det.detected(),
        Some(SubTone::Ctcss(885)),
        "88.5 Hz was asked for and the over did not carry it",
    );
}

/// The 1750 Hz burst, fired from receive: it keys the transmitter, sends a tone
/// of the length it was asked for, and lets go again.
///
/// The unkey is checked by the transmission simply stopping — no state flag,
/// because what matters is that the transmitter stopped and not that something
/// said it had.
#[test]
fn a_burst_fired_from_receive_keys_sends_and_unkeys() {
    let mut region_guard = TestRegionGuard::new();
    let heard = Arc::new(Mutex::new(Heard::default()));
    let mut h = start_engine(
        Box::new(Rig { center: SIMPLEX_HZ, heard: Arc::clone(&heard) }),
        caps(),
        EngineConfig { remember_session: false, tx_ham_only: false, ..Default::default() },
    );
    region_guard.preserve_current();
    region_guard.own_engine_thread(h.thread.take());
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: SIMPLEX_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    let _ = state_where(&h.event_rx, |s| s.rx[0].mode == Mode::Nfm);

    send(Command::ToneBurst);
    // Wait for it to start, then for it to stop: two consecutive looks that see
    // the same length mean nothing more is being transmitted.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last = 0usize;
    let mut settled = 0;
    while Instant::now() < deadline {
        while h.event_rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(80));
        let now = heard.lock().unwrap().iq.len();
        if now > 0 && now == last {
            settled += 1;
            if settled >= 2 {
                break;
            }
        } else {
            settled = 0;
        }
        last = now;
    }
    let iq = std::mem::take(&mut heard.lock().unwrap().iq);
    drop(h);
    drop(region_guard);

    // 500 ms at 48 kHz is 24 000 samples. A little short of that is expected
    // and is not the burst being cut off: the FM modulator's low-pass is a
    // 129-tap FIR and swallows its own delay line at the start of every over,
    // whatever is being modulated. Allow a block either way on top of that.
    let want = (f64::from(RepeaterState::default().burst_ms) / 1000.0 * RATE) as usize;
    let lost = 128 + 480;
    assert!(
        iq.len() + lost >= want && iq.len() <= want + 480,
        "the burst was {} samples against {want}: it never unkeyed, or it was cut short",
        iq.len(),
    );

    // …and it is 1750 Hz. Counted from the discriminated zero crossings over
    // the middle of the burst, past both ramps.
    let mid = &iq[iq.len() / 4..iq.len() * 3 / 4];
    let audio: Vec<f32> = mid
        .windows(2)
        .map(|w| {
            let d = w[1] * w[0].conj();
            (d.arg() as f64 * RATE / std::f64::consts::TAU / 5_000.0) as f32
        })
        .collect();
    let crossings = audio.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    let hz = crossings as f64 * RATE / (2.0 * audio.len() as f64);
    assert!(
        (hz - sdroxide_types::TONE_BURST_HZ).abs() < 20.0,
        "the burst came out at {hz:.0} Hz, not 1750",
    );
}
