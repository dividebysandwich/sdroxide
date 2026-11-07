//! The AIS lane, from a mock front end to a vessel on the panel's chart.
//!
//! Four things are pinned here, and none can be seen from inside the decoder
//! crate:
//!
//! * **The plumbing carries a decode.** A stand-in receiver over the two ship
//!   channels transmits a position report on one and a static report on the
//!   other; selecting the mode has to end with one row in an `AisStatus`
//!   carrying both — the position from one channel and the name from the other.
//!   Every piece between the source and the event is in that path: the tap in
//!   the audio pass, the window's down-converter, the per-channel
//!   down-converters inside the worker, the tracker, the snapshot.
//!
//! * **Both channels are listened to at once.** A ship alternates between them
//!   slot by slot, so a lane that decoded one would halve every vessel's
//!   reporting rate — and look exactly like a quiet sea while doing it.
//!
//! * **A receiver that reaches only one channel says which.** The window slides
//!   to take in what it can and the rest is reported as out of reach, because
//!   "that channel is quiet" and "that channel was never being listened to"
//!   produce the same empty column.
//!
//! * **A receiver that cannot do it at all says so.** A stream too narrow for
//!   even one channel must produce a sentence rather than an empty table.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{
    AIS_CHANNEL_A_HZ, AIS_CHANNEL_B_HZ, AIS_PLAN_CENTER_HZ, AisStatus, Command, DeviceCaps, Mode,
    RadioEvent, RxId, Vfo,
};

const MMSI: u32 = 244_660_000;
const NAME: &str = "NORDIC LIGHT";
const DESTINATION: &str = "ROTTERDAM";
const LAT: f64 = 52.3791;
const LON: f64 = 4.8973;

/// How far off frequency the stand-in transmitter is.
///
/// Deliberately not zero: a transmitter exactly on frequency is the one case
/// that proves nothing, and the decoder is supposed to *measure* this rather
/// than merely survive it.
const CARRIER_ERROR_HZ: f64 = 1_500.0;

/// A Class A position report — where the ship is, and how fast.
fn position_report() -> Vec<bool> {
    sdroxide_ais::tx::Payload::new(168)
        .put(0, 6, 1)
        .put(8, 30, u64::from(MMSI))
        .put(38, 4, 0) // under way using engine
        .put(50, 10, 154) // 15.4 knots
        .put_signed(61, 28, (LON * 600_000.0) as i64)
        .put_signed(89, 27, (LAT * 600_000.0) as i64)
        .put(116, 12, 2_734) // 273.4 degrees over ground
        .put(128, 9, 271)
        .bits()
}

/// The static and voyage report — what the ship is called, and where it is
/// going. A different message, on the other channel, minutes apart in reality.
fn static_report() -> Vec<bool> {
    sdroxide_ais::tx::Payload::new(424)
        .put(0, 6, 5)
        .put(8, 30, u64::from(MMSI))
        .put(232, 8, 70) // cargo
        .put(240, 9, 180)
        .put(249, 9, 20)
        .put(258, 6, 14)
        .put(264, 6, 14)
        .put_text(112, 20, NAME)
        .put_text(302, 20, DESTINATION)
        .bits()
}

/// A front end over the two AIS channels with one ship talking on both.
///
/// The slots are modulated once and handed out a block at a time, so the source
/// costs nothing per block and the test is not racing its own generator.
struct Band {
    center: Arc<Mutex<f64>>,
    rate: f64,
    samples: Vec<Complex32>,
    pos: usize,
    /// Where the slots were modulated about; a retune away from it shifts them
    /// the way a real band moves under a moving LO.
    plan_center: f64,
    phase: f64,
    /// See [`IqSource::lo_offset_hz`]: a zero-IF front end parks its LO this
    /// far above the dial.
    lo_offset: f64,
}

impl Band {
    fn new(rate: f64, center: Arc<Mutex<f64>>, plan_center: f64) -> Band {
        let p = sdroxide_ais::tx::TxParams { sample_rate: rate, ..Default::default() };
        // Silence either side of each slot, so the gate has a floor to learn
        // and an edge to close on — a burst welded to the loop's seam would
        // never end.
        let gap = (rate * 0.02) as usize;
        let mut samples = vec![Complex32::default(); gap];
        for (bits, channel_hz) in
            [(position_report(), AIS_CHANNEL_B_HZ), (static_report(), AIS_CHANNEL_A_HZ)]
        {
            let mut burst = sdroxide_ais::tx::modulate_bits(&bits, &p);
            sdroxide_ais::tx::shift(&mut burst, channel_hz - plan_center + CARRIER_ERROR_HZ, rate);
            samples.extend_from_slice(&burst);
            samples.resize(samples.len() + gap, Complex32::default());
        }
        let mut noise = sdroxide_ais::tx::Noise::new(0x4149_5300);
        noise.add(&mut samples, 0.004);
        Band { center, rate, samples, pos: 0, plan_center, phase: 0.0, lo_offset: 0.0 }
    }
}

