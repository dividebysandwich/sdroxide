//! Beast over TCP, both ways.
//!
//! [`BeastServer`] is the output: it listens (30005 by convention) and sends
//! every frame this station's own demodulator accepts to every client that
//! connects — `readsb` with a `beast_in` net-connector, `tar1090`'s feeders,
//! `mlat-client`'s Beast input, a second SDR Oxide.
//!
//! [`BeastClient`] is the input: it connects to somebody else's Beast output
//! and hands the frames over, reconnecting when the far end goes away or falls
//! silent. Every decoder in the process that asks for the same address shares
//! one connection.
//!
//! # Why the two never meet
//!
//! A station that both serves its frames and subscribes to an aggregator that
//! merges them gets its own frames back. Were anything received here passed
//! to the output, they would go round for ever. So the output is fed by the
//! demodulator and by nothing else: [`BeastOut`] is handed to the worker's
//! decode path only, and the frames a [`BeastClient`] delivers go to the
//! aircraft table and stop there. The barrier is in the wiring, not in a
//! filter that could be got wrong — see the controller's tests.

use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TrySendError, bounded};
use tracing::{info, warn};

use crate::beast::{BeastFrame, Parser};

/// How long the server waits for a frame before looking for new clients.
const ACCEPT_POLL: Duration = Duration::from_millis(100);

/// A client whose socket will not take this much more is dropped rather than
/// let it hold up the rest. Two seconds of a busy sector.
const CLIENT_BACKLOG_MAX: usize = 256 * 1024;

