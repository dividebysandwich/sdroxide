//! A station reached through a reverse proxy that does not strip its path
//! prefix (issue #241).
//!
//! Putting sdroxide behind `https://host/sdroxide/` is an ordinary arrangement
//! and not an exotic one: a Tailscale certificate covers a host and offers no
//! subdomains, so a subpath is the only place a second service can go. Caddy's
//! plain `reverse_proxy` forwards the prefix untouched, and every request then
//! arrives naming something the router has never heard of — the page, the
//! bundle beside it, the radio listing and the socket alike.
//!
//! So the assertions here are made against the *prefixed* addresses, with the
//! unprefixed ones checked alongside: a station reached directly must go on
//! working exactly as it did.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{AudioParams, EngineConfig, MicParams, SigGenSource, start_engine};
use sdroxide_server::{RadioParams, ServerParams, serve};
use sdroxide_types::DeviceCaps;

/// The HTTP test's station. One port per test: they run side by side, and two
/// servers cannot have the same one — the loser's bind fails and its client
/// finds nothing listening.
const PORT: u16 = 39478;
/// The socket test's.
const WS_PORT: u16 = 39479;

/// An engine on a signal generator, served on `port`.
async fn spawn_server(port: u16) {
    let (audio_producer, audio_consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(96_000);
    let (mic_producer, mic_consumer) = sdroxide_radio::rtrb::RingBuffer::<f32>::new(48_000);
    let handles = start_engine(
        Box::new(SigGenSource::demo(1_536_000.0, 14_200_000.0)),
        DeviceCaps {
            driver: "siggen".into(),
            label: "Test signal generator".into(),
            rx_channels: 1,
            freq_ranges_rx: vec![(0.0, 6e9)],
            ..DeviceCaps::default()
        },
        EngineConfig {
            audio: Some(AudioParams { producer: audio_producer, out_rate: 48_000.0 }),
            mic: Some(MicParams { consumer: mic_consumer, rate: 48_000.0 }),
            ..Default::default()
        },
    );
    tokio::spawn(serve(ServerParams {
        radios: vec![RadioParams {
            id: 0,
            name: String::new(),
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: audio_consumer,
            mic_tx: mic_producer,
        }],
        bind: "127.0.0.1".into(),
        port,
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
}

/// A bare HTTP GET, because the whole point is which *path* was asked for and
/// pulling in an HTTP client to say that would be more machinery than the
/// request has bytes.
async fn get(path: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", PORT)).await.expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    sock.write_all(req.as_bytes()).await.expect("write");
    let mut body = String::new();
    tokio::time::timeout(Duration::from_secs(5), sock.read_to_string(&mut body))
        .await
        .expect("timeout reading the answer")
        .expect("read");
    body
}

/// The radio listing, asked for through a prefix the proxy did not strip.
///
/// Chosen as the HTTP half of the test because it is the one route that answers
/// with a body this crate can check without the web client being built: the
/// static handler only exists under the `embed-web` feature, and it takes the
/// same path through the same rewrite.
#[tokio::test]
async fn a_path_prefix_the_proxy_left_on_still_reaches_the_station() {
    spawn_server(PORT).await;

    let direct = get("/radios").await;
    assert!(direct.starts_with("HTTP/1.1 200"), "the station's own address broke: {direct}");
    assert!(direct.contains("\"path\":\"/ws/0\""), "{direct}");

    for path in ["/sdroxide/radios", "/shack/sdr/radios"] {
        let through = get(path).await;
        assert!(through.starts_with("HTTP/1.1 200"), "{path} answered: {through}");
        assert!(through.contains("\"path\":\"/ws/0\""), "{path} answered: {through}");
    }

    // The strip takes a prefix off an address that names an endpoint; it does
    // not turn one that names nothing into one that does. Where the answer goes
    // instead depends on the build — the client's own page with the web client
    // embedded, the placeholder without it — but it is never another endpoint's.
    let missing = get("/sdroxide/nonsense").await;
    assert!(!missing.contains("\"path\":\"/ws/0\""), "a stray address reached /radios: {missing}");
}

/// The socket too, which is the half that matters: the page loading and then
/// failing to connect is the same broken client as the page not loading.
#[tokio::test]
async fn the_socket_opens_through_a_path_prefix() {
    spawn_server(WS_PORT).await;

    for path in ["ws", "sdroxide/ws", "sdroxide/ws/0", "shack/sdr/ws"] {
        let url = format!("ws://127.0.0.1:{WS_PORT}/{path}");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .unwrap_or_else(|e| panic!("{url} refused the upgrade: {e}"));
        ws.send(Message::Binary(
            encode(&ClientMsg::Hello {
                proto: PROTO_VERSION,
                audio: AudioCaps { opus_decode: false, opus_encode: false },
            })
            .expect("encode")
            .into(),
        ))
        .await
        .expect("send");
        let greeting = loop {
            let m = tokio::time::timeout(Duration::from_secs(15), ws.next())
                .await
                .expect("timeout")
                .expect("stream ended")
                .expect("ws error");
            if let Message::Binary(bytes) = m {
                break decode::<ServerMsg>(&bytes).expect("decode");
            }
        };
        assert!(
            matches!(greeting, ServerMsg::HelloAck { .. }),
            "{url} did not greet us: {greeting:?}"
        );
        let _ = ws.close(None).await;
        // One client per session, and the last one is not gone until the
        // server has noticed: give it the moment rather than racing it into a
        // Busy on the next address.
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// The client itself, through a prefix: the page, and the bundle the page asks
/// for. This is the half the issue was reported against — `https://host/sdroxide/`
/// answering "not found".
///
/// Only where the client is embedded, which is also the only build that has one
/// to serve.
#[cfg(feature = "embed-web")]
#[tokio::test]
async fn the_client_and_its_bundle_load_through_a_path_prefix() {
    const WEB_PORT: u16 = 39480;
    spawn_server(WEB_PORT).await;

    let get = |path: String| async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock =
            tokio::net::TcpStream::connect(("127.0.0.1", WEB_PORT)).await.expect("connect");
        let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        sock.write_all(req.as_bytes()).await.expect("write");
        let mut body = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), sock.read_to_end(&mut body))
            .await
            .expect("timeout")
            .expect("read");
        String::from_utf8_lossy(&body).into_owned()
    };

    // With the trailing slash and without it: a browser resolves the page's
    // relative links against whichever one the operator typed, and both have to
    // arrive at the client.
    for path in ["/sdroxide/", "/sdroxide", "/shack/sdr/"] {
        let page = get(path.to_string()).await;
        assert!(page.starts_with("HTTP/1.1 200"), "{path}: {}", &page[..page.len().min(200)]);
        assert!(page.contains("sdroxide_canvas"), "{path} did not answer with the client");
    }

    // The page's own bundle, asked for the way the browser will ask for it —
    // which is the whole reason the emitted URL is relative. Anything else and
    // the page loads and the client never starts.
    let page = get("/sdroxide/".to_string()).await;
    let bundle = page
        .split_once("module_or_path: '")
        .and_then(|(_, rest)| rest.split_once('\''))
        .map(|(url, _)| url.to_string())
        .expect("index.html should name the wasm bundle");
    assert!(
        bundle.starts_with("./"),
        "the bundle is addressed from the server root ({bundle}), so it cannot be found under a \
         prefix — see crates/sdroxide-web/Trunk.toml"
    );
    let asked_for = format!("/sdroxide/{}", bundle.trim_start_matches("./"));
    let answer = get(asked_for.clone()).await;
    assert!(
        answer.starts_with("HTTP/1.1 200"),
        "{asked_for}: {}",
        &answer[..answer.len().min(200)]
    );
}
