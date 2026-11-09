//! Device questions over a real socket.
//!
//! This is the other half of what makes a headless station's radio changeable
//! from somewhere else: `radio_config.rs` covers the settings travelling, and
//! settings alone are no use if the lists they are chosen from cannot. What a
//! machine has on its USB bus, which serial ports it owns, whether an address
//! answers from where it stands — all of it is asked of the machine the radio
//! is attached to, and comes back over the session.
//!
//! The prober here answers from a table rather than touching hardware: what is
//! under test is the lane, not this test runner's bus.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{EngineConfig, SigGenSource, start_engine};
use sdroxide_server::{ProbeFn, RadioParams, ServerParams, serve};
use sdroxide_types::{
    DeviceCaps, DeviceProbe, ProbeAnswer, ProbeTest, PublicSdrDirectory, PublicSdrEntry,
    PublicSdrNetwork, RtlSdrDevice, TestKind,
};

const PORT: u16 = 39474;
const UNSUPPORTED_PORT: u16 = 39475;

async fn recv_msg(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> ServerMsg {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(15), ws.next())
            .await
            .expect("timeout waiting for server message")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Binary(bytes) = m {
            return decode::<ServerMsg>(&bytes).expect("decode");
        }
    }
}

async fn send(
    ws: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    m: &ClientMsg,
) {
    ws.send(Message::Binary(encode(m).unwrap().into())).await.unwrap();
}

fn hello() -> ClientMsg {
    ClientMsg::Hello {
        proto: PROTO_VERSION,
        audio: AudioCaps { opus_decode: false, opus_encode: false },
    }
}

/// The dongle this station is pretending to have on its bus.
fn far_end_dongle() -> RtlSdrDevice {
    RtlSdrDevice {
        serial: Some("00000042".into()),
        name: "Generic RTL2832U OEM".into(),
        vid: 0x0bda,
        pid: 0x2838,
    }
}

/// The receiver list this station is pretending to have fetched.
fn far_end_directory(refresh: bool) -> PublicSdrDirectory {
    PublicSdrDirectory {
        entries: vec![PublicSdrEntry {
            network: PublicSdrNetwork::KiwiSdr,
            name: "Wessex KiWi".into(),
            location: "South West England, UK".into(),
            antenna: "Switched".into(),
            device: "KiwiSDR 1 v1.902".into(),
            address: "wessex.example.org:8073".into(),
            lat: Some(50.74),
            lon: Some(-2.63),
            grid: "IO80qr".into(),
            min_hz: 50_000.0,
            max_hz: 30_000_000.0,
            users: 3,
            max_users: 8,
            api_channels: Some(1),
            max_iq_rate: 12_000.0,
            full_control: true,
            session_limit_min: 0,
            snr_db: Some(20),
        }],
        fetched_unix: 1_788_000_000,
        // Which way round this came is part of the answer: a list served from
        // disk is what makes the window open instantly, and the operator is
        // told how old it is.
        stale: !refresh,
        notes: Vec::new(),
    }
}

