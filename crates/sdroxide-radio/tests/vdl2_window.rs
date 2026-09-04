//! The VDL Mode 2 lane, from a mock front end to a message on the panel's log.
//!
//! Three things are pinned here, and none can be seen from inside the decoder
//! crate:
//!
//! * **The plumbing carries a decode.** A stand-in receiver over the datalink
//!   group transmits an aircraft's ACARS downlink on the Common Signalling
//!   Channel; selecting the mode has to end with that message in a
//!   `Vdl2Status`, with the flight and the text the frame was built from. Every
//!   piece between the source and the event — the tap in the audio pass, the
//!   window's downconverter, the per-channel downconverters inside the worker,
//!   the snapshot — is in that path.
//!
//! * **A receiver that reaches only part of the plan says which part.** The
//!   window slides to take in what it can and the rest is reported as out of
//!   reach, because "that channel is quiet" and "that channel was never being
//!   listened to" produce the same empty column.
//!
//! * **A receiver that cannot do it at all says so.** A stream too narrow for
//!   even one channel must produce a sentence rather than an empty log.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{AudioParams, Complex32, EngineConfig, IqSource, Result, rtrb, start_engine};
use sdroxide_types::{
    Command, DeviceCaps, Mode, RadioEvent, RxId, VDL2_CSC_HZ, VDL2_PLAN_CENTER_HZ, Vdl2AddrKind,
    Vdl2Frame, Vdl2Payload, Vdl2Status, Vfo,
};

/// The aircraft, the ground station, and what is said.
const AIRCRAFT: u32 = 0x44_0F_31;
const GROUND: u32 = 0x10_20_30;
const FLIGHT: &str = "AUA123";
const TEXT: &str = "REQUEST DESCENT FL240";

/// The frame the stand-in receiver transmits: an ACARS downlink over AVLC.
fn frame() -> Vec<u8> {
    let acars = sdroxide_types::Vdl2Acars {
        mode: '2',
        registration: "OE-LWA".to_string(),
        ack: '\x15',
        label: "H1".to_string(),
        block_id: '1',
        msn: "M01A".to_string(),
        flight: FLIGHT.to_string(),
        text: TEXT.to_string(),
        ..sdroxide_types::Vdl2Acars::default()
    };
    sdroxide_vdl2::avlc::build(
        sdroxide_vdl2::avlc::Address { addr: GROUND, kind: Vdl2AddrKind::GroundAdmin, cr: false },
        sdroxide_vdl2::avlc::Address { addr: AIRCRAFT, kind: Vdl2AddrKind::Aircraft, cr: false },
        sdroxide_vdl2::avlc::control_octet(Vdl2Frame::I { ns: 0, nr: 0, p: false }),
        &sdroxide_vdl2::acars::build(&acars, true),
    )
}

/// A front end over the datalink group with one aircraft talking on the Common
/// Signalling Channel.
///
/// The burst is modulated once and handed out a block at a time, so the source
/// costs nothing per block and the test is not racing its own generator. It is
/// given a carrier offset and a clock error, because a transmitter that is
/// exactly on frequency is the one case that proves nothing.
struct Band {
    center: Arc<Mutex<f64>>,
    rate: f64,
    samples: Vec<Complex32>,
    pos: usize,
}

