//! Who is driving the modem, and how their commands and data reach it.
//!
//! A TNC is driven over two TCP ports — commands on one (CR-terminated lines
//! both ways) and the link's payload on the next — and every Winlink, Pat,
//! VarAC and BPQ client speaks that. Here the same language has three
//! speakers: an external program on the TCP ports, the Winlink window, and the
//! operator's own panel. All three go through one [`HostMux`], so the modem has
//! exactly one host at a time and the in-process ones exercise the very path
//! Pat does.
//!
//! The one-host rule is enforced by [`TncHandle::claim`]: while a TCP client
//! holds the command port an in-process claim is refused with a reason, and
//! while an in-process client holds it a TCP connection is closed on accept.
//! Lines are carried, never interpreted: the modem behind the mux answers them.

use std::io::{self, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, unbounded};

/// Longest command line accepted.
pub const MAX_LINE: usize = 1024;
const DATA_CHUNK: usize = 65536;
const POLL: Duration = Duration::from_millis(50);

/// One thing a host did.
#[derive(Debug)]
pub enum Input {
    /// One command line, without the CR.
    Command(String),
    Data(Vec<u8>),
    /// A host attached to the command port.
    Connected,
    /// The host went away.
    Closed,
}

/// Who holds the modem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    /// An external program on the TCP ports, by peer address.
    Tcp(String),
    /// An in-process client, by the name it claimed with.
    Local(String),
}

