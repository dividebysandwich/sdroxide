//! Adding, switching and closing a station's radios from a client, over real
//! sockets.
//!
//! This is what lets a station with no screen gain a radio at all: before it,
//! `radios.json` on the server was the only way, and editing it meant
//! restarting the server and dropping everyone on the air to add a dongle. The
//! switch is the same story one step smaller — a radio put down for the season
//! rather than closed, and until it went on the wire the only way to put one
//! down was to stop the server.
//!
//! Its own test binary because it redirects the config directory through the
//! environment, which is process-global — the same reason `multi_radio.rs` and
//! `radio_config.rs` have one.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{EngineConfig, SigGenSource, start_engine};
use sdroxide_server::{RadioParams, ServerParams, serve};
use sdroxide_types::DeviceCaps;

const PORT: u16 = 39477;

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

/// The next roster announcement, past whatever else is on the wire.
async fn next_radios(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> (u32, Vec<sdroxide_proto::RadioInfo>, bool) {
    for _ in 0..80 {
        if let ServerMsg::Radios { me, radios, editable } = recv_msg(ws).await {
            return (me, radios, editable);
        }
    }
    panic!("the station never announced its radios");
}

/// Roster announcements until one satisfies `done`, which is how a client
/// follows a station: the roster is re-announced whenever it changes, and a
/// radio that has only just been added is announced before its engine has said
/// what it is. Bounded, so a station that never gets there fails the test
/// rather than hanging it.
async fn radios_until(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
    done: impl Fn(&[sdroxide_proto::RadioInfo]) -> bool,
) -> (u32, Vec<sdroxide_proto::RadioInfo>) {
    for _ in 0..10 {
        let (me, radios, _) = next_radios(ws).await;
        if done(&radios) {
            return (me, radios);
        }
    }
    panic!("the station's roster never settled");
}

/// The next operator notice, which is how a refused roster edit comes back.
async fn next_notice(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> String {
    for _ in 0..80 {
        if let ServerMsg::Notice(Some(n)) = recv_msg(ws).await {
            return n;
        }
    }
    panic!("the station never said why");
}

fn hello() -> ClientMsg {
    ClientMsg::Hello {
        proto: PROTO_VERSION,
        audio: AudioCaps { opus_decode: false, opus_encode: false },
    }
}

/// One radio's engine on a signal generator, as the station's own host would
/// hand it over — with the engine's thread handed back beside it, which is how
/// this test watches the engine stop.
fn radio_and_thread(
    id: u32,
    name: &str,
    label: &str,
    rate: f64,
) -> (RadioParams, Option<std::thread::JoinHandle<()>>) {
    let mut handles = start_engine(
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
    let thread = handles.thread.take();
    let params = RadioParams {
        id,
        name: name.into(),
        cmd_tx: handles.cmd_tx,
        event_rx: handles.event_rx,
        spectrum_out: handles.spectrum_out,
        wide_spectrum_out: handles.wide_spectrum_out,
        audio_rx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000).1,
        mic_tx: sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000).0,
    };
    (params, thread)
}

fn radio(id: u32, name: &str, label: &str, rate: f64) -> RadioParams {
    radio_and_thread(id, name, label, rate).0
}

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
async fn a_client_adds_renames_switches_and_closes_the_stations_radios() {
    let root = std::env::temp_dir().join(format!("sdroxide-roster-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch config dir");
    // SAFETY: set before anything reads it, and this binary holds one test.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // The added radio's engine thread, kept so the test can watch it stop when
    // the radio is closed.
    let engine: std::sync::Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let engine_slot = engine.clone();

    tokio::spawn(serve(ServerParams {
        radios: vec![radio(0, "", "First radio", 1_536_000.0)],
        bind: "127.0.0.1".into(),
        port: PORT,
        web_root: None,
        access: None,
        probe: None,
        // What the binary does, minus the backends: the roster entry and its
        // scope on disk through the same call, then an engine on it.
        add_radio: Some(Box::new(move |name: &str| {
            let slot = sdroxide_config::create_radio(name).map_err(|e| e.to_string())?;
            let (params, thread) =
                radio_and_thread(slot.id, &slot.name, "Added radio", 2_048_000.0);
            *engine_slot.lock().unwrap() = thread;
            Ok(params)
        })),
        remove_radio: Some(Box::new(|id| {
            sdroxide_config::remove_radio(id).map_err(|e| e.to_string())
        })),
        rename_radio: Some(Box::new(|id, name| {
            sdroxide_config::rename_radio(id, name).map_err(|e| e.to_string())
        })),
        // The roster file in both directions, as the binary wires it: the same
        // file the interface factory reads, so a switch thrown from a client
        // and an engine rebuilding on it are looking at one answer.
        radio_power: Some(Box::new(|id, set| {
            if let Some(on) = set {
                sdroxide_config::set_radio_enabled(id, on).map_err(|e| e.to_string())?;
            }
            Ok(sdroxide_config::load_radios().is_enabled(id))
        })),
        remote_clock_set: None,
    }));
    tokio::time::sleep(Duration::from_millis(400)).await;

    let (mut first, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws"))
        .await
        .expect("connect /ws");
    send(&mut first, &hello()).await;
    let (me, radios, editable) = next_radios(&mut first).await;
    assert_eq!(me, 0);
    assert_eq!(radios.len(), 1, "the station started with one radio");
    assert!(editable, "a station wired for roster edits did not say so");
    assert_eq!(radios[0].enabled, Some(true), "a station holding the switch did not say so");

    // --- add ----------------------------------------------------------
    send(&mut first, &ClientMsg::AddRadio { name: String::new() }).await;
    // Given no name of its own, the new radio is called after its interface —
    // which it says a moment after its engine starts, so the roster is
    // announced once when it appears and again when it can be named. Either
    // may be the first to arrive.
    let (me, radios) =
        radios_until(&mut first, |r| r.len() == 2 && r[1].name == "Added radio").await;
    assert_eq!(me, 0, "adding a radio moved this session onto another one");
    let added = radios[1].id;
    assert_ne!(added, 0, "the new radio took the first one's id");

    // It is a radio in its own right, at its own address, at the same time as
    // the first — which is the whole point of adding it.
    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws/{added}"))
            .await
            .expect("connect the added radio");
    send(&mut second, &hello()).await;
    match recv_msg(&mut second).await {
        ServerMsg::HelloAck { caps, state, .. } => {
            assert_eq!(caps.label, "Added radio");
            assert_eq!(state.sample_rate, 2_048_000.0);
        }
        other => panic!("expected HelloAck from the added radio, got {other:?}"),
    }
    let (me, _, _) = next_radios(&mut second).await;
    assert_eq!(me, added);

    // ...and it is in the listing an operator reads from outside the app.
    let (status, body) = http_get("/radios").await;
    assert!(status.contains("200"), "GET /radios: {status}");
    assert!(body.contains(&format!("\"path\":\"/ws/{added}\"")), "{body}");

    // --- rename -------------------------------------------------------
    send(&mut first, &ClientMsg::RenameRadio { id: added, name: "The Pluto".into() }).await;
    let (_, radios) = radios_until(&mut first, |r| r.len() == 2 && r[1].name == "The Pluto").await;
    // ...and said to be the operator's own, so a client shows that rather than
    // deriving one from the interface as it does for an unnamed radio.
    assert!(radios[1].named, "an operator's name was announced as a derived one");
    assert!(!radios[0].named, "an unnamed radio was announced as if somebody had named it");
    // The name is recorded where it lives — on the station, in its roster file
    // — rather than only on the screen of whoever typed it.
    assert_eq!(
        sdroxide_config::load_radios()
            .radios
            .iter()
            .find(|r| r.id == added)
            .map(|r| r.name.clone()),
        Some("The Pluto".to_string())
    );

    // --- switch off, and back on --------------------------------------
    //
    // A radio put down rather than closed: its engine, its scope and its
    // address all stay, and only its interface is let go. This is the whole of
    // what a browser client — the only screen a headless station has — needs to
    // free a dongle without stopping the server.
    send(&mut first, &ClientMsg::SetRadioEnabled { id: added, on: false }).await;
    let (_, radios) =
        radios_until(&mut first, |r| r.iter().any(|x| x.id == added && x.enabled == Some(false)))
            .await;
    assert_eq!(radios.len(), 2, "switching a radio off took it out of the roster");
    // Recorded where the interface factory will read it, not just announced.
    assert!(!sdroxide_config::load_radios().is_enabled(added));
    // Everyone on the station hears, not only whoever pressed the button —
    // including the client that is on the radio that was switched.
    let (me, radios) =
        radios_until(&mut second, |r| r.iter().any(|x| x.id == added && x.enabled == Some(false)))
            .await;
    assert_eq!(me, added);
    assert_eq!(radios.len(), 2);
    // Its socket is still open, which is what lets that client switch it back
    // on — from the tab that is looking at it.
    send(&mut second, &ClientMsg::SetRadioEnabled { id: added, on: true }).await;
    radios_until(&mut second, |r| r.iter().any(|x| x.id == added && x.enabled == Some(true))).await;
    assert!(sdroxide_config::load_radios().is_enabled(added));

    // --- the first radio is not closeable -----------------------------
    send(&mut first, &ClientMsg::RemoveRadio { id: 0 }).await;
    let why = next_notice(&mut first).await;
    assert!(why.contains("first radio"), "unexpected refusal: {why}");
    let (status, body) = http_get("/radios").await;
    assert!(status.contains("200"));
    assert!(body.contains("\"id\":0"), "the first radio went anyway: {body}");

    // --- close --------------------------------------------------------
    send(&mut first, &ClientMsg::RemoveRadio { id: added }).await;
    let (_, radios) = radios_until(&mut first, |r| r.len() == 1).await;
    assert_eq!(radios[0].id, 0, "the wrong radio was closed");

    // The client that was *on* the closed radio is told, on the way out: the
    // roster it gets no longer lists the radio it is on, which is how its tab
    // knows to close rather than to redial an address that is now a 404.
    let (me, radios) = radios_until(&mut second, |r| !r.iter().any(|x| x.id == added)).await;
    assert_eq!(me, added, "the closed radio's client was told about somebody else");
    assert_eq!(radios.len(), 1, "the roster it went out with was wrong");

    // ...and its engine stops, which is what hands the device back. Nothing on
    // the wire says so, so this waits for the engine's own event channel to
    // shut: a radio closed from a client that left its dongle claimed would
    // leave the operator unable to give it to another radio without restarting
    // the server.
    let stopped = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if engine.lock().unwrap().as_ref().is_some_and(|t| t.is_finished()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(stopped.is_ok(), "the closed radio's engine is still running");

    // Gone from the roster file, and from the address space.
    assert!(!sdroxide_config::load_radios().radios.iter().any(|r| r.id == added));
    match tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{PORT}/ws/{added}")).await {
        Err(tokio_tungstenite::tungstenite::Error::Http(r)) => assert_eq!(r.status(), 404),
        Err(e) => panic!("expected a 404 for the closed radio, got {e}"),
        Ok(_) => panic!("a socket opened on a radio that has been closed"),
    }
}
