//! Outbound TCI client that follows this radio's dial.
//!
//! sdroxide's other TCI client ([`crate::TciDevice`]) takes wideband IQ from
//! somebody else's rig and uses that rig as the front end. This one does not.
//! It connects to a TCI server — 590Ctrl, in the setup it was written for —
//! and sends the VFO whenever the operator tunes here, so that radio moves
//! with the dial. One text line, `vfo:0,0,<hz>;`. No IQ, no audio, and never
//! `trx:` or `tune:`: those key the transmitter, and a tune of the dial is
//! not a request to go on the air.
//!
//! It also listens. When the operator turns the knob on the far radio, the
//! server broadcasts `vfo:0,0,<hz>;` and this client hands it to the engine,
//! so the dial here follows the radio as well. 590Ctrl answers our own
//! frequency commands with the same broadcast, and acting on those would chase
//! our own echo, so any frequency sent in the last [`ECHO_WINDOW`] is ignored,
//! and nothing heard is acted on until the dial has been still for
//! [`QUIET_AFTER_SEND`].

use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use tungstenite::{Message, WebSocket};

use crate::protocol as p;
use crate::{HANDSHAKE_TIMEOUT, connect_socket, split_addr, ws_handshake};

/// How long a connect attempt may sit before the worker tries again.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);

/// Read timeout once the socket is up. Also the longest [`TciTuneClient`]'s
/// drop waits to join the worker, and the fastest a new frequency goes out.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// Don't hand the far radio a new frequency more often than this. A waterfall
/// drag is a frequency per frame; the server only needs the latest one, and
/// 590Ctrl still has to turn each one into a CAT command.
const MIN_SEND_GAP: Duration = Duration::from_millis(50);

/// A broadcast of any frequency sent this recently is our own echo. During a
/// drag the server's answers trail the dial, so this covers more than the
/// newest value.
const ECHO_WINDOW: Duration = Duration::from_secs(2);

/// What the radio reports right after a send may be a poll taken before the
/// command landed. Hold anything heard until the dial here has been still
/// this long, by which time the server has caught up with the newest value.
const QUIET_AFTER_SEND: Duration = Duration::from_millis(300);

/// What the settings dialog shows for the outbound client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TciTuneStatus {
    pub connected: bool,
    pub error: Option<String>,
}

/// The engine's handle on the outbound client. Dropping it closes the socket.
pub struct TciTuneClient {
    hz: Arc<AtomicU64>,
    address: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
    status_rx: Receiver<TciTuneStatus>,
    heard_rx: Receiver<u64>,
    thread: Option<JoinHandle<()>>,
}

impl TciTuneClient {
    /// Start connecting to `address` (`host[:port]`, port 50001 if omitted).
    /// The socket comes up on the worker thread; [`Self::poll_status`] reports
    /// when it has.
    pub fn start(address: String) -> Self {
        let address = Arc::new(Mutex::new(address));
        let hz = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (status_tx, status_rx) = crossbeam_channel::unbounded();
        let (heard_tx, heard_rx) = crossbeam_channel::unbounded();
        let thread = thread::spawn({
            let address = Arc::clone(&address);
            let hz = Arc::clone(&hz);
            let stop = Arc::clone(&stop);
            move || worker(address, hz, stop, status_tx, heard_tx)
        });
        Self { hz, address, stop, status_rx, heard_rx, thread: Some(thread) }
    }

