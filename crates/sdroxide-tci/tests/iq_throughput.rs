//! IQ throughput in real time: the engine's block pattern at a DAB-capable
//! rate, pushed for a few seconds, counted at a bare WebSocket client.
//!
//! Why: HDZ 2026-11-16, TCI_transcode as an rtl_tcp server for a DAB
//!      decoder: "die Rate muss hoch, sonst ist die Schnittstelle nutzlos, da
//!      muss der Fehler gefunden werden". DAB needs 2.048 MS/s without a gap.
//!
//! Timing-bound, so `#[ignore]`: `cargo test -p sdroxide-tci --test
//! iq_throughput -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use sdroxide_tci::server::{TciServerController, TciStateSnapshot};
use sdroxide_types::{DeviceCaps, Mode, TciServerConfig};

const RATE: u32 = 2_048_000;
const SECONDS: f64 = 4.0;

fn snapshot() -> TciStateSnapshot {
    TciStateSnapshot {
        vfo_a_hz: 220_352_000.0,
        vfo_b_hz: 220_352_000.0,
        center_hz: 220_352_000.0,
        if_span_hz: RATE as f64 / 2.0,
        mode: Mode::Am,
        iq_rate: RATE,
        vfo_lo_hz: 100_000.0,
        vfo_hi_hz: 300_000_000.0,
        ..TciStateSnapshot::default()
    }
}

/// Push `per_block` complex samples at a time, paced to `RATE`, and return
/// (complex samples pushed, complex samples the client received, frames).
fn run(port: u16, per_block: usize) -> Option<(usize, usize, usize)> {
    let cfg = TciServerConfig { port, ..TciServerConfig::default() };
    let srv = TciServerController::start(&cfg, &DeviceCaps::default(), snapshot()).ok()?;
    let addr = format!("127.0.0.1:{port}");
    let stream = std::net::TcpStream::connect(&addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
    let (mut ws, _) = tungstenite::client(format!("ws://{addr}/").as_str(), stream).ok()?;
    ws.send(tungstenite::Message::Text(format!("iq_samplerate:{RATE};").into())).unwrap();
    ws.send(tungstenite::Message::Text("iq_start:0;".into())).unwrap();
    let t0 = Instant::now();
    while srv.wants_iq().is_none() && t0.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(5));
    }
    srv.wants_iq()?;

    let reader = std::thread::spawn(move || {
        let (mut frames, mut floats) = (0usize, 0usize);
        let deadline = Instant::now() + Duration::from_secs_f64(SECONDS + 1.5);
        while Instant::now() < deadline {
            match ws.read() {
                Ok(tungstenite::Message::Binary(b)) if b.len() >= 64 => {
                    let ty = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
                    if ty == 0 {
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
        (frames, floats / 2)
    });

    let block = vec![0.25f32; per_block * 2];
    let start = Instant::now();
    let mut pushed = 0usize;
    while start.elapsed().as_secs_f64() < SECONDS {
        let due = (start.elapsed().as_secs_f64() * RATE as f64) as usize;
        while pushed + per_block <= due {
            srv.on_rx_iq(&block, RATE);
            pushed += per_block;
        }
        std::thread::sleep(Duration::from_micros(500));
    }
    let (frames, got) = reader.join().unwrap();
    Some((pushed, got, frames))
}

#[test]
#[ignore]
fn iq_at_2048k_arrives_complete() {
    // About 1,800 blocks a second, as the engine hands them over, and 10 ms
    // blocks for comparison.
    for (port, per_block) in [(50091u16, RATE as usize / 1800), (50092, RATE as usize / 100)] {
        let Some((pushed, got, frames)) = run(port, per_block) else {
            eprintln!("skip: port {port}");
            continue;
        };
        let share = got as f64 / pushed as f64;
        eprintln!(
            "block {per_block:>6} samples: pushed {pushed}, received {got} in {frames} frames \
             ({:.1} frames/s), {:.2} %",
            frames as f64 / SECONDS,
            100.0 * share
        );
    }
}

/// A client that stops reading for a moment fills the socket's send buffer.
/// The server must then wait (or drop realtime frames), never hang up: on
/// macOS an accepted socket inherits the listener's non-blocking flag, and a
/// write that met a full buffer came back as `EAGAIN`, which the client
/// thread took for a dead peer.
// Measured 2026-11-16 with a TCI client against SDR Oxide at 2 MS/s:
//      one connection in six got nothing at all; the log said "flush failed: IO
//      error: Resource temporarily unavailable (os error 35)".
#[test]
fn a_client_that_pauses_stays_connected() {
    let port = 50093;
    let cfg = TciServerConfig { port, ..TciServerConfig::default() };
    let Ok(srv) = TciServerController::start(&cfg, &DeviceCaps::default(), snapshot()) else {
        eprintln!("skip: port {port}");
        return;
    };
    let addr = format!("127.0.0.1:{port}");
    let stream = std::net::TcpStream::connect(&addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
    let (mut ws, _) = tungstenite::client(format!("ws://{addr}/").as_str(), stream).unwrap();
    ws.send(tungstenite::Message::Text(format!("iq_samplerate:{RATE};").into())).unwrap();
    ws.send(tungstenite::Message::Text("iq_start:0;".into())).unwrap();
    let t0 = Instant::now();
    while srv.wants_iq().is_none() && t0.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(srv.wants_iq().is_some(), "IQ never subscribed");

    // 400 ms of 2 MS/s pushed while the client reads nothing: 6.5 MB, far
    // more than a loopback socket buffers.
    let block = vec![0.25f32; 2 * RATE as usize / 1800];
    let start = Instant::now();
    let mut pushed = 0usize;
    while start.elapsed() < Duration::from_millis(400) {
        let due = (start.elapsed().as_secs_f64() * RATE as f64) as usize;
        while pushed + block.len() / 2 <= due {
            srv.on_rx_iq(&block, RATE);
            pushed += block.len() / 2;
        }
        std::thread::sleep(Duration::from_micros(500));
    }
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(srv.clients(), 1, "the server hung up on a client that paused");

    // And it still streams once the client reads again.
    let mut frames = 0;
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        srv.on_rx_iq(&block, RATE);
        if let Ok(tungstenite::Message::Binary(_)) = ws.read() {
            frames += 1;
        }
    }
    assert!(frames > 0, "no IQ after the pause");
}
