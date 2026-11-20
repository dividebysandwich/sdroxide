//! The hub thread: owns the listener and the client registry, fans realtime
//! frames out to subscribers, and applies every protocol policy decision
//! (subscriptions, transmit-key arbitration, status broadcast). Client threads
//! are pure transport, so all the rules live in one place.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded, unbounded};
use sdroxide_types::{DeviceCaps, TciServerConfig, TxTelemetry};
use tracing::{debug, info, warn};
use tungstenite::Bytes;

use super::client::{self, Out, Up};
use super::text::{self, ClientReq};
use super::{AUDIO_RATE_HZ, AudioBlock, Ctl, IqBlock, RX, ServerRequest, Shared, Subs};
use crate::protocol as p;

/// How often the hub wakes to accept, fan out and re-check stalled clients.
/// Short enough that control replies feel immediate and the TX chrono keeps up
/// with the engine's 10 ms blocks.
const POLL: Duration = Duration::from_millis(5);

/// Realtime frames a client may have queued before we consider it stalled.
const DATA_LANE: usize = 64;

/// Most complex IQ samples in one packet when the hub coalesces the engine's
/// blocks: 16 ms at 2 MS/s, 256 KiB on the wire.
const MAX_IQ_PER_PACKET: usize = 32_768;

/// How long a client's data lane may stay full before we conclude the peer is
/// gone (suspended laptop, dead network) and reclaim its slot. Without this a
/// vanished client would hold a slot until TCP finally timed out.
const STALL_LIMIT: Duration = Duration::from_secs(5);

pub(crate) struct HubParams {
    pub listener: TcpListener,
    pub cfg: TciServerConfig,
    pub device_name: String,
    pub caps: DeviceCaps,
    pub state: super::TciStateSnapshot,
    pub ctl_rx: Receiver<Ctl>,
    pub iq_rx: Receiver<IqBlock>,
    pub aud_rx: Receiver<AudioBlock>,
    pub req_tx: Sender<ServerRequest>,
    pub tx_prod: rtrb::Producer<f32>,
    pub shared: Arc<Shared>,
}

/// One connected client.
struct Slot {
    text_tx: Sender<String>,
    data_tx: Sender<Bytes>,
    subs: Arc<Subs>,
    /// Rate this client last asked for; the newest request across all clients
    /// wins, since TCI's `iq_samplerate` is a global setting.
    iq_rate: u32,
    handle: Option<JoinHandle<()>>,
    /// When this client's data lane first went full and stayed that way.
    stalled_since: Option<Instant>,
}

/// For a command we refuse, the line that states what is actually true instead
/// of what the client asked for. `None` means a plain echo is honest.
fn truth_for(cmd: &str) -> Option<String> {
    match cmd {
        // Disabling receiver 0 would silence the operator's own radio.
        "rx_enable" => Some(p::rx_enable(RX, true)),
        // Our audio streams are 48 kHz whatever was requested.
        "audio_samplerate" => Some(p::audio_samplerate(AUDIO_RATE_HZ)),
        _ => None,
    }
}

pub(crate) fn spawn(params: HubParams) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("sdroxide-tciserv".into())
        .spawn(move || Hub::new(params).run())
        .expect("spawn TCI server hub")
}

struct Hub {
    listener: TcpListener,
    cfg: TciServerConfig,
    device_name: String,
    ctl_rx: Receiver<Ctl>,
    iq_rx: Receiver<IqBlock>,
    aud_rx: Receiver<AudioBlock>,
    req_tx: Sender<ServerRequest>,
    tx_prod: rtrb::Producer<f32>,
    shared: Arc<Shared>,
    /// Shared with the client threads so a newly-accepted client can build its
    /// connect burst from live state without a round trip through the hub.
    state: Arc<RwLock<super::TciStateSnapshot>>,
    /// What we last broadcast, for the change diff.
    sent: super::TciStateSnapshot,
    up_tx: Sender<Up>,
    up_rx: Receiver<Up>,
    clients: HashMap<u64, Slot>,
    next_id: u64,
    /// The one client allowed to feed TX audio, if any.
    tx_owner: Option<u64>,
}