impl IqSource for Band {
    fn sample_rate(&self) -> f64 {
        self.rate
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Not real time: a trickle of blocks keeps the decoder and its status
        // clock talking without spending a core on megasamples of silence.
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(8192);
        let step = std::f64::consts::TAU * (self.plan_center - self.center_hz()) / self.rate;
        for z in buf[..n].iter_mut() {
            let (sin, cos) = self.phase.sin_cos();
            *z = self.samples[self.pos] * Complex32::new(cos as f32, sin as f32);
            self.phase = (self.phase + step) % std::f64::consts::TAU;
            self.pos = (self.pos + 1) % self.samples.len();
        }
        Ok(n)
    }
    fn lo_offset_hz(&self) -> f64 {
        self.lo_offset
    }
    fn describe(&self) -> String {
        "one ship reporting on both AIS channels".into()
    }
}

fn caps(rate: f64) -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        sample_rates: vec![rate],
        freq_ranges_rx: vec![(1_000_000.0, 2_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Wait for a status that satisfies `f`, or say what the last one was.
fn status(
    h: &sdroxide_radio::EngineHandles,
    what: &str,
    mut f: impl FnMut(&AisStatus) -> bool,
) -> AisStatus {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last: Option<AisStatus> = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::AisStatus(s) = ev {
                if f(&s) {
                    return *s;
                }
                last = Some(*s);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let brief = last.map(|s| {
        format!(
            "slots {} messages {} bad_fcs {} vessels {} unavailable {:?} degraded {:?} \
             channels {:?}",
            s.bursts,
            s.messages,
            s.bad_fcs,
            s.vessels.len(),
            s.unavailable,
            s.degraded,
            s.channels.iter().map(|c| (c.label.clone(), c.live, c.bursts)).collect::<Vec<_>>()
        )
    });
    panic!("the AIS decoder never reported {what}; last status: {brief:?}");
}

fn isolate_config(name: &str) {
    let root = std::env::temp_dir().join(format!("sdroxide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // An engine test that saves anything writes the operator's real config
    // directory unless this is set: the variable is process-global and unset
    // means the live one.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
}

fn engine(rate: f64, dial: f64) -> sdroxide_radio::EngineHandles {
    engine_with_lo_offset(rate, dial, 0.0)
}

fn engine_with_lo_offset(rate: f64, dial: f64, lo_offset: f64) -> sdroxide_radio::EngineHandles {
    // The lanes below the speaker — the skimmers, the ISM window, this one —
    // are fed from the same pass as the main receiver, so an engine with
    // nowhere to play audio never reaches them. Hence a ring nothing reads.
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let center = Arc::new(Mutex::new(dial));
    start_engine(
        Box::new(Band { lo_offset, ..Band::new(rate, center, dial) }),
        caps(rate),
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    )
}

#[test]
fn selecting_the_mode_puts_the_ship_on_the_chart() {
    isolate_config("ais-window");
    let mut h = engine(2_400_000.0, AIS_PLAN_CENTER_HZ);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: AIS_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Ais });

    // Both messages, which arrive on different channels: the position on B and
    // the name on A. Waiting for the *name* is waiting for both, because the
    // tracker only has one row to put them on.
    let st = status(&h, "a named ship", |s| s.vessels.iter().any(|v| !v.name.is_empty()));
    assert!(st.unavailable.is_none(), "the lane should be running: {:?}", st.unavailable);
    assert!(st.degraded.is_none(), "a 2.4 Msps front end holds both channels: {:?}", st.degraded);
    assert_eq!(st.channels.len(), 2, "both channels are always reported");
    assert!(st.channels.iter().all(|c| c.live), "both should be open: {:?}", st.channels);
    assert!(
        st.channels.iter().all(|c| c.messages > 0),
        "each channel carried one of the two messages: {:?}",
        st.channels
    );

    assert_eq!(st.vessels.len(), 1, "one ship, whatever it sent on however many channels");
    let v = &st.vessels[0];
    assert_eq!(v.mmsi, MMSI);
    assert_eq!(v.name, NAME, "the static report has to land on the position report's row");
    assert_eq!(v.destination, DESTINATION);
    assert_eq!(v.sog_kt, Some(15.4));
    assert_eq!(v.length_m(), Some(200));
    let (lat, lon) = (v.lat.expect("a position"), v.lon.expect("a position"));
    assert!((lat - LAT).abs() < 1e-4, "latitude {lat}");
    assert!((lon - LON).abs() < 1e-4, "longitude {lon}");
    // The sentence an operator can paste into any other AIS tool — the only
    // check on a decoder written from a standard rather than from a recording.
    assert!(v.nmea.starts_with("!AIVDM,"), "no sentence on the row: {:?}", v.nmea);

    // The transmitter was 1.5 kHz off and the decoder is supposed to notice —
    // it is what tells an operator to set a frequency correction rather than to
    // conclude the sea is empty.
    let off = st.offset_hz.expect("a decoded message measures the offset");
    assert!(
        (f64::from(off) - CARRIER_ERROR_HZ).abs() < 500.0,
        "the carrier offset should have been measured, not {off:.0} Hz"
    );

    // The window is a decimation of the stream, not the whole of it: this lane
    // needs 150 kHz and the rest is somebody else's.
    assert!(
        st.window_rate_hz > 100_000.0 && st.window_rate_hz < 200_000.0,
        "the window should be about 150 kHz, not {:.0} Hz",
        st.window_rate_hz
    );
    assert!(st.samples_per_bit >= sdroxide_types::AIS_GOOD_SPS as f32);

    // ...and leaving the mode stops it, because a receiver parked on 162 MHz is
    // not listening to anything else. Silence is what standing down looks like
    // from out here.
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    std::thread::sleep(Duration::from_millis(400));
    while h.event_rx.try_recv().is_ok() {}
    let quiet_until = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < quiet_until {
        while let Ok(ev) = h.event_rx.try_recv() {
            assert!(
                !matches!(ev, RadioEvent::AisStatus(_)),
                "the lane was still reporting after the mode changed"
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn a_narrow_window_says_which_channel_it_cannot_reach() {
    isolate_config("ais-narrow");
    // Wide enough for one channel and not for the pair, and parked on the one
    // it can reach. The lane runs; what it cannot do is the thing that has to
    // be said out loud, because a ship alternates and will be heard at half its
    // reporting rate.
    let mut h = engine(60_000.0, AIS_CHANNEL_A_HZ);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: AIS_CHANNEL_A_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Ais });

    let st = status(&h, "the channel it cannot reach", |s| s.degraded.is_some());
    assert!(st.unavailable.is_none(), "it should still run: {:?}", st.unavailable);
    let why = st.degraded.unwrap();
    assert!(why.contains("AIS A"), "the sentence should name the channel it has: {why}");
    assert!(why.contains("AIS B"), "...and the one it has not: {why}");
    let live = st.channels.iter().filter(|c| c.live).count();
    assert_eq!(live, 1, "exactly one should be live: {:?}", st.channels);
    let dark = st.channels.iter().find(|c| !c.live).expect("one out of reach");
    assert_eq!(dark.reason.as_deref(), Some("outside the receiver's window"));

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn a_receiver_too_narrow_for_either_channel_says_so_rather_than_going_quiet() {
    isolate_config("ais-tiny");
    // Narrower than one 25 kHz channel with its shoulders: there is nothing to
    // decode and no amount of processing downstream can make one.
    let mut h = engine(24_000.0, AIS_PLAN_CENTER_HZ);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: AIS_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Ais });

    let st = status(&h, "the reason it cannot run", |s| s.unavailable.is_some());
    let why = st.unavailable.unwrap();
    assert!(why.contains("kHz"), "the sentence should name the rate: {why}");
    assert!(st.vessels.is_empty());

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn a_decimated_zero_if_front_end_still_reaches_both_channels() {
    isolate_config("ais-lo-offset");
    // An RSPdx at 2 Msps parks its LO 500 kHz above the dial, which the engine
    // scales by the decimation: 62.5 kHz at DEC 8. Entering AIS left the LO
    // there, and the 125 kHz window, clamped inside the 250 kHz stream, only
    // reached AIS B (#658).
    let mut h = engine_with_lo_offset(2_000_000.0, AIS_PLAN_CENTER_HZ, 500_000.0);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetDecimation(8));
    send(Command::SetVfo { vfo: Vfo::A, hz: AIS_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Ais });

    let st = status(&h, "a named ship", |s| s.vessels.iter().any(|v| !v.name.is_empty()));
    assert!(st.degraded.is_none(), "both channels fit a 125 kHz window: {:?}", st.degraded);
    assert!(st.channels.iter().all(|c| c.live), "both should be open: {:?}", st.channels);

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
