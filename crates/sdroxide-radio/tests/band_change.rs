//! Crossing into another band, and what stops being true when you do.
//!
//! The decode list a digital mode fills is a record of one band's opening. A
//! decode records the audio tone it arrived on and nothing about the dial, so
//! carried across a QSY it is not a stale label: clicking it moves the transmit
//! offset, and a station queued from it is called at that offset, both against
//! a dial that has moved out from under them. The engine's job in that is the
//! two things the client cannot reach — the call queue, which transmits by
//! itself, and the WSJT-X broadcast, whose clients hold their own copy of the
//! list.
//!
//! The distinction under test is band-level, not a delta on the dial: tuning
//! about within a band is the same opening and invalidates nothing.

use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, Mode, RadioEvent, RxId, Vfo, WsjtxConfig};

/// Point the whole process's configuration at a scratch directory, once.
///
/// Without this an engine built from `EngineConfig::default()` reads and writes
/// the **operator's own** `~/.config/sdroxide` — `sdroxide_config::config_dir`
/// keeps `SDROXIDE_CONFIG_DIR` for exactly this reason. It is not merely
/// impolite: `Command::SetWsjtxConfig` is persisted, so a run of these tests
/// used to leave the operator's real `wsjtx.json` switched **on** and pointed at
/// the ephemeral port this test bound and then closed. The next run's engine
/// loaded that at startup, broadcast a `Clear` to it before this test's own
/// broadcast existed, and — whenever the kernel handed a test the same port back
/// — that stray `Clear` landed in the socket below and was counted as a second
/// one. That is what made `a_qsy_clears_the_wsjtx_clients` fail perhaps one run
/// in four, on nothing the engine had done.
///
/// The directory is removed first, so a run never inherits the last one's files.
///
/// ⚠️ Every test in this binary must call this as its FIRST statement.
/// `Once::call_once` makes the write itself safe — every other test is blocked
/// inside it until the variable is set, and nothing is read before it — but a
/// test that starts an engine without calling it would be reading the
/// environment while this writes it.
fn isolate_config() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("sdroxide-band-change-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch config directory");
        // SAFETY: no engine exists yet in any test — see the note above.
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) }
    });
}

/// An engine configuration with a config scope of its own.
///
/// Belt and braces rather than the fix: the scratch directory above is per
/// *process*, and `wsjtx.json` is one of the files a radio scope owns, so
/// without this the three tests in this binary share one. The window that opens
/// is narrow — every engine here loads that file within milliseconds of
/// starting, before any of the tests has written it — and 20 runs on a shared
/// scope showed no failure. But it is a real window under load, and the engine
/// that loses it starts by broadcasting a `Clear` to another test's live
/// socket, which is precisely the failure this file was made flaky by. A scope
/// each closes it, and it is what a station running three radios does anyway.
fn config(radio: u32) -> EngineConfig {
    isolate_config();
    EngineConfig {
        tx_ham_only: false,
        store: sdroxide_config::Store::radio(radio),
        ..Default::default()
    }
}

/// The 20 m FT8 slot, and the 40 m one it QSYs to.
const DIAL_20M: f64 = 14_074_000.0;
const DIAL_40M: f64 = 7_074_000.0;
/// Another spot inside 20 m — the FT4 slot. Far enough to be a real tune,
/// close enough to be the same band opening.
const DIAL_20M_FT4: f64 = 14_080_000.0;
const RATE: f64 = 48_000.0;

/// The WSJT-X `Clear` message type, from `NetworkMessage.hpp`. Spelled out
/// rather than imported because the encoder keeps its type numbers private.
const T_CLEAR: u32 = 3;

/// A silent front end that tunes anywhere: this test is about what the engine
/// decides, not about what a radio does.
struct Silent {
    center: Arc<Mutex<f64>>,
}