impl Hub {
    fn new(pr: HubParams) -> Hub {
        let (up_tx, up_rx) = unbounded();
        let mut state = pr.state;
        state.can_tx = pr.caps.is_transmit_capable();
        Hub {
            listener: pr.listener,
            cfg: pr.cfg,
            device_name: pr.device_name,
            ctl_rx: pr.ctl_rx,
            iq_rx: pr.iq_rx,
            aud_rx: pr.aud_rx,
            req_tx: pr.req_tx,
            tx_prod: pr.tx_prod,
            shared: pr.shared,
            sent: state.clone(),
            state: Arc::new(RwLock::new(state)),
            up_tx,
            up_rx,
            clients: HashMap::new(),
            next_id: 1,
            tx_owner: None,
        }
    }

    fn run(mut self) {
        info!(addr = ?self.listener.local_addr().ok(), "TCI server listening");
        loop {
            self.accept_new();
            if self.drain_ctl() {
                break;
            }
            self.drain_streams();
            self.drain_up();
            self.reap_stalled();
            std::thread::sleep(POLL);
        }
        // Close every client cleanly so peers see a WebSocket close rather than
        // a reset, then wait for the threads so the port is free on return.
        for (_, slot) in self.clients.drain() {
            drop(slot.text_tx);
            drop(slot.data_tx);
            if let Some(h) = slot.handle {
                let _ = h.join();
            }
        }
        info!("TCI server stopped");
    }

    // --- Accept ---------------------------------------------------------

    fn accept_new(&mut self) {
        loop {
            let stream = match self.listener.accept() {
                Ok((s, _peer)) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
                Err(e) => {
                    warn!("TCI server accept: {e}");
                    return;
                }
            };
            // The client thread is written for a blocking socket with a read
            // timeout. On macOS (and the other BSDs) an accepted socket
            // inherits the listener's non-blocking flag, Linux's does not:
            // left so, a write that meets a full send buffer returns EAGAIN,
            // which the thread takes for a dead peer and hangs up, and the
            // read timeout never paces its loop.
            if let Err(e) = stream.set_nonblocking(false) {
                warn!("TCI server: could not make a client socket blocking: {e}");
                continue;
            }
            if self.clients.len() >= self.cfg.max_clients {
                warn!(max = self.cfg.max_clients, "TCI server at client limit; refusing");
                continue; // dropping the stream closes it
            }
            let id = self.next_id;
            self.next_id += 1;

            let (text_tx, text_rx) = unbounded::<String>();
            let (data_tx, data_rx) = bounded::<Bytes>(DATA_LANE);
            let subs = Arc::new(Subs::default());

            // Queue the connect burst before the thread starts, so it goes out
            // as the very first thing the client sees.
            for line in text::connect_burst(&self.device_name, &self.state.read().unwrap()) {
                let _ = text_tx.send(line);
            }

            let handle = client::spawn(client::ClientParams {
                id,
                stream,
                out: Out { text_rx, data_rx },
                up_tx: self.up_tx.clone(),
            });
            let slot = Slot {
                text_tx,
                data_tx,
                subs,
                iq_rate: 0,
                handle: Some(handle),
                stalled_since: None,
            };
            self.clients.insert(id, slot);
            self.publish_clients();
            info!(id, clients = self.clients.len(), "TCI client connected");
        }
    }

    // --- Control from the engine ----------------------------------------

    /// Returns true when the server should stop — on an explicit `Stop`, or
    /// when the controller was dropped without one (the engine went away).
    fn drain_ctl(&mut self) -> bool {
        loop {
            match self.ctl_rx.try_recv() {
                Ok(Ctl::Stop) | Err(TryRecvError::Disconnected) => return true,
                Ok(Ctl::Config(cfg)) => self.cfg = cfg,
                Ok(Ctl::State(snap)) => self.on_state(*snap),
                Ok(Ctl::Telemetry(t)) => self.on_telemetry(t),
                Ok(Ctl::Chrono(frames)) => self.send_chrono(frames),
                Ok(Ctl::Unkey) => self.revoke_key(),
                Err(TryRecvError::Empty) => return false,
            }
        }
    }