    /// The address the worker is aiming at.
    pub fn address(&self) -> String {
        self.address.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Point at a different server. The worker drops the old socket and
    /// connects to this one; the frequency it has is sent again once up.
    pub fn set_address(&self, address: String) {
        if let Ok(mut g) = self.address.lock() {
            *g = address;
        }
    }

    /// Remember `hz` as the dial to send. Only the newest value is kept, so a
    /// drag does not queue a line per step.
    pub fn tune(&self, hz: u64) {
        self.hz.store(hz, Ordering::Relaxed);
    }

    /// The newest status the worker has reported since the last call.
    pub fn poll_status(&self) -> Option<TciTuneStatus> {
        let mut last = None;
        while let Ok(status) = self.status_rx.try_recv() {
            last = Some(status);
        }
        last
    }

    /// The newest frequency the far radio was tuned to from its own side
    /// since the last call. The worker already counts it as sent, so tuning
    /// the dial here to it does not send it back.
    pub fn poll_heard(&self) -> Option<u64> {
        let mut last = None;
        while let Ok(hz) = self.heard_rx.try_recv() {
            last = Some(hz);
        }
        last
    }
}

impl Drop for TciTuneClient {
    fn drop(&mut self) {
        // Don't join: a worker inside `connect` can take seconds to notice,
        // and this runs on the engine thread. It exits on its own.
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take();
    }
}

/// The one command this client ever writes.
pub(crate) fn vfo_line(hz: u64) -> String {
    format!("vfo:0,0,{hz};")
}

/// The receiver 0, channel 0 frequency in a line from the server, if it has
/// one. That is the VFO [`vfo_line`] sets; the other channel is VFO B.
pub(crate) fn heard_vfo(text: &str) -> Option<u64> {
    let mut found = None;
    for (cmd, args) in p::parse_status(text) {
        if cmd != "vfo" {
            continue;
        }
        let f: Vec<&str> = args.split(',').map(str::trim).collect();
        if let [rx, ch, hz] = f.as_slice()
            && *rx == "0"
            && *ch == "0"
            && let Ok(hz) = hz.parse::<u64>()
            && hz > 0
        {
            found = Some(hz);
        }
    }
    found
}

/// What the worker knows about frequencies going each way.
struct Dial {
    /// The last frequency on the far radio we know of: sent by us, or heard.
    sent: u64,
    /// Everything sent within [`ECHO_WINDOW`], oldest first.
    recent: VecDeque<(u64, Instant)>,
    /// The newest frequency heard that was not an echo, waiting for quiet.
    pending: Option<u64>,
}

impl Dial {
    fn new() -> Self {
        Self { sent: 0, recent: VecDeque::new(), pending: None }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn record_sent(&mut self, hz: u64, now: Instant) {
        self.sent = hz;
        self.recent.push_back((hz, now));
    }

    fn heard(&mut self, hz: u64, now: Instant) {
        while self.recent.front().is_some_and(|(_, t)| now.duration_since(*t) > ECHO_WINDOW) {
            self.recent.pop_front();
        }
        // Each report replaces the last: the newest is where the radio is.
        // A report of something we sent means the radio is on our dial.
        self.pending = if hz == self.sent || self.recent.iter().any(|(f, _)| *f == hz) {
            None
        } else {
            Some(hz)
        };
    }
}

fn worker(
    address: Arc<Mutex<String>>,
    hz: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    status_tx: Sender<TciTuneStatus>,
    heard_tx: Sender<u64>,
) {
    let mut backoff = Duration::from_millis(200);
    let mut socket: Option<WebSocket<TcpStream>> = None;
    let mut connected_to = String::new();
    let mut dial = Dial::new();
    let mut last_tx = Instant::now() - MIN_SEND_GAP;
    let mut reported = TciTuneStatus { connected: false, error: None };

    while !stop.load(Ordering::Relaxed) {
        let want_addr = address.lock().map(|g| g.clone()).unwrap_or_default();
        if socket.is_some() && want_addr != connected_to {
            close(&mut socket);
            dial.reset();
            report(&status_tx, &mut reported, TciTuneStatus { connected: false, error: None });
        }

        if socket.is_none() {
            if want_addr.trim().is_empty() {
                report(
                    &status_tx,
                    &mut reported,
                    TciTuneStatus {
                        connected: false,
                        error: Some("TCI Client needs a host:port".into()),
                    },
                );
                sleep_until_stop(&stop, Duration::from_millis(200));
                continue;
            }
            match connect(&want_addr) {
                Ok(ws) => {
                    tracing::info!(addr = %want_addr, "TCI client connected");
                    socket = Some(ws);
                    connected_to = want_addr;
                    backoff = Duration::from_millis(200);
                    dial.reset();
                    report(
                        &status_tx,
                        &mut reported,
                        TciTuneStatus { connected: true, error: None },
                    );
                }
                Err(error) => {
                    tracing::warn!("TCI client: {error}");
                    report(
                        &status_tx,
                        &mut reported,
                        TciTuneStatus { connected: false, error: Some(error) },
                    );
                    sleep_until_stop(&stop, backoff);
                    backoff = (backoff * 2).min(Duration::from_secs(5));
                    continue;
                }
            }
        }

        if !drain(&mut socket, &mut dial, &status_tx, &mut reported) {
            dial.reset();
            continue;
        }

        // The radio was tuned from its own side. Count it as sent before the
        // engine hears of it, so the dial moving here does not echo it back.
        // Swapped in only if the engine has not tuned since, which would be
        // the newer word.
        if let Some(heard) = dial.pending
            && last_tx.elapsed() >= QUIET_AFTER_SEND
        {
            dial.pending = None;
            let had = hz.load(Ordering::Relaxed);
            // On connect the server's state can arrive before our first send
            // of the same frequency, and there is then no echo to clear it.
            if heard != dial.sent
                && had == dial.sent
                && hz.compare_exchange(had, heard, Ordering::Relaxed, Ordering::Relaxed).is_ok()
            {
                tracing::debug!(hz = heard, "TCI client: radio tuned");
                dial.sent = heard;
                let _ = heard_tx.send(heard);
            }
        }

        let want = hz.load(Ordering::Relaxed);
        if want > 0 && want != dial.sent && last_tx.elapsed() >= MIN_SEND_GAP {
            let line = vfo_line(want);
            match send_line(socket.as_mut(), line) {
                Ok(()) => {
                    tracing::trace!(hz = want, "TCI client tune");
                    last_tx = Instant::now();
                    dial.record_sent(want, last_tx);
                }
                Err(error) => {
                    tracing::warn!("TCI client send: {error}");
                    close(&mut socket);
                    dial.reset();
                    report(
                        &status_tx,
                        &mut reported,
                        TciTuneStatus { connected: false, error: Some(format!("send: {error}")) },
                    );
                }
            }
        }
    }
    close(&mut socket);
}

fn send_line(socket: Option<&mut WebSocket<TcpStream>>, line: String) -> Result<(), String> {
    let Some(ws) = socket else {
        return Err("not connected".into());
    };
    ws.send(Message::Text(line.into())).map_err(|error| error.to_string())?;
    ws.flush().map_err(|error| error.to_string())
}

/// Read whatever the server has queued, up to a bound so a chatty server
/// cannot starve the send. `false` means the socket is gone.
fn drain(
    socket: &mut Option<WebSocket<TcpStream>>,
    dial: &mut Dial,
    status_tx: &Sender<TciTuneStatus>,
    reported: &mut TciTuneStatus,
) -> bool {
    for _ in 0..64 {
        // The read borrows the socket. Finish it before `close` takes the
        // `Option` back, or the two borrows overlap.
        let read = match socket.as_mut() {
            Some(ws) => ws.read(),
            None => return false,
        };
        match read {
            Ok(Message::Close(_)) | Err(tungstenite::Error::ConnectionClosed) => {
                close(socket);
                report(
                    status_tx,
                    reported,
                    TciTuneStatus {
                        connected: false,
                        error: Some("the TCI server closed the connection".into()),
                    },
                );
                return false;
            }
            Ok(Message::Text(text)) => {
                if let Some(hz) = heard_vfo(&text) {
                    dial.heard(hz, Instant::now());
                }
            }
            Ok(_) => continue,
            Err(tungstenite::Error::Io(error))
                if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut =>
            {
                return true;
            }
            Err(error) => {
                tracing::warn!("TCI client read: {error}");
                close(socket);
                report(
                    status_tx,
                    reported,
                    TciTuneStatus { connected: false, error: Some(format!("read: {error}")) },
                );
                return false;
            }
        }
    }
    true
}

fn connect(address: &str) -> Result<WebSocket<TcpStream>, String> {
    let (host, port) = split_addr(address)?;
    let sockaddr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| format!("resolve {host}:{port}: {error}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))?;
    let stream = connect_socket(&sockaddr, &host, port, CONNECT_TIMEOUT).map_err(|error| {
        // `connect_socket` names ExpertSDR3, which is the IQ client. This
        // socket is the control client, and the program on the other end is
        // whatever is serving TCI there — 590Ctrl, for the usual one.
        if error.contains("nothing is serving TCI") {
            format!(
                "connect {host}:{port}: nothing is listening. Start 590Ctrl and turn its TCI \
                 server on, on this port"
            )
        } else {
            error
        }
    })?;
    let url = format!("ws://{host}:{port}/");
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)).ok();
    let ws = ws_handshake(stream, &url, deadline)?;
    ws.get_ref().set_read_timeout(Some(READ_TIMEOUT)).ok();
    // A frequency line is a few dozen bytes. Holding it for a full segment
    // would put the radio a segment behind the dial.
    ws.get_ref().set_nodelay(true).ok();
    Ok(ws)
}

fn close(socket: &mut Option<WebSocket<TcpStream>>) {
    if let Some(mut ws) = socket.take() {
        let _ = ws.close(None);
        let _ = ws.flush();
    }
}

fn report(tx: &Sender<TciTuneStatus>, last: &mut TciTuneStatus, next: TciTuneStatus) {
    if *last == next {
        return;
    }
    *last = next.clone();
    let _ = tx.send(next);
}

fn sleep_until_stop(stop: &AtomicBool, total: Duration) {
    let until = Instant::now() + total;
    while !stop.load(Ordering::Relaxed) && Instant::now() < until {
        let slice = Duration::from_millis(50).min(until.saturating_duration_since(Instant::now()));
        thread::sleep(slice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn vfo_line_is_a_frequency_and_not_a_key() {
        let line = vfo_line(7_074_000);
        assert_eq!(line, "vfo:0,0,7074000;");
        assert!(!line.contains("trx"));
        assert!(!line.contains("tune:"));
        assert!(!line.contains("audio"));
        assert!(!line.contains("iq"));
    }

    #[test]
    fn heard_vfo_reads_only_vfo_a_of_receiver_zero() {
        assert_eq!(heard_vfo("VFO:0,0,14074000;"), Some(14_074_000));
        assert_eq!(heard_vfo("dds:0,14074000;vfo:0,1,7000000;"), None);
        assert_eq!(heard_vfo("vfo:1,0,7000000;"), None);
        assert_eq!(heard_vfo("vfo:0,0,7000000;vfo:0,0,7001000;"), Some(7_001_000));
        assert_eq!(heard_vfo("vfo:0,0,junk;"), None);
    }

    #[test]
    fn an_echo_of_anything_recently_sent_is_not_a_tune() {
        let now = Instant::now();
        let mut dial = Dial::new();
        dial.record_sent(7_074_000, now);
        dial.record_sent(7_075_000, now);
        // A trailing echo of the older value during a drag.
        dial.heard(7_074_000, now);
        assert_eq!(dial.pending, None);
        dial.heard(7_080_000, now);
        assert_eq!(dial.pending, Some(7_080_000));
        // The server caught up with our newest value: the radio is on it.
        dial.heard(7_075_000, now);
        assert_eq!(dial.pending, None);
    }

    /// Accept one client, send it `lines` once it has spoken, and collect
    /// what it writes until `until` appears or a few seconds pass.
    fn fake_server(
        lines: Vec<(Duration, &'static str)>,
        until: &'static str,
    ) -> (u16, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            stream.set_read_timeout(Some(Duration::from_millis(20))).ok();
            let mut ws = tungstenite::accept(stream).expect("handshake");
            let _ = ws.send(Message::Text("protocol:590Ctrl,1.9;ready;".into()));
            let mut got = Vec::new();
            let mut queue: VecDeque<_> = lines.into();
            let mut started: Option<Instant> = None;
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if let (Some(t0), Some((at, _))) = (started, queue.front())
                    && t0.elapsed() >= *at
                {
                    let (_, line) = queue.pop_front().expect("line");
                    let _ = ws.send(Message::Text(line.into()));
                }
                match ws.read() {
                    Ok(Message::Text(text)) => {
                        started.get_or_insert_with(Instant::now);
                        let text = text.to_string();
                        let done = !until.is_empty() && text.contains(until);
                        got.push(text);
                        if done {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(error))
                        if error.kind() == ErrorKind::WouldBlock
                            || error.kind() == ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
            }
            got
        });
        (port, server)
    }

    fn wait_connected(client: &TciTuneClient) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if client.poll_status().is_some_and(|status| status.connected) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("client never connected");
    }

    #[test]
    fn a_knob_turn_on_the_radio_reaches_the_dial_and_is_not_sent_back() {
        // Our tune is echoed, then the operator turns the radio's knob.
        let (port, server) = fake_server(
            vec![
                (Duration::from_millis(30), "vfo:0,0,7074000;"),
                (Duration::from_millis(600), "vfo:0,0,7071000;"),
            ],
            "",
        );
        let client = TciTuneClient::start(format!("127.0.0.1:{port}"));
        wait_connected(&client);
        client.tune(7_074_000);

        let mut heard = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && heard.is_empty() {
            heard.extend(client.poll_heard());
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(heard, vec![7_071_000], "the echo must be ignored and the knob turn heard");
        // The engine tunes the dial to what it heard; that must stay here.
        client.tune(7_071_000);
        thread::sleep(Duration::from_millis(300));
        drop(client);

        let lines = server.join().expect("server");
        let joined = lines.join("");
        assert!(joined.contains("vfo:0,0,7074000;"), "{lines:?}");
        assert!(!joined.contains("7071000"), "the heard frequency went back out: {lines:?}");
    }

    #[test]
    fn a_tune_is_one_vfo_line_and_nothing_that_keys_or_streams() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            stream.set_read_timeout(Some(Duration::from_millis(200))).ok();
            let mut ws = tungstenite::accept(stream).expect("handshake");
            let _ = ws.send(Message::Text("protocol:590Ctrl,1.9;ready;".into()));
            let mut lines = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                match ws.read() {
                    Ok(Message::Text(text)) => {
                        let text = text.to_string();
                        let done = text.contains("7075500");
                        lines.push(text);
                        if done {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(error))
                        if error.kind() == ErrorKind::WouldBlock
                            || error.kind() == ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
            }
            lines
        });

        let client = TciTuneClient::start(format!("127.0.0.1:{port}"));
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if client.poll_status().is_some_and(|status| status.connected) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        // Two tunes inside the send gap: the radio must end on the later one,
        // and neither line may be anything but a VFO.
        client.tune(7_074_000);
        client.tune(7_075_500);

        let lines = server.join().expect("server");
        let joined = lines.join("");
        assert!(joined.contains("vfo:0,0,7075500;"), "expected the final dial, got {lines:?}");
        let lower = joined.to_ascii_lowercase();
        assert!(!lower.contains("trx:"), "{lines:?}");
        assert!(!lower.contains("tune:"), "{lines:?}");
        assert!(!lower.contains("audio"), "{lines:?}");
        assert!(!lower.contains("iq"), "{lines:?}");
        drop(client);
    }
}
