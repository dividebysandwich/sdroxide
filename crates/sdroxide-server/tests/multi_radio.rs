//! A station with two radios, over real sockets.
//!
//! This is what makes `--server` worth running on a station that has more than
//! one radio: both engines are up, each is reachable in its own right, and
//! neither client is holding the other's radio. `/ws` still means the first
//! radio, because that is what every client written before there was a roster
//! asks for.
//!
//! Its own test binary because it redirects the config directory through the
//! environment, which is process-global — the same reason `radio_config.rs`
//! has one.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{EngineConfig, SigGenSource, start_engine};
use sdroxide_server::{RadioParams, ServerParams, serve};
use sdroxide_types::DeviceCaps;

const PORT: u16 = 39476;

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

/// The roster announcement, which follows `HelloAck` — past whatever else the
/// connect-time replay is sending.
async fn next_radios(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> (u32, Vec<sdroxide_proto::RadioInfo>, bool) {
    for _ in 0..50 {
        if let ServerMsg::Radios { me, radios, editable } = recv_msg(ws).await {
            return (me, radios, editable);
        }
    }
    panic!("the station never announced its radios");
}

fn hello() -> ClientMsg {
    ClientMsg::Hello {
        proto: PROTO_VERSION,
        audio: AudioCaps { opus_decode: false, opus_encode: false },
    }
}

/// One radio's engine, on a signal generator that is distinguishable from the
/// other's by both its label and its sample rate.
fn radio(id: u32, name: &str, label: &str, rate: f64) -> RadioParams {
    let handles = start_engine(
        Box::new(SigGenSource::demo(rate, 14_200_000.0)),
        DeviceCaps {
            driver: "siggen".into(),
            label: label.into(),
            rx_channels: 1,
            freq_ranges_rx: vec![(0.0, 6e9)],
            ..DeviceCaps::default()
        },
        EngineConfig::default(),
    );
    RadioParams {
        id,
        name: name.into(),
        cmd_tx: handles.cmd_tx,
        event_rx: handles.event_rx,
        spectrum_out: handles.spectrum_out,
        wide_spectrum_out: handles.wide_spectrum_out,
        audio_rx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000).1,
        mic_tx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000).0,
    }
}

/// A bare HTTP GET, so the JSON listing is exercised the way a browser or a
/// script would reach it rather than through anything this crate exports.
async fn http_get(path: &str) -> (String, String) {
    let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", PORT)).await.expect("connect");
    sock.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").as_bytes(),
    )
    .await
    .expect("request");
    let mut raw = String::new();
    sock.read_to_string(&mut raw).await.expect("response");
    let (head, body) = raw.split_once("\r\n\r\n").expect("headers and body");
    (head.lines().next().unwrap_or_default().to_string(), body.to_string())
}