impl Band {
    fn new(rate: f64, center: Arc<Mutex<f64>>) -> Band {
        let p = sdroxide_vdl2::tx::TxParams {
            sample_rate: rate,
            freq_offset_hz: VDL2_CSC_HZ - VDL2_PLAN_CENTER_HZ + 900.0,
            clock_ppm: 6.0,
            amplitude: 0.5,
            ..sdroxide_vdl2::tx::TxParams::default()
        };
        // A gap either side, so the gate has a floor to learn and an edge to
        // close on — a burst welded to the loop's seam would never end.
        let mut samples = vec![Complex32::default(); (rate * 0.05) as usize];
        sdroxide_vdl2::tx::modulate_at(&frame(), &p, samples.len() as f64 + 0.37, &mut samples);
        samples.resize(samples.len() + (rate * 0.05) as usize, Complex32::default());
        let mut noise = sdroxide_vdl2::tx::Noise::new(0x5644_4c32);
        noise.add(&mut samples, 0.004);
        Band { center, rate, samples, pos: 0 }
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
        for z in buf[..n].iter_mut() {
            *z = self.samples[self.pos];
            self.pos = (self.pos + 1) % self.samples.len();
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "one aeroplane on the signalling channel".into()
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
    mut f: impl FnMut(&Vdl2Status) -> bool,
) -> Vdl2Status {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last: Option<Vdl2Status> = None;
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Vdl2Status(s) = ev {
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
            "bursts {} syncs {} headers {} (bad {}) rs_fail {} fcs_bad {} frames {} \
             unavailable {:?} degraded {:?}",
            s.bursts,
            s.syncs,
            s.headers,
            s.header_bad,
            s.rs_fail,
            s.fcs_bad,
            s.frames,
            s.unavailable,
            s.degraded
        )
    });
    panic!("the VDL2 decoder never reported {what}; last status: {brief:?}");
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

fn engine(rate: f64) -> sdroxide_radio::EngineHandles {
    // The lanes below the speaker — the skimmers, the ISM window, this one —
    // are fed from the same pass as the main receiver, so an engine with
    // nowhere to play audio never reaches them. Hence a ring nothing reads.
    let (producer, _consumer) = rtrb::RingBuffer::<f32>::new(48_000);
    let center = Arc::new(Mutex::new(VDL2_PLAN_CENTER_HZ));
    start_engine(
        Box::new(Band::new(rate, center)),
        caps(rate),
        EngineConfig {
            remember_session: false,
            audio: Some(AudioParams { producer, out_rate: 48_000.0 }),
            ..Default::default()
        },
    )
}

#[test]
fn selecting_the_mode_puts_the_message_in_the_log() {
    isolate_config("vdl2-window");
    let mut h = engine(2_400_000.0);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: VDL2_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Vdl2 });

    let st = status(&h, "a decoded message", |s| !s.messages.is_empty());
    assert!(st.unavailable.is_none(), "the lane should be running: {:?}", st.unavailable);
    assert!(st.degraded.is_none(), "a 2.4 Msps front end holds the whole plan: {:?}", st.degraded);
    assert_eq!(
        st.channels.len(),
        sdroxide_types::VDL2_CHANNELS_HZ.len(),
        "every channel of the plan is reported"
    );
    assert!(st.channels.iter().all(|c| c.live), "all of them should be open: {:?}", st.channels);

    let m = st.messages.last().expect("a message");
    assert_eq!(m.src, AIRCRAFT);
    assert_eq!(m.dst, GROUND);
    assert_eq!(m.freq_hz, VDL2_CSC_HZ, "it was transmitted on the signalling channel");
    match &m.payload {
        Vdl2Payload::Acars(a) => {
            assert_eq!(a.flight, FLIGHT);
            assert_eq!(a.text, TEXT);
        }
        other => panic!("expected ACARS, got {other:?}"),
    }
    // The transmitter was 900 Hz off and the decoder is supposed to notice: the
    // whole decision margin is 656 Hz, so an unmeasured offset would be fatal
    // rather than cosmetic.
    assert!(
        (m.freq_err_hz - 900.0).abs() < 250.0,
        "the carrier offset should have been measured, not {:.0} Hz",
        m.freq_err_hz
    );
    assert!(st.stations.iter().any(|s| s.addr == AIRCRAFT && s.flight == FLIGHT));

    // The window is a decimation of the stream, not the whole of it: this lane
    // needs a third of a megahertz and the rest is somebody else's.
    assert!(
        st.window_rate_hz > 440_000.0 && st.window_rate_hz < 2_400_000.0,
        "the window should be about half a megahertz, not {:.0} Hz",
        st.window_rate_hz
    );

    // ...and leaving the mode stops it, because a receiver parked on the
    // datalink group is not listening to anything else. Silence is what
    // standing down looks like from out here.
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    std::thread::sleep(Duration::from_millis(400));
    while h.event_rx.try_recv().is_ok() {}
    let quiet_until = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < quiet_until {
        while let Ok(ev) = h.event_rx.try_recv() {
            assert!(
                !matches!(ev, RadioEvent::Vdl2Status(_)),
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
fn a_narrow_window_says_which_channels_it_cannot_reach() {
    isolate_config("vdl2-narrow");
    // Wide enough for the middle of the plan and not for its edges. The lane
    // runs; what it cannot do is the thing that has to be said out loud.
    let mut h = engine(300_000.0);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: VDL2_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Vdl2 });

    let st = status(&h, "the channels it cannot reach", |s| s.degraded.is_some());
    assert!(st.unavailable.is_none(), "it should still run: {:?}", st.unavailable);
    let why = st.degraded.unwrap();
    let live = st.channels.iter().filter(|c| c.live).count();
    assert!(
        live > 0 && live < sdroxide_types::VDL2_CHANNELS_HZ.len(),
        "some but not all should be live, got {live}"
    );
    // The sentence has to say which channels are left, in frequencies: a count
    // on its own does not tell an operator whether the one they came for is
    // among them.
    let lowest = st.channels.iter().find(|c| c.live).expect("one live channel");
    let highest = st.channels.iter().rev().find(|c| c.live).expect("one live channel");
    for c in [lowest, highest] {
        let name = format!("{:.3}", c.freq_hz / 1e6);
        assert!(why.contains(&name), "the sentence should name {name}: {why}");
    }
    let dark = st.channels.iter().find(|c| !c.live).expect("one out of reach");
    assert_eq!(dark.reason.as_deref(), Some("outside the receiver's window"));

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn a_receiver_too_narrow_for_any_channel_says_so_rather_than_going_quiet() {
    isolate_config("vdl2-tiny");
    // Narrower than one 25 kHz channel with its shoulders: there is nothing to
    // decode and no amount of processing downstream can make one.
    let mut h = engine(24_000.0);
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    send(Command::SetVfo { vfo: Vfo::A, hz: VDL2_PLAN_CENTER_HZ });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Vdl2 });

    let st = status(&h, "the reason it cannot run", |s| s.unavailable.is_some());
    let why = st.unavailable.unwrap();
    assert!(why.contains("kHz"), "the sentence should name the rate: {why}");
    assert!(st.messages.is_empty());

    drop(h);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
