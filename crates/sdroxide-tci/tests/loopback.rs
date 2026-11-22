//! In-process loopback: this crate's own TCI client driven against this
//! crate's own TCI server, with no radio and no third-party application.
//!
//! The two halves are independent implementations of the same wire format —
//! the client was written against real ExpertSDR3 hardware and the server
//! against the spec — so agreement between them is evidence, not a tautology.
//! Covers the connect burst, the IQ stream, the TX-audio path with its chrono
//! pacing, unsolicited status broadcast, and client accounting.
//!
//! Each test binds its own port in the 50051.. range (not the default 50001,
//! which a local ExpertSDR3 or another sdroxide would hold) so the suite can
//! run in parallel; a test whose port is unavailable skips rather than fails.

use std::time::{Duration, Instant};

use sdroxide_tci::server::{ServerRequest, TciServerController, TciStateSnapshot};
use sdroxide_tci::{TciHandle, TciUpdate};
use sdroxide_types::{Command, DeviceCaps, Mode, TciServerConfig};

const IQ_RATE: u32 = 96_000;

fn caps() -> DeviceCaps {
    DeviceCaps {
        tx_channels: 1,
        freq_ranges_rx: vec![(100_000.0, 60_000_000.0)],
        freq_ranges_tx: vec![(100_000.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

fn snapshot() -> TciStateSnapshot {
    TciStateSnapshot {
        vfo_a_hz: 14_074_000.0,
        vfo_b_hz: 14_074_000.0,
        center_hz: 14_100_000.0,
        if_span_hz: IQ_RATE as f64 / 2.0,
        mode: Mode::Digu,
        drive_pct: 40,
        tune_drive_pct: 10,
        iq_rate: IQ_RATE,
        vfo_lo_hz: 100_000.0,
        vfo_hi_hz: 60_000_000.0,
        can_tx: true,
        ..TciStateSnapshot::default()
    }
}

fn addr(port: u16) -> String {
    format!("127.0.0.1:{port}")
}

/// Start a server on `port`, or `None` when the port is taken (skip).
fn serve(port: u16) -> Option<TciServerController> {
    let cfg = TciServerConfig { port, ..TciServerConfig::default() };
    match TciServerController::start(&cfg, &caps(), snapshot()) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("skip: cannot bind 127.0.0.1:{port} ({e})");
            None
        }
    }
}

fn connect(port: u16) -> TciHandle {
    TciHandle::connect(&addr(port), IQ_RATE as f64).expect("connect")
}

/// Spin until `f` yields a value, up to `limit`. The hub polls at 5 ms and
/// clients at 20 ms, so nothing here happens instantly.
fn wait_for<T>(limit: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() > deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn await_clients(srv: &TciServerController, n: usize) {
    assert!(
        wait_for(Duration::from_secs(3), || (srv.clients() == n).then_some(())).is_some(),
        "expected {n} client(s), saw {}",
        srv.clients()
    );
}

/// A bare WebSocket client, for the parts `TciHandle` doesn't exercise: it
/// consumes IQ but not RX audio, and it never sends a command we refuse.
struct RawClient {
    ws: tungstenite::WebSocket<std::net::TcpStream>,
}

impl RawClient {
    fn connect(port: u16) -> RawClient {
        let stream = std::net::TcpStream::connect(addr(port)).expect("tcp connect");
        stream.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
        let url = format!("ws://{}/", addr(port));
        let (ws, _) = tungstenite::client(url.as_str(), stream).expect("ws handshake");
        RawClient { ws }
    }

    fn send(&mut self, line: &str) {
        self.ws.send(tungstenite::Message::Text(line.into())).expect("send");
    }

    /// Read for up to `limit`, collecting text lines and counting binary frames
    /// of `dtype` along with the floats they carried.
    fn collect(&mut self, limit: Duration, dtype: u32) -> (Vec<String>, usize, usize) {
        let (mut lines, mut frames, mut floats) = (Vec::new(), 0usize, 0usize);
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            match self.ws.read() {
                Ok(tungstenite::Message::Text(t)) => {
                    lines.extend(
                        t.as_str()
                            .split(';')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                    );
                }
                Ok(tungstenite::Message::Binary(b)) if b.len() >= 64 => {
                    let ty = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
                    if ty == dtype {
                        frames += 1;
                        floats += u32::from_le_bytes([b[20], b[21], b[22], b[23]]) as usize;
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
        }
        (lines, frames, floats)
    }
}

/// RX audio out: 48 kHz mono in from the engine, 48 kHz stereo out to the
/// client — and nothing at all until it subscribes.
#[test]
fn rx_audio_streams_only_to_subscribers() {
    let Some(srv) = serve(50059) else { return };
    let mut client = RawClient::connect(50059);
    await_clients(&srv, 1);

    // Before `audio_start`, pushing audio must produce no frames at all.
    let block = vec![0.25f32; 480];
    for _ in 0..10 {
        srv.on_rx_audio(&block);
    }
    let (_, frames, _) = client.collect(Duration::from_millis(200), 1);
    assert_eq!(frames, 0, "audio was streamed to a client that never subscribed");
    assert!(!srv.wants_audio());

    client.send("audio_start:0;");
    assert!(
        wait_for(Duration::from_secs(2), || srv.wants_audio().then_some(())).is_some(),
        "subscription never registered"
    );

    let mut got = 0usize;
    let deadline = Instant::now() + Duration::from_secs(3);
    while got < 480 && Instant::now() < deadline {
        srv.on_rx_audio(&block);
        let (_, _, floats) = client.collect(Duration::from_millis(50), 1);
        got += floats;
    }
    // Mono in, stereo out: each sample is duplicated across both channels.
    assert!(got >= 480, "RX audio never arrived ({got} floats)");

    client.send("audio_stop:0;");
    assert!(
        wait_for(Duration::from_secs(2), || (!srv.wants_audio()).then_some(())).is_some(),
        "unsubscribe never registered"
    );
}

/// Every accepted command must be echoed back, **even when it changes
/// nothing**. Clients treat the echo as the acknowledgement: WSJT-X sends
/// `audio_start:0;`, waits for it to come back, and reports "TCI Audio could
/// not be switched on" if it doesn't. Suppressing no-op echoes broke it.
#[test]
fn every_command_is_acknowledged() {
    let Some(srv) = serve(50062) else { return };
    let mut client = RawClient::connect(50062);
    await_clients(&srv, 1);
    client.collect(Duration::from_millis(200), 0); // drain the connect burst

    // Each of these asks for a value the radio already has, so nothing changes
    // and the state diff has nothing to say — the echo must come anyway.
    for cmd in ["audio_start:0;", "split_enable:false;", "iq_start:0;", "audio_stop:0;"] {
        client.send(cmd);
        let want = cmd.trim_end_matches(';');
        let found = wait_for(Duration::from_secs(2), || {
            let (lines, _, _) = client.collect(Duration::from_millis(50), 0);
            lines.iter().any(|l| l == want).then_some(())
        });
        assert!(found.is_some(), "no acknowledgement for {cmd}");
    }
}

/// Commands we understand but refuse must be answered with the truth, or the
/// client is left believing they took effect.
#[test]
fn refused_commands_are_corrected() {
    let Some(srv) = serve(50060) else { return };
    let mut client = RawClient::connect(50060);
    await_clients(&srv, 1);
    client.collect(Duration::from_millis(200), 0); // drain the connect burst

    // Turning receiver 0 off would silence the operator's own radio.
    client.send("rx_enable:0,false;");
    let (lines, _, _) = client.collect(Duration::from_millis(500), 0);
    assert!(
        lines.iter().any(|l| l == "rx_enable:0,true"),
        "expected the server to re-state rx_enable, got {lines:?}"
    );
}

/// The connect burst must be well-formed and end with `ready;` — exactly what
/// a third-party client blocks on before it will proceed.
#[test]
fn connect_burst_is_complete() {
    let Some(_srv) = serve(50051) else { return };

    let summary = sdroxide_tci::test_connection(&addr(50051), Duration::from_secs(3))
        .expect("server should complete the handshake");
    assert!(summary.contains("sdroxide"), "device name missing from {summary:?}");
    assert!(summary.contains("96000"), "IQ rate missing from {summary:?}");
}

/// Wideband IQ out: what the engine hands the server is what the client reads.
#[test]
fn iq_stream_round_trips() {
    let Some(srv) = serve(50052) else { return };
    let mut client = connect(50052);
    // `TciHandle::connect` sends `iq_start`; the subscription becomes visible
    // once the hub has processed it.
    assert!(
        wait_for(Duration::from_secs(3), || srv.wants_iq()).is_some(),
        "client never subscribed to IQ"
    );

    // A strictly increasing ramp, pushed as interleaved I,Q, so any misframing
    // shows up as an out-of-order value.
    let iq: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
    let mut got = vec![0.0f32; iq.len()];
    let mut have = 0usize;
    let deadline = Instant::now() + Duration::from_secs(5);
    while have < iq.len() && Instant::now() < deadline {
        srv.on_rx_iq(&iq, IQ_RATE);
        have += client.rx_read(&mut got[have..]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(have, iq.len(), "IQ never arrived");

    // The same block repeats, so the received run must match the ramp from
    // wherever in it we started reading.
    let start = iq.iter().position(|v| (*v - got[0]).abs() < 1e-6).expect("first value in ramp");
    for (i, v) in got.iter().enumerate() {
        assert!((v - iq[(start + i) % iq.len()]).abs() < 1e-6, "IQ mismatch at {i}");
    }
}

/// TX audio in: keying hands us the transmit request, and the client's audio
/// arrives at the engine-side ring downmixed to mono.
#[test]
fn tx_audio_reaches_the_engine() {
    let Some(mut srv) = serve(50053) else { return };
    let mut client = connect(50053);
    await_clients(&srv, 1);

    client.tx_begin(14_074_000.0);
    assert!(
        wait_for(Duration::from_secs(3), || {
            srv.poll().into_iter().find(|r| *r == ServerRequest::Key(true))
        })
        .is_some(),
        "server never reported the key request"
    );
    assert!(srv.tx_keyed(), "server should record the transmit owner");

    // A constant, so the value is unambiguous after the stereo downmix.
    let tone = vec![0.5f32; 480];
    let mut out = vec![0.0f32; 480];
    let mut have = 0usize;
    let deadline = Instant::now() + Duration::from_secs(5);
    while have < out.len() && Instant::now() < deadline {
        client.tx_write(&tone);
        // Chronos are how the engine paces a client; ask for a block's worth.
        srv.request_chrono(480);
        have += srv.read_tx_audio(&mut out[have..]);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(have, out.len(), "TX audio never arrived");
    assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-6), "TX audio corrupted: {:?}", &out[..8]);

    client.tx_end();
    assert!(
        wait_for(Duration::from_secs(3), || (!srv.tx_keyed()).then_some(())).is_some(),
        "unkey never took effect"
    );
}

/// State the operator changes locally must reach every client unsolicited.
#[test]
fn state_changes_are_broadcast() {
    let Some(srv) = serve(50054) else { return };
    let client = connect(50054);
    await_clients(&srv, 1);
    client.poll_updates(); // discard whatever the connect burst produced

    let mut moved = snapshot();
    moved.vfo_a_hz = 14_080_000.0;
    srv.broadcast_state(moved);

    let freq = wait_for(Duration::from_secs(3), || {
        client.poll_updates().into_iter().find_map(|u| match u {
            TciUpdate::Freq(hz) => Some(hz),
            _ => None,
        })
    });
    assert_eq!(freq, Some(14_080_000.0));
}

/// A command from a client becomes a `Command` for the engine.
#[test]
fn client_commands_reach_the_engine() {
    let Some(srv) = serve(50055) else { return };
    let client = connect(50055);
    await_clients(&srv, 1);

    client.set_center(14_200_000.0);
    let cmd = wait_for(Duration::from_secs(3), || {
        srv.poll().into_iter().find(|r| matches!(r, ServerRequest::Cmd(Command::SetCenter(_))))
    });
    assert_eq!(cmd, Some(ServerRequest::Cmd(Command::SetCenter(14_200_000.0))));

    client.set_mode(Mode::Cw);
    let cmd = wait_for(Duration::from_secs(3), || {
        srv.poll().into_iter().find(|r| matches!(r, ServerRequest::Cmd(Command::SetMode { .. })))
    });
    assert!(
        matches!(cmd, Some(ServerRequest::Cmd(Command::SetMode { mode: Mode::Cw, .. }))),
        "expected a mode change, got {cmd:?}"
    );
}

/// Several clients at once, and the count follows them in and out.
#[test]
fn client_count_tracks_connections() {
    let Some(srv) = serve(50056) else { return };

    let a = connect(50056);
    let b = connect(50056);
    await_clients(&srv, 2);

    drop(b);
    await_clients(&srv, 1);
    drop(a);
    await_clients(&srv, 0);
}

/// Only one client may transmit; a second key request is refused rather than
/// mixed into the first client's over.
#[test]
fn transmit_key_is_exclusive() {
    let Some(srv) = serve(50057) else { return };
    let a = connect(50057);
    let mut b = connect(50057);
    await_clients(&srv, 2);

    a.tx_begin(14_074_000.0);
    assert!(
        wait_for(Duration::from_secs(3), || {
            srv.poll().into_iter().find(|r| *r == ServerRequest::Key(true))
        })
        .is_some(),
        "first client never got the key"
    );

    b.tx_begin(14_074_000.0);
    b.tx_write(&[0.9f32; 480]);
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !srv.poll().contains(&ServerRequest::Key(true)),
        "second client should not have been granted the key"
    );
}

/// Dropping the controller must free the port, or a rebind after a settings
/// change would fail.
#[test]
fn restart_rebinds_cleanly() {
    let Some(srv) = serve(50058) else { return };
    let client = connect(50058);
    await_clients(&srv, 1);
    drop(client);
    drop(srv);

    let cfg = TciServerConfig { port: 50058, ..TciServerConfig::default() };
    TciServerController::start(&cfg, &caps(), snapshot()).expect("port should be free again");
}

/// The latest spot book the server has handed the engine, waiting until one
/// matching `want` arrives.
fn spots_until(
    srv: &TciServerController,
    want: impl Fn(&[sdroxide_types::Spot]) -> bool,
) -> Option<Vec<sdroxide_types::Spot>> {
    wait_for(Duration::from_secs(3), || {
        srv.poll().into_iter().rev().find_map(|r| match r {
            ServerRequest::Spots(s) if want(&s) => Some(s),
            _ => None,
        })
    })
}

/// `SPOT:` reaches the engine as a TCI label, is not echoed, survives its
/// client's disconnect, and any client can delete or clear it — the spec has
/// one panorama, not one per client.
#[test]
fn spots_reach_the_engine_and_outlive_their_client() {
    let Some(srv) = serve(50063) else { return };
    let mut a = RawClient::connect(50063);
    let mut b = RawClient::connect(50063);
    await_clients(&srv, 2);
    a.collect(Duration::from_millis(200), 0);
    b.collect(Duration::from_millis(200), 0);

    // The spec's example, word for word, and a second spot in the same message.
    a.send("SPOT:RN6LHF,CW,7100000,16711680,ANY_TEXT;spot:DL1ABC,FT8,14074000,-16711936,new;");
    let spots = spots_until(&srv, |s| s.len() == 2).expect("both spots reach the engine");
    let rn = spots.iter().find(|s| s.call == "RN6LHF").unwrap();
    assert_eq!(rn.kind, sdroxide_types::SpotKind::Tci);
    assert_eq!(
        (rn.freq_hz, rn.color, rn.comment.as_str()),
        (7_100_000.0, Some([255, 0, 0]), "ANY_TEXT")
    );
    assert_eq!(spots.iter().find(|s| s.call == "DL1ABC").unwrap().color, Some([0, 255, 0]));
    let (lines, _, _) = b.collect(Duration::from_millis(200), 0);
    assert!(!lines.iter().any(|l| l.to_lowercase().starts_with("spot")), "echoed: {lines:?}");

    // A spot missing its colour is ignored, as the spec says.
    a.send("spot:XX1XX,CW,7020000;");
    std::thread::sleep(Duration::from_millis(100));
    assert!(srv.poll().iter().all(|r| !matches!(r, ServerRequest::Spots(_))));

    // The placing client leaves; its spots stay.
    drop(a);
    await_clients(&srv, 1);
    std::thread::sleep(Duration::from_millis(100));
    assert!(srv.poll().iter().all(|r| !matches!(r, ServerRequest::Spots(_))));

    // The other client deletes one by callsign, in any case, then clears.
    b.send("spot_delete:rn6lhf;");
    let left = spots_until(&srv, |s| s.len() == 1).expect("delete reaches the engine");
    assert_eq!(left[0].call, "DL1ABC");
    b.send("SPOT_CLEAR;");
    assert!(spots_until(&srv, |s| s.is_empty()).is_some(), "clear reaches the engine");
}