impl Holder {
    pub fn describe(&self) -> String {
        match self {
            Holder::Tcp(a) => format!("TCP client {a}"),
            Holder::Local(w) => w.clone(),
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The modem-side ends of an in-process client, waiting for the mux to pick
/// them up.
struct LocalEnds {
    id: u64,
    input: Receiver<Input>,
    lines: Sender<String>,
    data: Sender<Vec<u8>>,
}

#[derive(Default)]
struct Slot {
    holder: Option<Holder>,
    /// Bumped on every claim, so a stale client's drop cannot release a newer
    /// one's hold.
    next_id: u64,
    pending: Option<LocalEnds>,
    /// Set while the modem exists; a claim against a modem that is gone (the
    /// mode was changed away) is refused rather than left hanging.
    alive: bool,
    /// The modem's name, for the reasons a claim is refused.
    label: &'static str,
}

/// The engine's (and the Winlink manager's) way in: cheap to clone, `Send`,
/// and outlives nothing — a claim made after the modem is gone is refused.
#[derive(Clone)]
pub struct TncHandle {
    slot: Arc<Mutex<Slot>>,
}

impl TncHandle {
    /// Take the modem for an in-process host. `who` is shown to anyone else who
    /// tries meanwhile.
    pub fn claim(&self, who: &str) -> Result<LocalClient, String> {
        let mut s = lock(&self.slot);
        if !s.alive {
            return Err(format!("the radio is no longer in {}", s.label));
        }
        if let Some(h) = &s.holder {
            return Err(format!("the {} modem is in use by {}", s.label, h.describe()));
        }
        s.next_id += 1;
        let id = s.next_id;
        let (to_tnc, input) = unbounded();
        let (lines, from_lines) = unbounded();
        let (data, from_data) = unbounded();
        to_tnc.send(Input::Connected).ok();
        s.pending = Some(LocalEnds { id, input, lines, data });
        s.holder = Some(Holder::Local(who.into()));
        Ok(LocalClient { id, to_tnc, lines: from_lines, data: from_data, slot: self.slot.clone() })
    }

    /// Who holds the modem now, if anyone.
    pub fn holder(&self) -> Option<Holder> {
        lock(&self.slot).holder.clone()
    }

    /// Whether two handles reach the same modem — so the engine can tell a
    /// fresh modem from the one a destroyed controller left behind.
    pub fn same_modem(&self, other: &TncHandle) -> bool {
        Arc::ptr_eq(&self.slot, &other.slot)
    }
}

/// An in-process host. Speaks the same command language as the TCP port;
/// dropping it is the same as closing the socket.
pub struct LocalClient {
    id: u64,
    to_tnc: Sender<Input>,
    lines: Receiver<String>,
    data: Receiver<Vec<u8>>,
    slot: Arc<Mutex<Slot>>,
}

impl LocalClient {
    /// Send one command line, without the CR.
    pub fn command(&self, line: &str) {
        self.to_tnc.send(Input::Command(line.trim().into())).ok();
    }

    pub fn send_data(&self, b: &[u8]) {
        if !b.is_empty() {
            self.to_tnc.send(Input::Data(b.to_vec())).ok();
        }
    }

    /// The next reply or asynchronous line from the modem, without the CR.
    /// `Err(true)` means the modem is gone.
    pub fn recv_line(&self, timeout: Duration) -> Result<String, bool> {
        match self.lines.recv_timeout(timeout) {
            Ok(l) => Ok(l),
            Err(RecvTimeoutError::Timeout) => Err(false),
            Err(RecvTimeoutError::Disconnected) => Err(true),
        }
    }

    pub fn try_recv_line(&self) -> Option<String> {
        self.lines.try_recv().ok()
    }

    pub fn try_recv_data(&self) -> Result<Vec<u8>, bool> {
        match self.data.try_recv() {
            Ok(d) => Ok(d),
            Err(TryRecvError::Empty) => Err(false),
            Err(TryRecvError::Disconnected) => Err(true),
        }
    }

    pub fn recv_data(&self, timeout: Duration) -> Result<Vec<u8>, bool> {
        match self.data.recv_timeout(timeout) {
            Ok(d) => Ok(d),
            Err(RecvTimeoutError::Timeout) => Err(false),
            Err(RecvTimeoutError::Disconnected) => Err(true),
        }
    }
}

impl Drop for LocalClient {
    fn drop(&mut self) {
        self.to_tnc.send(Input::Closed).ok();
        let mut s = lock(&self.slot);
        if s.pending.as_ref().is_some_and(|p| p.id == self.id) {
            s.pending = None;
        }
        if s.next_id == self.id && matches!(s.holder, Some(Holder::Local(_))) {
            s.holder = None;
        }
    }
}

/// A TCP writer on its own thread, so a stalled client cannot stall the audio
/// path that feeds it.
struct Writer {
    tx: Sender<Vec<u8>>,
}

impl Writer {
    fn new(mut s: TcpStream, name: &str) -> io::Result<Self> {
        let (tx, rx) = unbounded::<Vec<u8>>();
        thread::Builder::new().name(name.into()).spawn(move || {
            for b in rx {
                if s.write_all(&b).is_err() {
                    break;
                }
            }
            let _ = s.shutdown(Shutdown::Both);
        })?;
        Ok(Self { tx })
    }

    fn send(&self, b: &[u8]) {
        self.tx.send(b.to_vec()).ok();
    }
}

#[derive(Default)]
struct TcpClients {
    cmd: Option<(u64, Writer)>,
    data: Option<(u64, Writer)>,
    next: u64,
}

/// The command and data ports.
struct TcpHost {
    clients: Arc<Mutex<TcpClients>>,
    closed: Arc<AtomicBool>,
    addrs: [SocketAddr; 2],
}

impl TcpHost {
    fn bind(
        addr: &str,
        port: u16,
        input: Sender<Input>,
        slot: Arc<Mutex<Slot>>,
        hold: Arc<AtomicBool>,
    ) -> io::Result<Self> {
        let data_port = match port {
            0 => 0,
            p => p.checked_add(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "port must be < 65535")
            })?,
        };
        let cl = TcpListener::bind((addr, port))?;
        let dl = TcpListener::bind((addr, data_port))?;
        cl.set_nonblocking(true)?;
        dl.set_nonblocking(true)?;
        let clients = Arc::<Mutex<TcpClients>>::default();
        let closed = Arc::<AtomicBool>::default();
        let addrs = [cl.local_addr()?, dl.local_addr()?];
        for (l, is_cmd) in [(cl, true), (dl, false)] {
            let (clients, closed, input, slot, hold) =
                (clients.clone(), closed.clone(), input.clone(), slot.clone(), hold.clone());
            let name = if is_cmd { "tnc-cmd-accept" } else { "tnc-data-accept" };
            thread::Builder::new()
                .name(name.into())
                .spawn(move || accept_loop(l, is_cmd, clients, closed, input, slot, hold))?;
        }
        Ok(Self { clients, closed, addrs })
    }

    fn line(&self, s: &str) {
        if let Some((_, w)) = &lock(&self.clients).cmd {
            w.send(format!("{s}\r").as_bytes());
        }
    }

    fn data(&self, b: &[u8]) {
        if let Some((_, w)) = &lock(&self.clients).data {
            w.send(b);
        }
    }

    fn has_cmd(&self) -> bool {
        lock(&self.clients).cmd.is_some()
    }

    fn has_data(&self) -> bool {
        lock(&self.clients).data.is_some()
    }
}

impl Drop for TcpHost {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        let mut c = lock(&self.clients);
        c.cmd = None;
        c.data = None;
    }
}