    fn on_state(&mut self, snap: super::TciStateSnapshot) {
        let lines = text::diff_lines(Some(&self.sent), &snap);
        self.sent = snap.clone();
        *self.state.write().unwrap() = snap;
        if lines.is_empty() {
            return;
        }
        for line in lines {
            self.broadcast(&line);
        }
    }

    fn on_telemetry(&mut self, t: TxTelemetry) {
        let line = p::tx_sensors(RX, 0.0, t.fwd_w.unwrap_or(0.0), 0.0, t.swr.unwrap_or(1.0));
        for slot in self.clients.values() {
            if slot.subs.tx_sensors.load(Ordering::Relaxed) {
                let _ = slot.text_tx.send(line.clone());
            }
        }
    }

    /// Ask the keying client for more TX audio. Chronos are the pacing signal,
    /// so they go only to the client that owns the key.
    fn send_chrono(&mut self, frames: u32) {
        let Some(owner) = self.tx_owner else { return };
        let Some(slot) = self.clients.get(&owner) else { return };
        let pkt = p::build_chrono(AUDIO_RATE_HZ, RX, frames);
        // Chronos ride the realtime lane: a stale one is worthless, and the
        // engine issues a fresh one every block anyway.
        let _ = slot.data_tx.try_send(Bytes::from(pkt));
    }

    // --- Realtime fan-out ------------------------------------------------

    fn drain_streams(&mut self) {
        // Everything the engine handed over since the last wake goes out as
        // one packet per rate, up to MAX_IQ_PER_PACKET samples. The engine
        // hands IQ over in small blocks, about 1,800 a second at 2 MS/s, and a
        // client thread writes 32 frames per 20 ms tick: one frame per block
        // would cap a client near 1,600 a second. One packet per 5 ms wake is
        // ~200 a second at any rate.
        let mut pending: Option<(u32, Vec<f32>)> = None;
        while let Ok(block) = self.iq_rx.try_recv() {
            if block.rate == 0 {
                continue;
            }
            match pending.as_mut() {
                Some((rate, buf))
                    if *rate == block.rate
                        && buf.len() + block.data.len() <= 2 * MAX_IQ_PER_PACKET =>
                {
                    buf.extend_from_slice(&block.data);
                }
                _ => {
                    if let Some((rate, buf)) = pending.take() {
                        let pkt = Bytes::from(p::build_iq(rate, RX, &buf));
                        self.fan_out(pkt, |s| s.iq_on.load(Ordering::Relaxed));
                    }
                    pending = Some((block.rate, block.data));
                }
            }
        }
        if let Some((rate, buf)) = pending {
            let pkt = Bytes::from(p::build_iq(rate, RX, &buf));
            self.fan_out(pkt, |s| s.iq_on.load(Ordering::Relaxed));
        }
        while let Ok(AudioBlock(mono)) = self.aud_rx.try_recv() {
            let pkt = Bytes::from(p::build_rx_audio(AUDIO_RATE_HZ, RX, &mono));
            self.fan_out(pkt, |s| s.audio_on.load(Ordering::Relaxed));
        }
    }

    /// Push one already-built packet to every subscriber. The payload is built
    /// once and the `Bytes` refcount cloned per client, so N subscribers cost
    /// one encode — this is the hot path for a 192 kHz IQ stream.
    fn fan_out(&mut self, pkt: Bytes, wants: impl Fn(&Subs) -> bool) {
        let now = Instant::now();
        for slot in self.clients.values_mut() {
            if !wants(&slot.subs) {
                continue;
            }
            match slot.data_tx.try_send(pkt.clone()) {
                Ok(()) => slot.stalled_since = None,
                Err(TrySendError::Full(_)) => {
                    // Realtime data is worth less than latency: drop it and
                    // note how long this client has been unable to keep up.
                    let n = slot.subs.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_power_of_two() {
                        debug!(dropped = n, "TCI client behind; dropping stream frames");
                    }
                    slot.stalled_since.get_or_insert(now);
                }
                Err(TrySendError::Disconnected(_)) => {}
            }
        }
    }