/// How often the output says it is alive when it has nothing else to say, and
/// what it says: readsb's heartbeat, a Mode A/C frame of zeros. A reader can
/// then tell a quiet sky from a dead connection — see [`IDLE_MAX`].
const HEARTBEAT: Duration = Duration::from_secs(60);
const HEARTBEAT_FRAME: [u8; 11] = [0x1a, b'1', 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Reconnect backoff for the client, doubling from the first to the last.
const RETRY_FIRST: Duration = Duration::from_secs(2);
const RETRY_MAX: Duration = Duration::from_secs(30);

/// A handle the decoder writes encoded frames into. Cheap to clone; dropping
/// every handle and the server stops it.
#[derive(Clone)]
pub struct BeastOut {
    tx: Sender<Vec<u8>>,
}

impl BeastOut {
    /// Hand over bytes already in Beast framing. Never blocks: on a server
    /// that has fallen behind the bytes are dropped, as I/Q is.
    pub fn send(&self, bytes: Vec<u8>) {
        let _ = self.tx.try_send(bytes);
    }

    /// A handle that goes nowhere but into the returned receiver — for tests.
    pub fn capture() -> (BeastOut, Receiver<Vec<u8>>) {
        let (tx, rx) = bounded(4096);
        (BeastOut { tx }, rx)
    }
}

/// The Beast output: a listening socket and the thread that serves it.
pub struct BeastServer {
    out: BeastOut,
    addr: SocketAddr,
    clients: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl BeastServer {
    /// Bind `addr` ("0.0.0.0:30005") and start serving. Fails as the bind
    /// fails, so "port already in use" reaches the operator.
    pub fn bind(addr: &str) -> std::io::Result<BeastServer> {
        Self::bind_with(addr, HEARTBEAT)
    }

    fn bind_with(addr: &str, heartbeat: Duration) -> std::io::Result<BeastServer> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let local = listener.local_addr()?;
        let (tx, rx) = bounded::<Vec<u8>>(4096);
        let clients = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let (clients, stop) = (clients.clone(), stop.clone());
            std::thread::Builder::new()
                .name("sdroxide-beast-out".into())
                .spawn(move || serve(listener, rx, clients, stop, heartbeat))?
        };
        info!(%local, "Beast output listening");
        Ok(BeastServer { out: BeastOut { tx }, addr: local, clients, stop, worker: Some(worker) })
    }

    pub fn out(&self) -> BeastOut {
        self.out.clone()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn clients(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }
}

impl Drop for BeastServer {
    fn drop(&mut self) {
        // Not joined: the thread may sit in a DNS lookup or a connect
        // timeout, and this drop runs on the engine thread when the ADS-B
        // lane stops. It sees `stop`, or its send failing, and ends itself.
        self.stop.store(true, Ordering::Relaxed);
        drop(self.worker.take());
        info!(addr = %self.addr, "Beast output stopped");
    }
}

struct Peer {
    sock: TcpStream,
    /// What the socket has not taken yet.
    backlog: Vec<u8>,
}

fn serve(
    listener: TcpListener,
    rx: Receiver<Vec<u8>>,
    clients: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    heartbeat: Duration,
) {
    let mut peers: Vec<Peer> = Vec::new();
    let mut scratch = [0u8; 512];
    let mut said = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        loop {
            match listener.accept() {
                Ok((sock, from)) => {
                    // Non-blocking writes, so one stalled reader cannot hold
                    // up the others; Nagle off, since a frame is a few dozen
                    // bytes and a feeder wants it now.
                    if sock.set_nonblocking(true).is_ok() {
                        let _ = sock.set_nodelay(true);
                        info!(%from, "Beast client connected");
                        peers.push(Peer { sock, backlog: Vec::new() });
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    warn!("Beast accept: {e}");
                    break;
                }
            }
        }

        let chunk = match rx.recv_timeout(ACCEPT_POLL) {
            Ok(first) => {
                let mut c = first;
                while let Ok(more) = rx.try_recv() {
                    c.extend_from_slice(&more);
                }
                c
            }
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let chunk = if chunk.is_empty() && said.elapsed() >= heartbeat {
            HEARTBEAT_FRAME.to_vec()
        } else {
            chunk
        };
        if !chunk.is_empty() {
            said = Instant::now();
        }

        peers.retain_mut(|p| {
            // A Beast output has nothing to hear, but a reader that closed
            // shows up here first, as a zero-length read.
            match p.sock.read(&mut scratch) {
                Ok(0) => return false,
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(_) => return false,
            }
            p.backlog.extend_from_slice(&chunk);
            while !p.backlog.is_empty() {
                match p.sock.write(&p.backlog) {
                    Ok(0) => return false,
                    Ok(n) => {
                        p.backlog.drain(..n);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => return false,
                }
            }
            if p.backlog.len() > CLIENT_BACKLOG_MAX {
                warn!("Beast client too slow; dropped");
                let _ = p.sock.shutdown(Shutdown::Both);
                return false;
            }
            true
        });
        clients.store(peers.len(), Ordering::Relaxed);
    }
    clients.store(0, Ordering::Relaxed);
}

/// What the input is doing, for the panel's header.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FeedState {
    #[default]
    Connecting,
    Connected,
    /// Down, and why; the client is waiting to try again.
    Down(String),
}

/// How long a connected input may go without a single byte before it is
/// counted dead and dialled again.
///
/// A Beast output sends a heartbeat at least once a minute when its sky is
/// empty (readsb and dump1090 both do; so does [`BeastServer`]), and this
/// client only reads, so a far end that rebooted or a NAT that forgot the
/// connection is otherwise never noticed: the socket stays open, nothing
/// arrives, and the table empties as if the sky had. Two and a half
/// heartbeats is late enough not to trip on one lost to a slow link.
const IDLE_MAX: Duration = Duration::from_secs(150);

/// The input as callers hold it: a subscription to the one connection this
/// process keeps to `addr`.
///
/// Every radio in the ADS-B mode with a Beast feed starts a decoder, and each
/// decoder asks for the feed. Were each to dial its own, four radios would pull
/// the same stream four times over the same link — on a Wi-Fi hop to a
/// receiver outdoors that is the difference between a feed that holds and one
/// the far end drops for being too slow. So the connection is shared: the
/// first subscriber to an address opens it, every later one is handed the
/// same frames, and the last to leave closes it.
pub struct BeastClient {
    addr: String,
    id: u64,
    link: Arc<Link>,
}

/// One connection, and the thread that keeps it.
struct Link {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

/// Who is listening to a connection, each under the id its handle carries.
type Subs = Mutex<Vec<(u64, Sender<Vec<BeastFrame>>)>>;

/// What the connection's thread and its subscribers both see.
struct Shared {
    state: Mutex<FeedState>,
    subs: Subs,
    stop: AtomicBool,
    idle_max: Duration,
}

/// The connections open in this process, by address. Weak, so the last
/// subscriber's drop is what closes a connection, not this table.
static LINKS: LazyLock<Mutex<HashMap<String, Weak<Link>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl BeastClient {
    /// Subscribe to the Beast output at `addr` ("192.168.178.127:30005"),
    /// connecting if nobody in this process is yet. Frames go to `tx` in
    /// batches, one per read; a full channel drops the batch for that
    /// subscriber only.
    pub fn start(addr: &str, tx: Sender<Vec<BeastFrame>>) -> BeastClient {
        Self::start_with(addr, tx, IDLE_MAX)
    }

    fn start_with(addr: &str, tx: Sender<Vec<BeastFrame>>, idle_max: Duration) -> BeastClient {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut links = LINKS.lock().unwrap_or_else(|e| e.into_inner());
        let link = match links.get(addr).and_then(Weak::upgrade) {
            Some(l) => l,
            None => {
                let l = Link::open(addr, idle_max);
                links.insert(addr.to_string(), Arc::downgrade(&l));
                l
            }
        };
        // Under the table's lock, so a link cannot lose its last subscriber
        // and stop between being found and being joined.
        if let Ok(mut s) = link.shared.subs.lock() {
            s.push((id, tx));
        }
        drop(links);
        BeastClient { addr: addr.to_string(), id, link }
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn state(&self) -> FeedState {
        self.link.shared.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for BeastClient {
    fn drop(&mut self) {
        if let Ok(mut s) = self.link.shared.subs.lock() {
            s.retain(|(id, _)| *id != self.id);
        }
        // The `Arc<Link>` goes with this; if it was the last, the link stops.
    }
}

impl Link {
    fn open(addr: &str, idle_max: Duration) -> Arc<Link> {
        let shared = Arc::new(Shared {
            state: Mutex::new(FeedState::Connecting),
            subs: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
            idle_max,
        });
        let worker = {
            let (addr, shared) = (addr.to_string(), shared.clone());
            std::thread::Builder::new()
                .name("sdroxide-beast-in".into())
                .spawn(move || subscribe(&addr, &shared))
                .ok()
        };
        Arc::new(Link { shared, worker: Mutex::new(worker) })
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        // Not joined, as the output is not: the thread may sit in a DNS
        // lookup or a five-second connect, and the last subscriber leaves on
        // the engine thread when the ADS-B lane stops. It sees `stop` within
        // a read timeout and ends itself.
        self.shared.stop.store(true, Ordering::Relaxed);
        drop(self.worker.lock().ok().and_then(|mut w| w.take()));
    }
}

fn set(state: &Mutex<FeedState>, s: FeedState) {
    if let Ok(mut g) = state.lock() {
        *g = s;
    }
}

/// Sleep `d` in short steps, so a stop is not kept waiting half a minute.
/// Returns false if stopped.
fn wait(d: Duration, stop: &AtomicBool) -> bool {
    let until = Instant::now() + d;
    while Instant::now() < until {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    !stop.load(Ordering::Relaxed)
}

fn connect(addr: &str) -> std::io::Result<TcpStream> {
    let mut last = std::io::Error::new(ErrorKind::NotFound, "address did not resolve");
    for a in addr.to_socket_addrs()? {
        match TcpStream::connect_timeout(&a, Duration::from_secs(5)) {
            Ok(s) => return Ok(s),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Hand a batch to every subscriber. A subscriber whose decoder has gone is
/// struck off; one that is merely behind loses this batch, and nobody else
/// waits for it.
fn fan_out(subs: &Subs, frames: Vec<BeastFrame>) {
    let Ok(mut s) = subs.lock() else { return };
    s.retain(|(_, tx)| !matches!(tx.try_send(frames.clone()), Err(TrySendError::Disconnected(_))));
}

fn subscribe(addr: &str, shared: &Shared) {
    let stop = &shared.stop;
    let mut retry = RETRY_FIRST;
    let mut buf = vec![0u8; 16 * 1024];
    while !stop.load(Ordering::Relaxed) {
        set(&shared.state, FeedState::Connecting);
        let why = match connect(addr) {
            Ok(mut sock) => {
                let _ = sock.set_read_timeout(Some(Duration::from_millis(250)));
                set(&shared.state, FeedState::Connected);
                info!(addr, "Beast input connected");
                retry = RETRY_FIRST;
                let mut parser = Parser::new();
                let mut heard = Instant::now();
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    match sock.read(&mut buf) {
                        Ok(0) => break "closed by the far end".to_string(),
                        Ok(n) => {
                            heard = Instant::now();
                            let frames = parser.push(&buf[..n]);
                            if !frames.is_empty() {
                                fan_out(&shared.subs, frames);
                            }
                        }
                        Err(e)
                            if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                        {
                            if heard.elapsed() >= shared.idle_max {
                                // Closing here is the point: the far end may
                                // still hold its half, and a socket kept open
                                // on our side is one more it has to time out.
                                let _ = sock.shutdown(Shutdown::Both);
                                break format!(
                                    "nothing received for {} s",
                                    shared.idle_max.as_secs()
                                );
                            }
                        }
                        Err(e) => break e.to_string(),
                    }
                }
            }
            Err(e) => e.to_string(),
        };
        warn!(addr, %why, "Beast input down; retrying in {retry:?}");
        set(&shared.state, FeedState::Down(why));
        if !wait(retry, stop) {
            return;
        }
        retry = (retry * 2).min(RETRY_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beast::encode;

    /// Bytes handed to the server reach a client that connected, unchanged,
    /// and the server counts the client.
    #[test]
    fn the_server_sends_what_it_is_given_to_a_client() {
        let srv = BeastServer::bind("127.0.0.1:0").unwrap();
        let mut c = TcpStream::connect(srv.local_addr()).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let t0 = Instant::now();
        while srv.clients() == 0 && t0.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(srv.clients(), 1);

        let mut wire = Vec::new();
        encode(&mut wire, 1, 2, &[0x5D, 0x4C, 0xA2, 0xD4, 0xA5, 0xA0, 0xF6]);
        srv.out().send(wire.clone());
        let mut got = vec![0u8; wire.len()];
        c.read_exact(&mut got).unwrap();
        assert_eq!(got, wire);

        drop(c);
        let t0 = Instant::now();
        while srv.clients() != 0 && t0.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(srv.clients(), 0, "a client that left is not counted");
    }

    /// A port already held is the error the operator sees, not a silent
    /// output that never sends.
    #[test]
    fn a_port_in_use_is_an_error() {
        let a = BeastServer::bind("127.0.0.1:0").unwrap();
        assert!(BeastServer::bind(&a.local_addr().to_string()).is_err());
    }

    /// The client reads frames from a Beast output and, when the output goes
    /// away and comes back on the same port, connects again by itself.
    #[test]
    fn the_client_reads_frames_and_reconnects() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let (tx, rx) = bounded(16);
        let client = BeastClient::start(&addr.to_string(), tx);

        let mut wire = Vec::new();
        encode(&mut wire, 9, 9, &[0x5D, 0x4C, 0xA2, 0xD4, 0xA5, 0xA0, 0xF6]);
        let (mut s, _) = l.accept().unwrap();
        s.write_all(&wire).unwrap();
        let got = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(got[0].timestamp, 9);
        assert_eq!(client.state(), FeedState::Connected);

        drop(s);
        let t0 = Instant::now();
        while !matches!(client.state(), FeedState::Down(_)) && t0.elapsed() < Duration::from_secs(3)
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(matches!(client.state(), FeedState::Down(_)), "{:?}", client.state());

        // Back after the first backoff.
        let (mut s, _) = l.accept().unwrap();
        s.write_all(&wire).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    /// Two decoders on one address make one connection, both get every
    /// frame, and the connection lives exactly as long as its last subscriber.
    #[test]
    fn two_decoders_on_one_address_share_one_connection() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let (tx1, rx1) = bounded(16);
        let (tx2, rx2) = bounded(16);
        let c1 = BeastClient::start(&addr, tx1);
        let c2 = BeastClient::start(&addr, tx2);

        let (mut s, _) = l.accept().unwrap();
        l.set_nonblocking(true).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(l.accept().is_err(), "a second subscriber dialled its own connection");

        let mut wire = Vec::new();
        encode(&mut wire, 7, 9, &[0x5D, 0x4C, 0xA2, 0xD4, 0xA5, 0xA0, 0xF6]);
        s.write_all(&wire).unwrap();
        assert_eq!(rx1.recv_timeout(Duration::from_secs(3)).unwrap()[0].timestamp, 7);
        assert_eq!(rx2.recv_timeout(Duration::from_secs(3)).unwrap()[0].timestamp, 7);

        // The first leaves; the second still hears.
        drop(c1);
        s.write_all(&wire).unwrap();
        assert!(rx2.recv_timeout(Duration::from_secs(3)).is_ok());
        assert_eq!(c2.state(), FeedState::Connected);

        // The last leaves; the connection closes.
        drop(c2);
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut b = [0u8; 16];
        assert_eq!(s.read(&mut b).unwrap(), 0, "the connection outlived its subscribers");
    }

    /// An input that stops hearing anything — the far end rebooted, a NAT
    /// forgot the mapping — is closed and dialled again, instead of waiting on
    /// a socket nothing will ever arrive on.
    #[test]
    fn a_silent_input_is_closed_and_dialled_again() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let (tx, rx) = bounded(16);
        let client = BeastClient::start_with(&addr, tx, Duration::from_millis(500));

        let (mut s, _) = l.accept().unwrap();
        let mut wire = Vec::new();
        encode(&mut wire, 3, 9, &[0x5D, 0x4C, 0xA2, 0xD4, 0xA5, 0xA0, 0xF6]);
        s.write_all(&wire).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());

        // Hold the socket open and say nothing.
        let t0 = Instant::now();
        while !matches!(client.state(), FeedState::Down(_)) && t0.elapsed() < Duration::from_secs(3)
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        match client.state() {
            FeedState::Down(why) => assert!(why.contains("nothing received"), "{why}"),
            other => panic!("still {other:?} after a silence"),
        }
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut b = [0u8; 16];
        assert_eq!(s.read(&mut b).unwrap(), 0, "the silent socket was left open");

        // And it comes back by itself.
        let (mut s2, _) = l.accept().unwrap();
        s2.write_all(&wire).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    /// A quiet output still speaks: a heartbeat, so its readers can tell an
    /// empty sky from a dead link.
    #[test]
    fn a_quiet_output_sends_a_heartbeat() {
        let srv = BeastServer::bind_with("127.0.0.1:0", Duration::from_millis(200)).unwrap();
        let mut c = TcpStream::connect(srv.local_addr()).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut got = [0u8; 11];
        c.read_exact(&mut got).unwrap();
        assert_eq!(got, HEARTBEAT_FRAME);
    }

    /// The heartbeat parses as a Mode A/C frame, which the table skips: it
    /// keeps the link alive and adds nothing to the sky.
    #[test]
    fn the_heartbeat_is_a_mode_ac_frame() {
        let frames = Parser::new().push(&HEARTBEAT_FRAME);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].bytes.len(), 2);
    }
}