impl IqSource for Silent {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(1024);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "silent stand-in".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(10_000.0, 60_000_000.0)],
        freq_ranges_tx: vec![(1_800_000.0, 54_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// Wait for a published state that satisfies `f`.
fn settle(
    h: &sdroxide_radio::EngineHandles,
    mut f: impl FnMut(&sdroxide_types::RadioState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev
                && f(&s)
            {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the engine never published the state this test was waiting for");
}

fn shutdown(mut h: sdroxide_radio::EngineHandles) {
    let thread = h.thread.take();
    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

/// Count the `Clear` datagrams that arrive within `dur`, draining everything
/// else the broadcast sends (heartbeats, status) as it goes.
fn clears_within(sock: &UdpSocket, dur: Duration) -> usize {
    let deadline = Instant::now() + dur;
    let mut clears = 0;
    let mut buf = [0u8; 2048];
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        sock.set_read_timeout(Some(left.max(Duration::from_millis(1)))).unwrap();
        match sock.recv(&mut buf) {
            // Magic, schema, then the type — the first twelve bytes of every
            // message the protocol has.
            Ok(n) if n >= 12 => {
                if u32::from_be_bytes(buf[8..12].try_into().unwrap()) == T_CLEAR {
                    clears += 1;
                }
            }
            Ok(_) => {}
            Err(_) => break, // timed out: nothing more is coming
        }
    }
    clears
}

/// An engine parked on 20 m, broadcasting to a socket this test holds.
///
/// It is parked there by *construction*, not by the command below: the engine
/// takes its opening dial from the front end, and this one's centre is already
/// 20 m — so the band it starts on is the band the tests work from, and the
/// `SetVfo` is the assertion that says so rather than a move. That matters,
/// because a band change still in hand when the broadcast comes up would be
/// announced to it: `poll_band_change` runs on its own tick, after the command
/// that caused it, so the clients would be told about a QSY made before they
/// were listening. Nothing here should be able to produce that, and starting on
/// the band under test is what guarantees it.
fn engine_broadcasting_to(sock: &UdpSocket, radio: u32) -> sdroxide_radio::EngineHandles {
    let center = Arc::new(Mutex::new(DIAL_20M));
    let src = Silent { center };
    let h = start_engine(Box::new(src), caps(), config(radio));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_20M }).unwrap();
    settle(&h, |s| s.vfo_a_hz == DIAL_20M);
    h.cmd_tx
        .send(Command::SetWsjtxConfig(WsjtxConfig {
            enabled: true,
            host: "127.0.0.1".into(),
            port: sock.local_addr().unwrap().port(),
            id: "sdroxide-test".into(),
        }))
        .unwrap();
    // The socket coming up sends one Clear of its own ("a fresh session starts
    // with an empty decode window"). Take it, so what follows is ours.
    assert_eq!(
        clears_within(sock, Duration::from_secs(2)),
        1,
        "starting the broadcast should clear the clients' windows once"
    );
    h
}

/// The reported bug, on the far side of the broadcast: a client left holding
/// the previous band's decodes because nothing told it we had gone.
#[test]
fn a_qsy_clears_the_wsjtx_clients() {
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let h = engine_broadcasting_to(&sock, 1);

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_40M }).unwrap();
    settle(&h, |s| s.vfo_a_hz == DIAL_40M);
    assert_eq!(
        clears_within(&sock, Duration::from_secs(2)),
        1,
        "20 m to 40 m should tell the clients to empty their decode windows"
    );
    shutdown(h);
}

/// The half with teeth: the test is the band, not the size of the step. The FT8
/// slot to the FT4 slot is six kilohertz and the same opening; a client that
/// emptied there would lose the list every time the operator looked at the
/// other end of the segment.
#[test]
fn tuning_about_inside_a_band_tells_the_clients_nothing() {
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let h = engine_broadcasting_to(&sock, 2);

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_20M_FT4 }).unwrap();
    settle(&h, |s| s.vfo_a_hz == DIAL_20M_FT4);
    assert_eq!(
        clears_within(&sock, Duration::from_secs(1)),
        0,
        "moving within 20 m is the same band opening and must clear nothing"
    );
    shutdown(h);
}

/// Wait for a digi status that satisfies `f`, up to three seconds.
fn digi_settles(
    h: &sdroxide_radio::EngineHandles,
    mut f: impl FnMut(&sdroxide_types::DigiStatus) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::Ft8Status(st) = ev
                && f(&st)
            {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// The one that transmits. A station marked to be worked carries the audio
/// offset it was heard at, and the sequencer calls the next one the moment it
/// is free — so a queue that survives a QSY is not a stale list, it is an over
/// on the wrong frequency to somebody who is not on the band at all.
#[test]
fn a_qsy_empties_the_call_queue() {
    let center = Arc::new(Mutex::new(DIAL_20M));
    let h = start_engine(Box::new(Silent { center }), caps(), config(3));
    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_20M }).unwrap();
    h.cmd_tx.send(Command::SetMode { rx: RxId::Main, mode: Mode::Ft8 }).unwrap();
    settle(&h, |s| s.vfo_a_hz == DIAL_20M && s.rx[0].mode == Mode::Ft8);

    // The sequencer has to be busy for the queue to hold anything: idle, it
    // takes the next marked station on the very next poll. Which is the shape
    // of the hazard — a queue is a queue precisely because the operator marks
    // several while working one.
    h.cmd_tx
        .send(Command::DigiStartQso {
            from: "W9XYZ".into(),
            grid: Some("EN52".into()),
            snr: -8,
            audio_hz: 900.0,
            wait_for_cq: false,
        })
        .unwrap();
    h.cmd_tx
        .send(Command::DigiQueueAdd {
            from: "K1ABC".into(),
            grid: Some("FN42".into()),
            snr: -12,
            audio_hz: 1_234.0,
            wait_for_cq: false,
        })
        .unwrap();
    assert!(
        digi_settles(&h, |st| st.call_queue.iter().any(|q| q.call == "K1ABC")),
        "the station should be queued to work on the band it was heard on"
    );

    h.cmd_tx.send(Command::SetVfo { vfo: Vfo::A, hz: DIAL_40M }).unwrap();
    settle(&h, |s| s.vfo_a_hz == DIAL_40M);
    assert!(
        digi_settles(&h, |st| st.call_queue.is_empty()),
        "a QSY must drop the queue: its offsets belong to the dial we have left"
    );
    shutdown(h);
}