async fn spawn_server(port: u16, probe: Option<ProbeFn>) {
    let handles = start_engine(
        Box::new(SigGenSource::demo(1_536_000.0, 14_200_000.0)),
        DeviceCaps {
            driver: "siggen".into(),
            label: "Test signal generator".into(),
            rx_channels: 1,
            freq_ranges_rx: vec![(0.0, 6e9)],
            ..DeviceCaps::default()
        },
        EngineConfig::default(),
    );
    tokio::spawn(serve(ServerParams {
        radios: vec![RadioParams {
            id: 0,
            name: String::new(),
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000).1,
            mic_tx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000).0,
        }],
        bind: "127.0.0.1".into(),
        port,
        web_root: None,
        access: None,
        probe,
        add_radio: None,
        remove_radio: None,
        rename_radio: None,
        radio_power: None,
        remote_clock_set: None,
    }));
    tokio::time::sleep(Duration::from_millis(400)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_far_ends_devices_reach_a_remote_client() {
    // Held while the prober is inside its slow scan, so the assertion below is
    // about what the session did *during* one rather than after it.
    let scanning = Arc::new(AtomicBool::new(false));
    let flag = scanning.clone();
    let prober: ProbeFn = Box::new(move |req, radio| {
        // Every question here is about the machine, so the asking radio only
        // has to be the one this server has.
        assert_eq!(radio, 0, "the answer goes back to the radio that asked");
        match req {
            DeviceProbe::RtlSdr => ProbeAnswer::RtlSdr(vec![far_end_dongle()]),
            DeviceProbe::SerialPorts => ProbeAnswer::SerialPorts(vec!["/dev/ttyACM1".into()]),
            // The one probe that is not about this machine's own hardware. It
            // still has to come from here: a browser client has no HTTP client
            // and could not read either directory across origins if it had, so
            // asking the station is the only route the web UI has to it.
            DeviceProbe::PublicSdrs { refresh } => {
                ProbeAnswer::PublicSdrs(Box::new(far_end_directory(refresh)))
            }
            // The slow one: a connection test takes seconds, and this is where
            // a server that ran probes on the socket task would show it.
            DeviceProbe::Test(ProbeTest::Tci(addr)) => {
                flag.store(true, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(600));
                flag.store(false, Ordering::SeqCst);
                ProbeAnswer::Test(TestKind::Tci, Ok(format!("ExpertSDR3 at {addr}")))
            }
            other => panic!("unexpected probe {other:?}"),
        }
    });
    spawn_server(PORT, Some(prober)).await;

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect");
    send(&mut ws, &hello()).await;

    // The dongle list is the far end's, not this machine's — which is the whole
    // point: a client cannot choose a device out of a list it never gets.
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::RtlSdr)).await;
    match next_answer(&mut ws).await {
        ProbeAnswer::RtlSdr(devices) => assert_eq!(devices, vec![far_end_dongle()]),
        other => panic!("expected the dongle list, got {other:?}"),
    }

    // Answers come back in the order the questions were asked, which is what
    // lets them go without request ids: two Rescans in flight must not swap.
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::SerialPorts)).await;
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::RtlSdr)).await;
    assert!(matches!(next_answer(&mut ws).await, ProbeAnswer::SerialPorts(_)));
    assert!(matches!(next_answer(&mut ws).await, ProbeAnswer::RtlSdr(_)));

    // A slow probe must not stall the session: the Ping sent behind it is
    // answered while the scan is still running. Without a worker of its own —
    // running these on the socket task, or worse on the engine thread — the
    // Pong would queue up behind the scan and the radio would go deaf for its
    // duration.
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::Test(ProbeTest::Tci("shack:40001".into())))).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(scanning.load(Ordering::SeqCst), "the prober should still be scanning");
    send(&mut ws, &ClientMsg::Ping(7)).await;
    loop {
        match recv_msg(&mut ws).await {
            ServerMsg::Pong(7) => break,
            ServerMsg::ProbeAnswer(a) => panic!("the scan finished before the ping: {a:?}"),
            _ => continue,
        }
    }
    assert!(scanning.load(Ordering::SeqCst), "the ping was answered after the scan, not during");

    match next_answer(&mut ws).await {
        ProbeAnswer::Test(TestKind::Tci, Ok(s)) => assert!(s.contains("shack:40001")),
        other => panic!("expected the connection test's answer, got {other:?}"),
    }

    // The public-SDR directory, which is how a browser client browses at all.
    // Both forms, because they are different requests: the cached one is what
    // opening the window asks for, and the refresh is the ⟳ chip.
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::PublicSdrs { refresh: false })).await;
    match next_answer(&mut ws).await {
        ProbeAnswer::PublicSdrs(d) => {
            assert!(d.stale, "a cached answer must say so, or its age is invisible");
            assert_eq!(d.entries.len(), 1);
            assert_eq!(d.entries[0].address, "wessex.example.org:8073");
            assert_eq!(d.count(PublicSdrNetwork::KiwiSdr), 1);
            assert_eq!(d.count(PublicSdrNetwork::SpyServer), 0);
        }
        other => panic!("expected the receiver list, got {other:?}"),
    }
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::PublicSdrs { refresh: true })).await;
    match next_answer(&mut ws).await {
        ProbeAnswer::PublicSdrs(d) => assert!(!d.stale, "a refresh went to the network"),
        other => panic!("expected the receiver list, got {other:?}"),
    }
}

/// A server with no prober says so, rather than leaving the client waiting for
/// an answer that is never coming — that is what greys the Rescan and Discover
/// buttons out with a reason instead of leaving them looking broken.
#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_cannot_answer_says_so() {
    spawn_server(UNSUPPORTED_PORT, None).await;

    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{UNSUPPORTED_PORT}/ws"))
            .await
            .expect("connect");
    send(&mut ws, &hello()).await;
    send(&mut ws, &ClientMsg::Probe(DeviceProbe::Hpsdr)).await;
    assert!(matches!(next_answer(&mut ws).await, ProbeAnswer::Unsupported));
}

/// The next probe answer, ignoring the streams running underneath it.
async fn next_answer(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> ProbeAnswer {
    loop {
        if let ServerMsg::ProbeAnswer(a) = recv_msg(ws).await {
            return *a;
        }
    }
}