    /// Close clients whose data lane has been full continuously for too long —
    /// a peer that stopped reading but whose socket hasn't failed yet.
    fn reap_stalled(&mut self) {
        let dead: Vec<u64> = self
            .clients
            .iter()
            .filter(|(_, s)| s.stalled_since.is_some_and(|t| t.elapsed() > STALL_LIMIT))
            .map(|(id, _)| *id)
            .collect();
        for id in dead {
            warn!(id, "TCI client not reading for {STALL_LIMIT:?}; disconnecting");
            self.remove(id);
        }
    }

    // --- Client traffic ---------------------------------------------------

    fn drain_up(&mut self) {
        while let Ok(msg) = self.up_rx.try_recv() {
            match msg {
                Up::Text(id, text) => self.on_client_text(id, &text),
                Up::TxAudio(id, mono) => self.on_tx_audio(id, &mono),
                Up::Gone(id) => {
                    info!(id, "TCI client disconnected");
                    self.remove(id);
                }
            }
        }
    }

    fn on_client_text(&mut self, id: u64, text: &str) {
        // Every third-party client implements a slightly different subset, and a
        // handshake it doesn't like it simply disconnects from — so the whole
        // conversation is logged at debug for interop diagnosis
        // (`RUST_LOG=sdroxide_tci=debug`).
        debug!(id, text, "TCI server: client ->");
        for (cmd, args) in p::parse_status(text) {
            // TCI clients treat the echo of a command as its acknowledgement
            // and stall waiting for one — WSJT-X gives up on the connection
            // entirely. So every command we accept is echoed straight back,
            // *even when it changed nothing*, which is what the reference
            // ExpertSDR3 server does. If the engine then clamps or refuses the
            // value, the state diff corrects it a tick later.
            let echo = if args.is_empty() { format!("{cmd};") } else { format!("{cmd}:{args};") };
            let snap = self.state.read().unwrap().clone();
            let Some(req) = text::translate(&cmd, &args, &snap) else {
                debug!(id, cmd, args, "TCI server: unhandled command");
                continue;
            };
            match req {
                ClientReq::Cmd(c) => {
                    // Radio state is shared, so its echo goes to everyone.
                    self.broadcast(&echo);
                    let _ = self.req_tx.send(ServerRequest::Cmd(c));
                }
                ClientReq::Key(on) => self.on_key(id, on),
                ClientReq::Iq(on) => {
                    if let Some(slot) = self.clients.get(&id) {
                        slot.subs.iq_on.store(on, Ordering::Relaxed);
                    }
                    self.recompute_iq();
                    // Subscriptions are per-client: only the asker is told.
                    self.send_to(id, echo);
                }
                ClientReq::IqRate(hz) => {
                    if let Some(slot) = self.clients.get_mut(&id) {
                        slot.iq_rate = hz;
                    }
                    self.recompute_iq();
                    // Acknowledge now; the engine follows with the rate it
                    // could actually achieve, as a state broadcast.
                    self.send_to(id, echo);
                }
                ClientReq::Audio(on) => {
                    if let Some(slot) = self.clients.get(&id) {
                        slot.subs.audio_on.store(on, Ordering::Relaxed);
                    }
                    let any =
                        self.clients.values().any(|s| s.subs.audio_on.load(Ordering::Relaxed));
                    self.shared.audio_on.store(any, Ordering::Relaxed);
                    self.send_to(id, echo);
                }
                ClientReq::TxSensors(on) => {
                    if let Some(slot) = self.clients.get(&id) {
                        slot.subs.tx_sensors.store(on, Ordering::Relaxed);
                    }
                    self.send_to(id, echo);
                }
                // Understood but not obeyed. Still acknowledge it, then correct
                // the client where what it asked for isn't what is true.
                ClientReq::Ignored => match truth_for(&cmd) {
                    Some(line) => self.send_to(id, line),
                    None => self.send_to(id, echo),
                },
            }
        }
    }