#[tokio::test(flavor = "multi_thread")]
async fn every_radio_in_the_roster_is_served_and_separately_addressable() {
    let root =
        std::env::temp_dir().join(format!("sdroxide-multiradio-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch config dir");
    // SAFETY: set before anything reads it, and this binary holds one test.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    tokio::spawn(serve(ServerParams {
        radios: vec![
            radio(0, "", "First radio", 1_536_000.0),
            // Named, because the roster leaves the name empty far more often
            // than not and the listing has to carry both cases.
            radio(1, "The Pluto", "Second radio", 2_048_000.0),
        ],
        bind: "127.0.0.1".into(),
        port: PORT,
        web_root: None,
        access: None,
        probe: None,
        add_radio: None,
        remove_radio: None,
        rename_radio: None,
        radio_power: None,
        remote_clock_set: None,
    }));
    tokio::time::sleep(Duration::from_millis(400)).await;

    // `/ws` is the first radio — unchanged, and the only address a client that
    // knows nothing of a roster ever asks for.
    let (mut first, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect /ws");
    send(&mut first, &hello()).await;
    let first_rate = match recv_msg(&mut first).await {
        ServerMsg::HelloAck { caps, state, .. } => {
            assert_eq!(caps.label, "First radio");
            state.sample_rate
        }
        other => panic!("expected HelloAck on /ws, got {other:?}"),
    };
    assert_eq!(first_rate, 1_536_000.0);

    // ...and the second radio is reachable *at the same time*: the one-client
    // rule is about one radio, not about the station. Before this, a station's
    // second radio was not merely unaddressable — it was never opened.
    let (mut second, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws/1"))
        .await
        .expect("connect /ws/1");
    send(&mut second, &hello()).await;
    match recv_msg(&mut second).await {
        ServerMsg::HelloAck { caps, state, .. } => {
            assert_eq!(caps.label, "Second radio", "/ws/1 answered with the wrong radio");
            assert_eq!(state.sample_rate, 2_048_000.0);
        }
        other => panic!("expected HelloAck on /ws/1, got {other:?}"),
    }

    // Each session is told which radio it is on and what else the station has
    // — the whole of how a client that holds several radios finds the others
    // without anybody typing an address in.
    let (me, radios, editable) = next_radios(&mut second).await;
    assert_eq!(me, 1, "the second session did not know which radio it was on");
    assert_eq!(radios.len(), 2, "the roster was not announced in full");
    assert_eq!(radios[0].id, 0);
    assert_eq!(radios[1].id, 1);
    // Named as the listing names them: the operator's name where there is one,
    // the interface's where there is not.
    assert_eq!(radios[0].name, "First radio");
    assert_eq!(radios[1].name, "The Pluto");
    // This station was handed no way to edit its roster, so it says so rather
    // than leaving the client to press a button nothing answers.
    assert!(!editable, "a station with no roster callbacks called itself editable");

    // The first client is still on its own radio, not displaced by the second.
    send(&mut first, &ClientMsg::Ping(7)).await;
    loop {
        match recv_msg(&mut first).await {
            ServerMsg::Pong(7) => break,
            ServerMsg::Busy | ServerMsg::Error(_) => panic!("the first session was displaced"),
            _ => continue,
        }
    }

    // The explicit form reaches radio 0 as well, so a client need not special-
    // case the first radio.
    let (mut zero, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws/0"))
        .await
        .expect("connect /ws/0");
    send(&mut zero, &hello()).await;
    match recv_msg(&mut zero).await {
        // Busy is the *right* answer: /ws/0 and /ws are the same radio, and it
        // already has a client.
        ServerMsg::Busy => {}
        other => panic!("expected Busy on /ws/0 while /ws is held, got {other:?}"),
    }

    // A radio the station does not have is refused rather than rounded to one
    // it does — a client that asked for the Pluto must never end up keying
    // something else. Refused as "no such thing", too, and not as some failure
    // of the upgrade: the client has to be able to tell the difference between
    // a station that has no radio 9 and one that is not answering.
    match tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws/9")).await {
        Err(tokio_tungstenite::tungstenite::Error::Http(r)) => {
            assert_eq!(r.status(), 404, "wrong refusal for a radio that does not exist");
        }
        Err(e) => panic!("expected a 404 for an unknown radio, got {e}"),
        Ok(_) => panic!("a socket opened on a radio that does not exist"),
    }

    // And the listing an operator (or a browser) reads to find all this out.
    let (status, body) = http_get("/radios").await;
    assert!(status.contains("200"), "GET /radios: {status}");
    assert!(body.contains("\"id\":0"), "{body}");
    assert!(body.contains("\"id\":1"), "{body}");
    assert!(body.contains("\"path\":\"/ws/1\""), "{body}");
    // Radio 0 was given no name, so it is listed under what its interface
    // calls itself; radio 1's own name wins over its interface's.
    assert!(body.contains("First radio"), "{body}");
    assert!(body.contains("The Pluto"), "{body}");
}
