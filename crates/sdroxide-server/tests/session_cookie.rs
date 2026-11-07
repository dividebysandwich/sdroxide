//! The sign-in cookie, end to end: three radios on one station, one password.
//!
//! This is the report the feature exists for (fork discussion #16, kevin2008-01:
//! *"it's asking for a session password for each radio station… Radio 1 = …
//! Radio 2 = … Radio 3 = … This is way too much!"*), so it is asserted the way
//! it was reported: a station of **three** radios, one operator, and no second
//! prompt. Everything else here is the other half — what must *not* stop
//! asking.
//!
//! One port per test, deliberately: the sign-in gate's turnstile and lockout
//! are server-wide state, and two tests sharing a port would be judging each
//! other's answers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use sdroxide_proto::{AudioCaps, ClientMsg, PROTO_VERSION, ServerMsg, decode, encode};
use sdroxide_radio::{AudioParams, EngineConfig, MicParams, SigGenSource, start_engine};
use sdroxide_server::{AccessFn, RadioParams, ServerParams, serve};
use sdroxide_types::{DeviceCaps, RemoteAccess};

const USER: &str = "f6kim";
const PASS: &str = "correct horse battery staple";
const REMEMBERED: u64 = 30 * 24 * 3600;
const SESSION: u64 = 12 * 3600;

/// A station of three radios — the shape in the report — behind one configured
/// password.
async fn spawn_station(port: u16) {
    let access: AccessFn = Box::new(|| RemoteAccess { username: USER.into(), password: PASS.into() });
    let mut radios = Vec::new();
    for id in 0..3 {
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
        radios.push(RadioParams {
            id,
            name: String::new(),
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum_out: handles.spectrum_out,
            wide_spectrum_out: handles.wide_spectrum_out,
            audio_rx: audio_consumer,
            mic_tx: mic_producer,
        });
    }

    tokio::spawn(serve(ServerParams {
        radios,
        bind: "127.0.0.1".into(),
        port,
        web_root: None,
        access: Some(access),
        probe: None,
        add_radio: None,
        remove_radio: None,
        rename_radio: None,
        radio_power: None,
    }));
    tokio::time::sleep(Duration::from_millis(600)).await;
}

fn hello() -> ClientMsg {
    ClientMsg::Hello {
        proto: PROTO_VERSION,
        audio: AudioCaps { opus_decode: false, opus_encode: false },
    }
}

/// POST the credentials and hand back the `Set-Cookie` value, if there is one.
fn sign_in(port: u16, username: &str, password: &str, remember: bool) -> Option<String> {
    let body = serde_json::json!({ "username": username, "password": password, "remember": remember });
    match ureq::post(&format!("http://127.0.0.1:{port}/signin"))
        .header("content-type", "application/json")
        .send(body.to_string())
    {
        Ok(resp) => resp
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .map(|c| c.split(';').next().unwrap_or("").to_string()),
        Err(_) => None,
    }
}

/// Open a radio's socket, optionally carrying a cookie, say Hello, and read the
/// one message that decides the test: `AuthRequired` means somebody is being
/// asked to sign in, anything else means they were not.
async fn first_answer(port: u16, path: &str, cookie: Option<&str>) -> ServerMsg {
    let url = format!("ws://127.0.0.1:{port}{path}");
    // Built one way whether there is a cookie or not: `connect_async` is
    // generic over the request type, so two arms would be two futures.
    let mut request = url.into_client_request().expect("a handshake request");
    if let Some(c) = cookie {
        request.headers_mut().insert("cookie", c.parse().expect("cookie header value"));
    }
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await.expect("connect");
    ws.send(Message::Binary(encode(&hello()).unwrap().into())).await.unwrap();
    loop {
        let m = tokio::time::timeout(Duration::from_secs(15), ws.next())
            .await
            .expect("timeout")
            .expect("stream ended")
            .expect("ws error");
        if let Message::Binary(bytes) = m {
            let msg = decode::<ServerMsg>(&bytes).expect("decode");
            // Streaming frames are not a verdict; keep reading.
            if matches!(msg, ServerMsg::Spectrum(_) | ServerMsg::RxAudio { .. }) {
                continue;
            }
            return msg;
        }
    }
}