    /// Grant or release the transmit key. Exactly one client may hold it, which
    /// is what keeps the TX ring single-producer.
    fn on_key(&mut self, id: u64, on: bool) {
        if on {
            if !self.cfg.allow_tx {
                self.send_to(id, p::trx(RX, false, false));
                return;
            }
            match self.tx_owner {
                Some(owner) if owner != id => {
                    // Somebody else is already transmitting.
                    self.send_to(id, p::trx(RX, false, false));
                    return;
                }
                Some(_) => return, // already ours; nothing to do
                None => {}
            }
            self.tx_owner = Some(id);
            self.shared.tx_keyed.store(true, Ordering::Relaxed);
            let _ = self.req_tx.send(ServerRequest::Key(true));
        } else {
            if self.tx_owner != Some(id) {
                return;
            }
            self.release_key();
        }
    }

    /// The keying client went away: drop the key and ask the engine to unkey.
    fn release_key(&mut self) {
        if let Some(owner) = self.tx_owner.take() {
            self.send_to(owner, p::trx(RX, false, false));
        }
        self.shared.tx_keyed.store(false, Ordering::Relaxed);
        let _ = self.req_tx.send(ServerRequest::Key(false));
    }

    /// The engine took the transmitter back (refused, denied by a safety rail,
    /// starved of audio, or the operator keyed up). Tell the client, but don't
    /// ask the engine to unkey — it is the one that decided.
    fn revoke_key(&mut self) {
        let Some(owner) = self.tx_owner.take() else { return };
        self.shared.tx_keyed.store(false, Ordering::Relaxed);
        self.send_to(owner, p::trx(RX, false, false));
    }

    fn on_tx_audio(&mut self, id: u64, mono: &[f32]) {
        if self.tx_owner != Some(id) {
            return; // not the keying client; ignore rather than mix
        }
        for &s in mono {
            // A full ring means the client is racing ahead of real time; drop
            // the excess rather than block the hub.
            if self.tx_prod.push(s).is_err() {
                break;
            }
        }
    }

    /// The newest rate any IQ-subscribed client asked for, or 0 when nobody
    /// wants IQ. TCI's `iq_samplerate` is global, so there is one answer.
    fn recompute_iq(&mut self) {
        let rate = self
            .clients
            .values()
            .filter(|s| s.subs.iq_on.load(Ordering::Relaxed))
            .map(|s| if s.iq_rate == 0 { 192_000 } else { s.iq_rate })
            .max()
            .unwrap_or(0);
        self.shared.iq_rate.store(rate, Ordering::Relaxed);
    }

    // --- Plumbing ---------------------------------------------------------

    fn send_to(&self, id: u64, line: String) {
        debug!(id, line, "TCI server: -> client");
        if let Some(slot) = self.clients.get(&id) {
            let _ = slot.text_tx.send(line);
        }
    }

    fn broadcast(&self, line: &str) {
        debug!(line, "TCI server: -> all clients");
        for slot in self.clients.values() {
            let _ = slot.text_tx.send(line.to_string());
        }
    }

    fn remove(&mut self, id: u64) {
        let Some(slot) = self.clients.remove(&id) else { return };
        // Dropping both senders ends the client thread's loop.
        drop(slot.text_tx);
        drop(slot.data_tx);
        if let Some(h) = slot.handle {
            let _ = h.join();
        }
        if self.tx_owner == Some(id) {
            // A client that vanishes mid-over must not leave us transmitting.
            self.release_key();
        }
        self.recompute_iq();
        let any = self.clients.values().any(|s| s.subs.audio_on.load(Ordering::Relaxed));
        self.shared.audio_on.store(any, Ordering::Relaxed);
        self.publish_clients();
    }

    fn publish_clients(&self) {
        let n = self.clients.len();
        self.shared.clients.store(n, Ordering::Relaxed);
        let _ = self.req_tx.send(ServerRequest::Clients(n));
    }
}