fn accept_loop(
    l: TcpListener,
    is_cmd: bool,
    clients: Arc<Mutex<TcpClients>>,
    closed: Arc<AtomicBool>,
    input: Sender<Input>,
    slot: Arc<Mutex<Slot>>,
    hold: Arc<AtomicBool>,
) {
    while !closed.load(Ordering::SeqCst) {
        let (s, peer) = match l.accept() {
            Ok(x) => x,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(POLL);
                continue;
            }
            Err(_) => {
                thread::sleep(POLL);
                continue;
            }
        };
        // Accepted sockets inherit non-blocking on some platforms.
        if s.set_nonblocking(false).is_err() {
            continue;
        }
        let _ = s.set_nodelay(true);
        let _ = s.set_read_timeout(Some(POLL * 4));
        {
            let mut sl = lock(&slot);
            let taken_locally = matches!(sl.holder, Some(Holder::Local(_)));
            let mut c = lock(&clients);
            let busy = if is_cmd { c.cmd.is_some() } else { c.data.is_some() };
            if taken_locally || busy {
                let _ = s.shutdown(Shutdown::Both);
                continue;
            }
            let Ok(w) = s
                .try_clone()
                .and_then(|w| Writer::new(w, if is_cmd { "tnc-cmd-tx" } else { "tnc-data-tx" }))
            else {
                continue;
            };
            c.next += 1;
            let id = c.next;
            if is_cmd {
                c.cmd = Some((id, w));
                sl.holder = Some(Holder::Tcp(peer.to_string()));
                input.send(Input::Connected).ok();
            } else {
                c.data = Some((id, w));
            }
            let (clients, closed, input, slot, hold) =
                (clients.clone(), closed.clone(), input.clone(), slot.clone(), hold.clone());
            let name = if is_cmd { "tnc-cmd-rx" } else { "tnc-data-rx" };
            let spawned = thread::Builder::new().name(name.into()).spawn(move || {
                if is_cmd {
                    read_commands(s, &closed, &input);
                } else {
                    read_data(s, &closed, &input, &hold);
                }
                let mut sl = lock(&slot);
                let mut c = lock(&clients);
                let slot_ref = if is_cmd { &mut c.cmd } else { &mut c.data };
                if slot_ref.as_ref().is_some_and(|(i, _)| *i == id) {
                    *slot_ref = None;
                    if is_cmd {
                        if matches!(sl.holder, Some(Holder::Tcp(_))) {
                            sl.holder = None;
                        }
                        input.send(Input::Closed).ok();
                    }
                }
            });
            if spawned.is_err() {
                if is_cmd {
                    c.cmd = None;
                    sl.holder = None;
                } else {
                    c.data = None;
                }
            }
        }
    }
}

fn read_commands(s: TcpStream, closed: &AtomicBool, input: &Sender<Input>) {
    let mut r = BufReader::new(s);
    let mut buf = Vec::new();
    while !closed.load(Ordering::SeqCst) {
        let mut byte = [0u8; 1];
        match r.read(&mut byte) {
            Ok(0) => return,
            Ok(_) => match byte[0] {
                b'\r' | b'\n' => {
                    if buf.is_empty() {
                        continue;
                    }
                    let line = String::from_utf8_lossy(&buf).trim().to_string();
                    buf.clear();
                    if input.send(Input::Command(line)).is_err() {
                        return;
                    }
                }
                b => {
                    if buf.len() < MAX_LINE {
                        buf.push(b);
                    }
                }
            },
            // The read timeout only exists so `closed` is looked at.
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(_) => return,
        }
    }
}