/// The report, fixed: one sign-in, and the station's other radios let themselves
/// in with the cookie rather than asking the operator the same thing again.
#[tokio::test(flavor = "multi_thread")]
async fn one_sign_in_covers_every_radio_on_the_station() {
    let port = 39481;
    spawn_station(port).await;

    let cookie = sign_in(port, USER, PASS, true).expect("the right password gets a cookie");

    for path in ["/ws", "/ws/1", "/ws/2"] {
        match first_answer(port, path, Some(&cookie)).await {
            ServerMsg::HelloAck { .. } => {}
            other => panic!("{path} asked for a password it already had: {other:?}"),
        }
    }
}

/// The other half, and the one that matters more: no cookie, no radio. If this
/// ever passes without one, the feature has become a way in that the password
/// was not.
#[tokio::test(flavor = "multi_thread")]
async fn without_a_cookie_it_still_asks() {
    let port = 39482;
    spawn_station(port).await;

    for path in ["/ws", "/ws/1", "/ws/2"] {
        match first_answer(port, path, None).await {
            ServerMsg::AuthRequired => {}
            other => panic!("{path} let a cookie-less client in: {other:?}"),
        }
    }
}

/// A cookie is a signed statement about *these* credentials, so a wrong answer
/// must not buy one.
#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_buys_no_cookie() {
    let port = 39483;
    spawn_station(port).await;

    assert!(sign_in(port, USER, "not the password", true).is_none(), "issued a cookie anyway");
    assert!(matches!(first_answer(port, "/ws", None).await, ServerMsg::AuthRequired));
}

/// One answer, and the cookie has to carry the one that was given: a ticked box
/// is kept for as long as the box says, and an **unticked** one is both short
/// *and* carries no `Max-Age`, so the browser drops it when it closes. The two
/// halves are different mechanisms and a test that only checked the token would
/// pass on a cookie that sat in the store for twelve hours.
#[tokio::test(flavor = "multi_thread")]
async fn the_cookie_carries_the_answer_that_was_given() {
    let port = 39484;
    spawn_station(port).await;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let kept = sign_in(port, USER, PASS, true).expect("a cookie");
    let expiry: u64 = kept.rsplit(':').nth(1).expect("hex payload:expiry:mac").parse().expect("expiry");
    assert!(
        (REMEMBERED - 5..=REMEMBERED).contains(&(expiry - now)),
        "a ticked box asked to be remembered, got {}s",
        expiry - now
    );

    let session = sign_in(port, USER, PASS, false).expect("a cookie");
    let expiry: u64 = session.rsplit(':').nth(1).expect("hex payload:expiry:mac").parse().expect("expiry");
    assert!(
        (SESSION - 5..=SESSION).contains(&(expiry - now)),
        "an unticked box asked not to be, got {}s",
        expiry - now
    );
}

/// Signing out has to reach the cookie, and `HttpOnly` means the page cannot
/// do it — which is the whole reason this is an endpoint rather than a chip
/// that clears `document.cookie`.
#[tokio::test(flavor = "multi_thread")]
async fn signing_out_expires_the_cookie() {
    let port = 39485;
    spawn_station(port).await;

    sign_in(port, USER, PASS, true).expect("a cookie");
    let cleared = ureq::post(&format!("http://127.0.0.1:{port}/signout"))
        .send_empty()
        .expect("signout answers")
        .headers()
        .get("set-cookie")
        .expect("signout hands back a cookie")
        .to_str()
        .expect("a readable header")
        .to_string();
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert!(cleared.contains("HttpOnly"), "{cleared}");
}