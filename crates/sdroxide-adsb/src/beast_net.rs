//! Beast over TCP, both ways.
//!
//! [`BeastServer`] is the output: it listens (30005 by convention) and sends
//! every frame this station's own demodulator accepts to every client that
//! connects — `readsb` with a `beast_in` net-connector, `tar1090`'s feeders,
//! `mlat-client`'s Beast input, a second SDR Oxide.
//!
//! [`BeastClient`] is the input: it connects to somebody else's Beast output
//! and hands the frames over, reconnecting when the far end goes away.
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

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
                .spawn(move || serve(listener, rx, clients, stop))?
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
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
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
) {
    let mut peers: Vec<Peer> = Vec::new();
    let mut scratch = [0u8; 512];
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

/// The Beast input: a thread that keeps a connection to `addr` open and hands
/// over whatever frames arrive.
pub struct BeastClient {
    addr: String,
    state: Arc<Mutex<FeedState>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl BeastClient {
    /// Start connecting to `addr` ("192.168.178.127:30005"). Frames go to
    /// `tx` in batches, one per read; a full channel drops the batch.
    pub fn start(addr: &str, tx: Sender<Vec<BeastFrame>>) -> BeastClient {
        let state = Arc::new(Mutex::new(FeedState::Connecting));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let (addr, state, stop) = (addr.to_string(), state.clone(), stop.clone());
            std::thread::Builder::new()
                .name("sdroxide-beast-in".into())
                .spawn(move || subscribe(&addr, &tx, &state, &stop))
                .ok()
        };
        BeastClient { addr: addr.to_string(), state, stop, worker }
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn state(&self) -> FeedState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for BeastClient {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
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

fn subscribe(
    addr: &str,
    tx: &Sender<Vec<BeastFrame>>,
    state: &Mutex<FeedState>,
    stop: &AtomicBool,
) {
    let mut retry = RETRY_FIRST;
    let mut buf = vec![0u8; 16 * 1024];
    while !stop.load(Ordering::Relaxed) {
        set(state, FeedState::Connecting);
        let why = match connect(addr) {
            Ok(mut sock) => {
                let _ = sock.set_read_timeout(Some(Duration::from_millis(250)));
                set(state, FeedState::Connected);
                info!(addr, "Beast input connected");
                retry = RETRY_FIRST;
                let mut parser = Parser::new();
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    match sock.read(&mut buf) {
                        Ok(0) => break "closed by the far end".to_string(),
                        Ok(n) => {
                            let frames = parser.push(&buf[..n]);
                            // Full drops the batch; disconnected means the
                            // decoder is gone, and so is the reason to read.
                            if !frames.is_empty()
                                && let Err(TrySendError::Disconnected(_)) = tx.try_send(frames)
                            {
                                return;
                            }
                        }
                        Err(e)
                            if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                        Err(e) => break e.to_string(),
                    }
                }
            }
            Err(e) => e.to_string(),
        };
        warn!(addr, %why, "Beast input down; retrying in {retry:?}");
        set(state, FeedState::Down(why));
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
}