fn read_data(mut s: TcpStream, closed: &AtomicBool, input: &Sender<Input>, hold: &AtomicBool) {
    let mut buf = vec![0u8; DATA_CHUNK];
    while !closed.load(Ordering::SeqCst) {
        // Back-pressure: while the modem's transmit buffer is full the reader
        // stops reading, and TCP's own window pushes back on the client.
        if hold.load(Ordering::SeqCst) {
            thread::sleep(POLL);
            continue;
        }
        match s.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => {
                if input.send(Input::Data(buf[..n].to_vec())).is_err() {
                    return;
                }
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(_) => return,
        }
    }
}

/// The modem's side: every host's input merged, and replies routed to whoever
/// holds it.
pub struct HostMux {
    slot: Arc<Mutex<Slot>>,
    tcp: Option<TcpHost>,
    tcp_input: (Sender<Input>, Receiver<Input>),
    hold: Arc<AtomicBool>,
    local: Option<LocalEnds>,
}

impl HostMux {
    /// A mux for the modem called `label` (shown in refusals).
    pub fn new(label: &'static str) -> Self {
        let slot = Arc::new(Mutex::new(Slot { alive: true, label, ..Slot::default() }));
        Self { slot, tcp: None, tcp_input: unbounded(), hold: Arc::default(), local: None }
    }

    pub fn handle(&self) -> TncHandle {
        TncHandle { slot: self.slot.clone() }
    }

    /// Open the TCP ports (`port` for commands, `port + 1` for data).
    pub fn listen_tcp(&mut self, addr: &str, port: u16) -> io::Result<()> {
        self.tcp = None;
        self.tcp = Some(TcpHost::bind(
            addr,
            port,
            self.tcp_input.0.clone(),
            self.slot.clone(),
            self.hold.clone(),
        )?);
        Ok(())
    }

    /// Close the TCP ports, dropping any client on them.
    pub fn close_tcp(&mut self) {
        if self.tcp.take().is_some() {
            let mut s = lock(&self.slot);
            if matches!(s.holder, Some(Holder::Tcp(_))) {
                s.holder = None;
                self.tcp_input.0.send(Input::Closed).ok();
            }
        }
    }

    pub fn tcp_addrs(&self) -> Option<[SocketAddr; 2]> {
        self.tcp.as_ref().map(|t| t.addrs)
    }

    pub fn holder(&self) -> Option<Holder> {
        lock(&self.slot).holder.clone()
    }

    /// The next thing any host did.
    pub fn try_recv(&mut self) -> Option<Input> {
        if let Some(p) = lock(&self.slot).pending.take() {
            self.local = Some(p);
        }
        if let Some(l) = &self.local {
            match l.input.try_recv() {
                Ok(i) => {
                    if matches!(i, Input::Closed) {
                        self.local = None;
                    }
                    return Some(i);
                }
                Err(TryRecvError::Disconnected) => {
                    self.local = None;
                    return Some(Input::Closed);
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        self.tcp_input.1.try_recv().ok()
    }

    pub fn line(&self, s: &str) {
        if let Some(l) = &self.local {
            l.lines.send(s.into()).ok();
        } else if let Some(t) = &self.tcp {
            t.line(s);
        }
    }

    pub fn data(&self, b: &[u8]) {
        if let Some(l) = &self.local {
            l.data.send(b.to_vec()).ok();
        } else if let Some(t) = &self.tcp {
            t.data(b);
        }
    }

    pub fn hold_data(&self, on: bool) {
        self.hold.store(on, Ordering::SeqCst);
    }

    pub fn has_cmd_client(&self) -> bool {
        self.local.is_some() || self.tcp.as_ref().is_some_and(TcpHost::has_cmd)
    }

    pub fn has_data_client(&self) -> bool {
        self.local.is_some() || self.tcp.as_ref().is_some_and(TcpHost::has_data)
    }
}

impl Drop for HostMux {
    fn drop(&mut self) {
        let mut s = lock(&self.slot);
        s.alive = false;
        s.holder = None;
        s.pending = None;
    }
}
