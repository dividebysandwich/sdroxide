//! FreeDV Reporter (<https://qso.freedv.org/>) client: one blocking thread
//! holding a Socket.IO session that both *reports* our station and *receives*
//! everyone else's, the latter published through the ordinary spot-feed channel
//! as [`SpotKind::FreeDv`].
//!
//! Modelled on the [`crate::cluster`] thread — a blocking socket with a short
//! read timeout, a `crossbeam` control channel, and a `Drop` that shuts the
//! thread down — with [`crate::poll`]'s `recv_timeout`-as-sleep idiom for the
//! reconnect backoff and its "only emit a status line when it changes" dedup.
//!
//! Two rules from the protocol shape the whole design:
//!
//! * **Nothing may be emitted before the server's `connection_successful`.**
//!   Every setter therefore *stores* its value unconditionally and *sends* only
//!   when fully connected, which also makes reconnects replay state for free.
//! * **We are only visible while the radio is in RADE.** The engine pushes
//!   `Visible(mode.is_rade())` every tick and this thread turns transitions into
//!   `show_self` / `hide_self`, so no state tracking is needed in the engine.
//!
//! The engine pushes frequency, TX and visibility unconditionally on every poll
//! tick; the dedup guards in [`Reporter::apply`] make that free and keep the
//! reporter self-healing across reconnects and config rebuilds.
//!
//! Those same dedup guards are why a *listening* station needs the keepalive in
//! [`Reporter::send_keepalive`]: with nothing to say, an operator who neither
//! transmits nor retunes emits nothing at all, and the server's record of us
//! ages until it looks like a stale session. See that function for what the
//! server does with it.

use std::collections::{BTreeMap, HashMap};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use serde_json::{Value, json};
use tracing::{debug, info, warn};
use tungstenite::{Message, WebSocket};

use sdroxide_types::{FreeDvReporterConfig, Spot, SpotKind};

use crate::event::{EventTx, FeedTx, NetEvent};
use crate::socketio::{self, Frame};

/// The mode string FreeDV Reporter uses for RADE V1. It is the only waveform
/// sdroxide speaks, so it is a constant rather than derived from the radio.
const MODE_STRING: &str = "RADEV1";

/// The reporter protocol revision this client implements.
const PROTOCOL_VERSION: i64 = 2;

/// Read timeout on the socket. Bounds both shutdown latency and how long the
/// control channel can go unserviced.
const READ_TIMEOUT: Duration = Duration::from_millis(150);

/// Timeout for the TCP connect and the WebSocket handshake. Deliberately far
/// longer than [`READ_TIMEOUT`]: a handshake interrupted by a short timeout
/// aborts the connection rather than resuming.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Reconnect backoff bounds.
const BACKOFF_MIN: Duration = Duration::from_secs(2);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Fastest we will republish the roster as a spot batch.
const PUBLISH_MIN_INTERVAL: Duration = Duration::from_secs(1);

/// Republish at least this often even when nothing changed, so `when_utc` stays
/// fresh and live stations are never pruned by the manager's age filter.
const PUBLISH_MAX_INTERVAL: Duration = Duration::from_secs(60);

/// How long our reported state may go unrefreshed before the keepalive
/// re-asserts it. Any real report postpones it, so this is a floor on the
/// silence we tolerate, not a fixed cadence: we never emit more than one
/// keepalive per minute, and none at all while we have genuine traffic.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(60);

/// Guard against a malicious or buggy server nesting `bulk_update`s.
const MAX_BULK_DEPTH: u32 = 1;

/// UTC seconds now (native-only wall clock).
fn now_utc() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The transport. TLS would replace these two aliases and [`Reporter::open`];
/// nothing else in this module cares.
type Sock = TcpStream;
type Ws = WebSocket<Sock>;

// ───────────────────────────── handle ─────────────────────────────

/// Messages from the engine to the reporter thread.
enum Ctrl {
    Freq(u64),
    Tx(bool),
    /// `true` = `show_self`, `false` = `hide_self`. The engine pushes
    /// `mode.is_rade()`.
    Visible(bool),
    Message(String),
    RxReport {
        call: String,
        snr: i32,
    },
    /// A station we are receiving but have not identified yet: an `rx_report`
    /// with an empty callsign, which the server relays so the far end can see
    /// that somebody is hearing it. See [`Reporter::emit_rx_presence`].
    RxPresence(i32),
    Shutdown,
}

/// A live FreeDV Reporter session. Dropping it stops the thread.
pub struct ReporterHandle {
    ctrl: Sender<Ctrl>,
    join: Option<JoinHandle<()>>,
}

impl Drop for ReporterHandle {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl ReporterHandle {
    /// Spawn the session thread. `call`/`grid` are the *effective* operator
    /// identity (see `NetworkConfig::reporter_call`); when either is empty the
    /// session connects in the server's read-only "view" role and this station
    /// never appears publicly.
    pub fn connect(
        cfg: FreeDvReporterConfig,
        call: String,
        grid: String,
        feed: FeedTx,
        events: EventTx,
    ) -> ReporterHandle {
        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let join = std::thread::Builder::new()
            // Linux caps thread names at 15 bytes.
            .name("sdroxide-fdvrep".into())
            .spawn(move || {
                let mut r = Reporter::new(cfg, call, grid, feed, events, ctrl_rx);
                r.run_until(None);
            })
            .ok();
        ReporterHandle { ctrl: ctrl_tx, join }
    }

    pub fn set_freq(&self, hz: u64) {
        let _ = self.ctrl.send(Ctrl::Freq(hz));
    }
    pub fn set_tx(&self, on: bool) {
        let _ = self.ctrl.send(Ctrl::Tx(on));
    }
    pub fn set_visible(&self, visible: bool) {
        let _ = self.ctrl.send(Ctrl::Visible(visible));
    }
    pub fn set_message(&self, message: String) {
        let _ = self.ctrl.send(Ctrl::Message(message));
    }
    pub fn rx_report(&self, call: String, snr: i32) {
        let _ = self.ctrl.send(Ctrl::RxReport { call, snr });
    }
    /// Report that we are receiving *something* from an unidentified station.
    pub fn rx_presence(&self, snr: i32) {
        let _ = self.ctrl.send(Ctrl::RxPresence(snr));
    }
}

// ───────────────────────────── roster ─────────────────────────────

/// One station as the server describes it. Assembled incrementally: a station
/// appears on `new_connection` and is then updated by `freq_change`,
/// `tx_report`, `rx_report` and `message_update` naming its `sid`.
#[derive(Debug, Clone, Default, PartialEq)]
struct Station {
    call: String,
    grid: String,
    version: String,
    freq_hz: u64,
    tx: bool,
    mode: String,
    message: String,
    rx_only: bool,
    /// The most recent `rx_report` naming this station: who heard it, and how
    /// well. Folded in here rather than spotted separately, which would
    /// double-count the station on the overlay.
    heard_by: Option<(String, i16)>,
}

/// The server's view of who is on the air, keyed by socket.io session id.
///
/// Split out from the session so the whole event-handling surface is testable
/// without a socket.
#[derive(Debug, Default)]
struct Roster {
    by_sid: HashMap<String, Station>,
}

impl Roster {
    /// Apply one server event. Returns true when the spot set may have changed.
    ///
    /// Every field is type-checked and the whole event dropped on a mismatch,
    /// so a server-side change can degrade this client but never panic it.
    fn on_event(&mut self, name: &str, args: &Value) -> bool {
        match name {
            "new_connection" => {
                let Some(sid) = str_of(args, "sid") else { return false };
                let s = self.by_sid.entry(sid).or_default();
                // A reconnecting station keeps whatever frequency we already
                // knew; the server re-sends `freq_change` right afterwards.
                s.call = str_of(args, "callsign").unwrap_or_default().to_uppercase();
                s.grid = str_of(args, "grid_square").unwrap_or_default();
                s.version = str_of(args, "version").unwrap_or_default();
                s.rx_only = args.get("rx_only").and_then(Value::as_bool).unwrap_or(false);
                true
            }
            "remove_connection" => {
                let Some(sid) = str_of(args, "sid") else { return false };
                self.by_sid.remove(&sid).is_some()
            }
            "freq_change" => {
                let (Some(sid), Some(freq)) =
                    (str_of(args, "sid"), args.get("freq").and_then(Value::as_u64))
                else {
                    return false;
                };
                let s = self.by_sid.entry(sid).or_default();
                Self::fill_identity(s, args);
                s.freq_hz = freq;
                true
            }
            "tx_report" => {
                let (Some(sid), Some(tx)) =
                    (str_of(args, "sid"), args.get("transmitting").and_then(Value::as_bool))
                else {
                    return false;
                };
                let s = self.by_sid.entry(sid).or_default();
                Self::fill_identity(s, args);
                s.tx = tx;
                if let Some(m) = str_of(args, "mode") {
                    s.mode = m;
                }
                true
            }
            "rx_report" => {
                // Unlike the others this is keyed by the *heard* callsign, not
                // by sid: `sid` here is the receiving station.
                let (Some(heard), Some(rx_call)) =
                    (str_of(args, "callsign"), str_of(args, "receiver_callsign"))
                else {
                    return false;
                };
                // The server sends SNR as an integer or a float.
                let Some(snr) = args.get("snr").and_then(Value::as_f64) else { return false };
                // An empty callsign is a routine part of this event, not a
                // sighting: RADE stations report one to say they are hearing
                // *something*, and the server sends one to clear a station's RX
                // data when it retunes. Neither attributes a decode to anybody.
                let heard = heard.to_uppercase();
                if heard.is_empty() {
                    return false;
                }
                let mut changed = false;
                for s in self.by_sid.values_mut() {
                    if s.call == heard {
                        s.heard_by = Some((rx_call.to_uppercase(), snr.round() as i16));
                        changed = true;
                    }
                }
                changed
            }
            "message_update" => {
                let Some(sid) = str_of(args, "sid") else { return false };
                let Some(msg) = str_of(args, "message") else { return false };
                let s = self.by_sid.entry(sid).or_default();
                s.message = msg;
                true
            }
            _ => false,
        }
    }

    /// Most events repeat the station's identity; take it when present so a
    /// station first seen through `freq_change` still gets a callsign.
    fn fill_identity(s: &mut Station, args: &Value) {
        if let Some(c) = str_of(args, "callsign") {
            s.call = c.to_uppercase();
        }
        if let Some(g) = str_of(args, "grid_square") {
            s.grid = g;
        }
    }

    /// Convert the roster to a spot batch, skipping our own session and any
    /// station we cannot place on the band.
    fn spots(&self, now: i64, my_sid: Option<&str>) -> Vec<Spot> {
        self.by_sid
            .iter()
            .filter(|(sid, _)| Some(sid.as_str()) != my_sid)
            .filter(|(_, s)| !s.call.is_empty() && s.freq_hz > 0)
            .map(|(_, s)| {
                let freq_hz = s.freq_hz as f64;
                let (spotter, snr_db) = match &s.heard_by {
                    Some((rx_call, snr)) => (rx_call.clone(), Some(*snr)),
                    // Nobody has reported hearing them; the reporter itself is
                    // the source of the sighting.
                    None => ("qso.freedv.org".to_string(), None),
                };
                Spot {
                    id: Spot::make_id(SpotKind::FreeDv, &s.call, freq_hz),
                    kind: SpotKind::FreeDv,
                    freq_hz,
                    call: s.call.clone(),
                    spotter,
                    mode: if s.mode.is_empty() { MODE_STRING.to_string() } else { s.mode.clone() },
                    comment: comment_for(s),
                    reference: None,
                    grid: (!s.grid.is_empty()).then(|| s.grid.clone()),
                    // Filled from `grid` by SpotManager::snapshot.
                    loc: None,
                    // These stations are connected *now*; stamping the batch
                    // time is what keeps them alive against the manager's age
                    // pruning. Removal is driven by `remove_connection`.
                    when_utc: now,
                    snr_db,
                }
            })
            .collect()
    }
}

/// The spot comment line: the operator's own status message first, then the
/// facts the overlay has no column for.
fn comment_for(s: &Station) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !s.message.is_empty() {
        parts.push(s.message.clone());
    }
    if s.tx {
        parts.push("TX".to_string());
    }
    if s.rx_only {
        parts.push("RX only".to_string());
    }
    if !s.version.is_empty() {
        parts.push(s.version.clone());
    }
    parts.join(" · ")
}

/// A JSON string field, or `None` if absent or not a string.
fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

// ───────────────────────────── session ─────────────────────────────

struct Reporter {
    cfg: FreeDvReporterConfig,
    call: String,
    grid: String,
    feed: FeedTx,
    events: EventTx,
    ctrl: Receiver<Ctrl>,

    // ── Desired state, always current whether or not we are connected.
    freq: u64,
    tx: bool,
    message: String,
    /// Starts false: we must not appear publicly before the engine has told us
    /// the radio is actually in RADE.
    visible: bool,

    // ── Connection state.
    ws: Option<Ws>,
    /// Set by `connection_successful`; gates every emit.
    fully_connected: bool,
    /// Our own socket.io session id, so our station is filtered out of spots.
    my_sid: Option<String>,
    /// Server-advertised `pingInterval + pingTimeout`.
    watchdog: Duration,
    last_server_ping: Instant,
    backoff: Duration,
    /// Earliest time the next connection attempt may be made. Every attempt
    /// *and* every disconnect schedules the one after it, so no failure path
    /// can spin the loop.
    next_attempt: Instant,
    /// When we last sent something the server counts as an update from us.
    /// Only the four state emitters below move it — never the engine.io pong,
    /// which keeps the socket alive but tells the server nothing about us.
    last_report: Instant,
    last_status: Option<String>,

    // ── Inbound.
    roster: Roster,
    roster_dirty: bool,
    last_publish: Instant,

    // ── Diagnostics, surfaced by `probe`.
    got_open: bool,
    got_connect_ack: bool,
    got_connection_successful: bool,
    event_counts: BTreeMap<String, u64>,
}

impl Reporter {
    fn new(
        cfg: FreeDvReporterConfig,
        call: String,
        grid: String,
        feed: FeedTx,
        events: EventTx,
        ctrl: Receiver<Ctrl>,
    ) -> Reporter {
        let message = cfg.message.clone();
        Reporter {
            cfg,
            call,
            grid,
            feed,
            events,
            ctrl,
            freq: 0,
            tx: false,
            message,
            visible: false,
            ws: None,
            fully_connected: false,
            my_sid: None,
            watchdog: Duration::from_secs(30),
            last_server_ping: Instant::now(),
            backoff: BACKOFF_MIN,
            next_attempt: Instant::now(),
            last_report: Instant::now(),
            last_status: None,
            roster: Roster::default(),
            roster_dirty: false,
            last_publish: Instant::now(),
            got_open: false,
            got_connect_ack: false,
            got_connection_successful: false,
            event_counts: BTreeMap::new(),
        }
    }

    /// True when we have an identity to report. Without both a callsign and a
    /// grid the server puts us in the read-only "view" role.
    fn can_report(&self) -> bool {
        !self.call.trim().is_empty() && !self.grid.trim().is_empty()
    }

    /// The auth object sent in the socket.io CONNECT frame.
    ///
    /// The `"report"` role is what makes this station publicly visible, so it
    /// must only ever be reachable from an operator enabling the feature with a
    /// real callsign — never from a test or a diagnostic command.
    fn auth(&self) -> Value {
        let mut o = serde_json::Map::new();
        if self.can_report() {
            o.insert("role".into(), json!("report"));
            o.insert("callsign".into(), json!(self.call.trim().to_uppercase()));
            o.insert("grid_square".into(), json!(self.grid.trim()));
            o.insert("version".into(), json!(software_version()));
            o.insert("rx_only".into(), json!(self.cfg.rx_only));
            o.insert("os".into(), json!(os_string()));
        } else {
            o.insert("role".into(), json!("view"));
        }
        o.insert("protocol_version".into(), json!(PROTOCOL_VERSION));
        Value::Object(o)
    }

    // ── the loop ──

    /// Run until told to shut down, or until `deadline` if one is given.
    fn run_until(&mut self, deadline: Option<Instant>) {
        loop {
            if let Some(d) = deadline
                && Instant::now() >= d
            {
                self.close();
                return;
            }

            // (a) Connect, pacing every attempt.
            //
            // The backoff resets only once the server has actually accepted us
            // (`connection_successful`), never merely because the socket opened
            // — otherwise a peer that accepts TCP and then hangs up would spin
            // this loop at full speed.
            if self.ws.is_none() {
                let wait = self.next_attempt.saturating_duration_since(Instant::now());
                if !wait.is_zero() {
                    let nap = match deadline {
                        Some(d) => wait.min(d.saturating_duration_since(Instant::now())),
                        None => wait,
                    };
                    if self.sleep_or_stop(nap) {
                        return;
                    }
                    continue;
                }
                self.schedule_retry();
                match self.open() {
                    Ok(ws) => {
                        self.ws = Some(ws);
                        self.last_server_ping = Instant::now();
                    }
                    Err(e) => {
                        self.status(Some(format!("FreeDV Reporter: {e}")));
                        continue;
                    }
                }
            }

            // (b) Desired state from the engine.
            loop {
                match self.ctrl.try_recv() {
                    Ok(Ctrl::Shutdown) => {
                        self.close();
                        return;
                    }
                    Ok(c) => self.apply(c),
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        self.close();
                        return;
                    }
                }
            }

            // (c) One inbound frame.
            match self.read_frame() {
                Ok(Some(f)) => self.on_frame(f),
                Ok(None) => {}
                Err(e) => {
                    self.drop_connection(&e);
                    continue;
                }
            }

            // (d) Silent server → dead connection.
            if self.ws.is_some() && self.last_server_ping.elapsed() > self.watchdog {
                self.drop_connection("no ping from server");
                continue;
            }

            // (e) Publish the roster.
            let since = self.last_publish.elapsed();
            if (self.roster_dirty && since >= PUBLISH_MIN_INTERVAL) || since >= PUBLISH_MAX_INTERVAL
            {
                self.publish_spots();
            }

            // (f) Keep the server's record of us fresh through a quiet watch.
            if self.keepalive_due(Instant::now()) {
                self.send_keepalive();
            }
        }
    }

    /// Push the next permitted connection attempt out by the current backoff,
    /// then double it.
    fn schedule_retry(&mut self) {
        self.next_attempt = Instant::now() + self.backoff;
        self.backoff = (self.backoff * 2).min(BACKOFF_MAX);
    }

    /// Wait up to `dur`, still applying control messages. Returns true when
    /// told to shut down.
    fn sleep_or_stop(&mut self, dur: Duration) -> bool {
        let deadline = Instant::now() + dur;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            match self.ctrl.recv_timeout(left) {
                Ok(Ctrl::Shutdown) => return true,
                Ok(c) => self.apply(c),
                Err(RecvTimeoutError::Timeout) => return false,
                Err(RecvTimeoutError::Disconnected) => return true,
            }
        }
    }

    fn open(&mut self) -> Result<Ws, String> {
        let host =
            if self.cfg.host.trim().is_empty() { "qso.freedv.org" } else { self.cfg.host.trim() };
        let port = if self.cfg.port == 0 { 80 } else { self.cfg.port };

        let addr = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("cannot resolve {host}: {e}"))?
            .next()
            .ok_or_else(|| format!("no address for {host}"))?;
        let stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
            .map_err(|e| format!("connect {host}:{port}: {e}"))?;
        // The handshake gets a generous timeout of its own: interrupting it
        // aborts the connection rather than resuming where it left off.
        let _ = stream.set_read_timeout(Some(CONNECT_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CONNECT_TIMEOUT));
        let _ = stream.set_nodelay(true);

        let url = format!("ws://{host}:{port}/socket.io/?EIO=4&transport=websocket");
        let (mut ws, _) =
            tungstenite::client(url.as_str(), stream).map_err(|e| format!("handshake: {e}"))?;

        // Open the default namespace and authenticate in one frame.
        let auth = self.auth();
        debug!(%url, role = if self.can_report() { "report" } else { "view" }, "FreeDV Reporter connecting");
        ws.write(Message::Text(socketio::connect(&auth).into()))
            .and_then(|()| ws.flush())
            .map_err(|e| format!("sending connect: {e}"))?;

        // Now tighten the read timeout so the control channel is serviced
        // promptly and shutdown stays responsive.
        let _ = ws.get_ref().set_read_timeout(Some(READ_TIMEOUT));
        Ok(ws)
    }

    fn read_frame(&mut self) -> Result<Option<Frame>, String> {
        let Some(ws) = self.ws.as_mut() else { return Ok(None) };
        match ws.read() {
            Ok(Message::Text(t)) => Ok(Some(socketio::parse(t.as_str()))),
            Ok(Message::Ping(_)) => {
                // tungstenite queues the pong; make sure it leaves.
                let _ = ws.flush();
                Ok(None)
            }
            Ok(Message::Close(_)) => Err("server closed the connection".to_string()),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn on_frame(&mut self, f: Frame) {
        match f {
            Frame::Open { ping_interval_ms, ping_timeout_ms, .. } => {
                self.got_open = true;
                // Always take the watchdog from the server rather than assuming
                // a value; qso.freedv.org pings every 5 s with a 5 s timeout.
                self.watchdog = Duration::from_millis(ping_interval_ms + ping_timeout_ms);
                self.last_server_ping = Instant::now();
            }
            Frame::Ping => {
                self.last_server_ping = Instant::now();
                self.send(socketio::PONG.to_string());
            }
            Frame::Pong => self.last_server_ping = Instant::now(),
            Frame::Connected { sid } => {
                self.got_connect_ack = true;
                self.my_sid = (!sid.is_empty()).then_some(sid);
            }
            Frame::Event { name, args } => self.on_event(&name, &args, 0),
            Frame::Close => self.drop_connection("server closed the session"),
            Frame::ConnectError => self.drop_connection("server refused the connection"),
            Frame::Ignored => {}
            Frame::Invalid => self.drop_connection("unparseable data from server"),
        }
    }

    fn on_event(&mut self, name: &str, args: &Value, depth: u32) {
        *self.event_counts.entry(name.to_string()).or_default() += 1;
        match name {
            "connection_successful" => self.on_connection_successful(),
            "qsy_request" => {
                // Retuning the radio from a network message needs its own
                // consent design; for now the operator is merely told.
                let who = str_of(args, "callsign").unwrap_or_default();
                let hz = args.get("frequency").and_then(Value::as_u64).unwrap_or(0);
                let msg = str_of(args, "message").unwrap_or_default();
                self.status(Some(format!(
                    "FreeDV Reporter: {who} asks you to QSY to {:.3} kHz{}",
                    hz as f64 / 1000.0,
                    if msg.is_empty() { String::new() } else { format!(" — {msg}") }
                )));
            }
            "bulk_update" => {
                if depth >= MAX_BULK_DEPTH {
                    warn!("FreeDV Reporter: nested bulk_update ignored");
                    return;
                }
                let Some(items) = args.as_array() else { return };
                // Each item is `[name, args]` and is dispatched as if it had
                // arrived on its own.
                for item in items.clone() {
                    let Some(n) = item.get(0).and_then(Value::as_str).map(str::to_string) else {
                        continue;
                    };
                    let a = item.get(1).cloned().unwrap_or(Value::Null);
                    self.on_event(&n, &a, depth + 1);
                }
            }
            _ => {
                if self.roster.on_event(name, args) {
                    self.roster_dirty = true;
                }
            }
        }
    }

    /// The server is ready for us. This — not the socket.io connect — is where
    /// the initial state goes out.
    fn on_connection_successful(&mut self) {
        self.got_connection_successful = true;
        self.fully_connected = true;
        self.backoff = BACKOFF_MIN;
        self.status(Some(format!(
            "FreeDV Reporter: connected{}",
            if self.can_report() { "" } else { " (view only — no callsign/grid)" }
        )));
        if self.visible {
            self.emit_freq();
            self.emit_tx();
            self.emit_message();
        } else {
            self.send(socketio::emit("hide_self", None));
        }
    }

    fn apply(&mut self, c: Ctrl) {
        match c {
            Ctrl::Freq(hz) => {
                if hz != self.freq {
                    self.freq = hz;
                    self.emit_freq();
                }
            }
            Ctrl::Tx(on) => {
                if on != self.tx {
                    self.tx = on;
                    self.emit_tx();
                }
            }
            Ctrl::Message(m) => {
                if m != self.message {
                    self.message = m;
                    self.emit_message();
                }
            }
            Ctrl::Visible(v) => self.set_visible(v),
            Ctrl::RxReport { call, snr } => self.emit_rx_report(&call, snr),
            Ctrl::RxPresence(snr) => self.emit_rx_presence(snr),
            // Handled by the callers, which need to return from the loop.
            Ctrl::Shutdown => {}
        }
    }

    fn set_visible(&mut self, visible: bool) {
        if visible == self.visible {
            return; // already in this state
        }
        self.visible = visible;
        if !self.fully_connected {
            return; // replayed by on_connection_successful
        }
        if visible {
            self.send(socketio::emit("show_self", None));
            // Without this the site shows "unknown" for a just-unhidden station.
            self.emit_freq();
            self.emit_tx();
            self.emit_message();
        } else {
            self.send(socketio::emit("hide_self", None));
        }
    }

    /// True when our reported state has gone [`KEEPALIVE_INTERVAL`] without a
    /// refresh and there is a visible station for the server to remember.
    ///
    /// A hidden or view-only session is deliberately excluded: the server
    /// refuses state from the `view` role outright, and a station that has
    /// asked to be hidden has nothing to keep alive.
    fn keepalive_due(&self, now: Instant) -> bool {
        self.fully_connected
            && self.visible
            && self.can_report()
            && now.duration_since(self.last_report) >= KEEPALIVE_INTERVAL
    }

    /// Re-assert our current frequency and TX state.
    ///
    /// Both are exactly what we last reported, which is the point: the server
    /// stamps its `last_update` for our session on receipt but only relays a
    /// `freq_change` / `tx_report` on to other clients when the *value* changed
    /// — so this refreshes our record without adding a single broadcast to a
    /// server that is already busy. Re-sending unchanged state also cannot
    /// misreport us the way a synthetic RX report would: `last_tx` moves only
    /// while we really are transmitting.
    fn send_keepalive(&mut self) {
        debug!("FreeDV Reporter: keepalive");
        self.emit_freq();
        self.emit_tx();
    }

    // Each emitter stores unconditionally and sends only when fully connected,
    // so reconnects replay the current state rather than a stale one.

    fn emit_freq(&mut self) {
        if self.fully_connected {
            self.report(socketio::emit("freq_change", Some(&json!({ "freq": self.freq }))));
        }
    }

    fn emit_tx(&mut self) {
        if self.fully_connected {
            self.report(socketio::emit(
                "tx_report",
                Some(&json!({ "mode": MODE_STRING, "transmitting": self.tx })),
            ));
        }
    }

    fn emit_message(&mut self) {
        if self.fully_connected {
            self.report(socketio::emit(
                "message_update",
                Some(&json!({ "message": self.message })),
            ));
        }
    }

    fn emit_rx_report(&mut self, call: &str, snr: i32) {
        if !self.fully_connected || !self.can_report() || call.trim().is_empty() {
            return;
        }
        info!(%call, snr, "reporting RX callsign to FreeDV Reporter");
        let frame = rx_report_frame(call, snr);
        self.report(frame);
    }

    /// Report an unidentified reception: an `rx_report` with an empty callsign,
    /// which the server relays so the transmitting station can see that someone
    /// is hearing it before either end has identified the other. FreeDV GUI
    /// sends the same thing ("report '--' for callsign so we can at least report
    /// that we're receiving *something*"), and our own inbound handler already
    /// expects it.
    ///
    /// Deliberately never attributes the reception to a station: the empty
    /// callsign is what tells the server (and every listener) that nobody has
    /// been identified yet, so this can never mis-credit a decode.
    fn emit_rx_presence(&mut self, snr: i32) {
        if !self.fully_connected || !self.can_report() {
            return;
        }
        let frame = rx_report_frame("", snr);
        self.report(frame);
    }

    /// Send a frame the server treats as an update from us, and postpone the
    /// keepalive by a full interval — a station with real traffic never pays
    /// for one.
    fn report(&mut self, text: String) {
        self.last_report = Instant::now();
        self.send(text);
    }

    fn send(&mut self, text: String) {
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.write(Message::Text(text.into())).and_then(|()| ws.flush()) {
            let e = e.to_string();
            self.drop_connection(&e);
        }
    }

    fn publish_spots(&mut self) {
        self.roster_dirty = false;
        self.last_publish = Instant::now();
        let spots = if self.cfg.show_spots {
            self.roster.spots(now_utc(), self.my_sid.as_deref())
        } else {
            Vec::new()
        };
        let _ = self.feed.send((SpotKind::FreeDv, spots));
    }

    fn drop_connection(&mut self, why: &str) {
        if self.ws.take().is_some() {
            warn!("FreeDV Reporter disconnected: {why}");
            self.status(Some(format!("FreeDV Reporter: {why}")));
        }
        self.fully_connected = false;
        self.my_sid = None;
        self.roster = Roster::default();
        // Pace the reconnect too: a server that accepts us and then hangs up
        // immediately must not be retried in a tight loop either.
        self.schedule_retry();
        // Clear our markers immediately rather than letting them age out.
        let _ = self.feed.send((SpotKind::FreeDv, Vec::new()));
        self.roster_dirty = false;
        self.last_publish = Instant::now();
    }

    fn close(&mut self) {
        if let Some(mut ws) = self.ws.take() {
            let _ = ws.close(None);
            let _ = ws.flush();
        }
        let _ = self.feed.send((SpotKind::FreeDv, Vec::new()));
        self.status(None);
    }

    /// Emit a status line only when it actually changed (the [`crate::poll`]
    /// dedup), so a flapping connection cannot spam the UI.
    fn status(&mut self, s: Option<String>) {
        if s != self.last_status {
            self.last_status = s.clone();
            let _ = self.events.send(NetEvent::Status(s));
        }
    }
}

/// What we tell the reporter we are running.
fn software_version() -> String {
    format!("SDRoxide {}", env!("CARGO_PKG_VERSION"))
}

/// One `rx_report` frame. An empty `callsign` is an unidentified reception
/// ("hearing something"); a callsign is a decoded station, upper-cased for the
/// site. Shared by [`Reporter::emit_rx_report`] and
/// [`Reporter::emit_rx_presence`] so the two cannot drift in shape.
fn rx_report_frame(call: &str, snr: i32) -> String {
    socketio::emit(
        "rx_report",
        Some(&json!({
            "callsign": call.trim().to_uppercase(),
            "mode": MODE_STRING,
            "snr": snr,
        })),
    )
}

/// A short host-platform string, matching what other clients report.
fn os_string() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

// ───────────────────────────── probe ─────────────────────────────

/// One station seen during a [`probe`].
#[derive(Debug, Clone)]
pub struct ProbeStation {
    pub call: String,
    pub grid: String,
    pub freq_hz: u64,
    pub mode: String,
    pub tx: bool,
    pub rx_only: bool,
    pub version: String,
}

/// What a [`probe`] observed.
#[derive(Debug, Clone)]
pub struct ProbeSummary {
    pub host: String,
    pub port: u16,
    pub got_open: bool,
    pub got_connect_ack: bool,
    pub got_connection_successful: bool,
    /// How many of each event name arrived.
    pub event_counts: BTreeMap<String, u64>,
    pub stations: Vec<ProbeStation>,
    /// The last status line the session produced, if it failed.
    pub last_status: Option<String>,
}

impl ProbeSummary {
    /// True when the session completed a full handshake and saw at least one
    /// station — everything a working transport implies.
    pub fn ok(&self) -> bool {
        self.got_open
            && self.got_connect_ack
            && self.got_connection_successful
            && !self.stations.is_empty()
    }
}

/// Connect read-only for `secs` and report what arrived.
///
/// Uses the server's `"view"` role: nothing is reported and this station does
/// **not** appear on the public site, which makes it safe to run against
/// qso.freedv.org as a diagnostic.
pub fn probe(host: &str, port: u16, secs: f64) -> ProbeSummary {
    let cfg = FreeDvReporterConfig {
        enabled: true,
        host: host.to_string(),
        port,
        show_spots: true,
        ..Default::default()
    };
    let (feed_tx, _feed_rx) = crossbeam_channel::unbounded();
    let (event_tx, _event_rx) = crossbeam_channel::unbounded();
    let (_ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();

    // Empty callsign and grid force the read-only "view" role.
    let mut r = Reporter::new(cfg, String::new(), String::new(), feed_tx, event_tx, ctrl_rx);
    r.run_until(Some(Instant::now() + Duration::from_secs_f64(secs.max(1.0))));

    let mut stations: Vec<ProbeStation> = r
        .roster
        .by_sid
        .values()
        .map(|s| ProbeStation {
            call: s.call.clone(),
            grid: s.grid.clone(),
            freq_hz: s.freq_hz,
            mode: s.mode.clone(),
            tx: s.tx,
            rx_only: s.rx_only,
            version: s.version.clone(),
        })
        .collect();
    stations.sort_by(|a, b| a.freq_hz.cmp(&b.freq_hz).then_with(|| a.call.cmp(&b.call)));

    ProbeSummary {
        host: host.to_string(),
        port,
        got_open: r.got_open,
        got_connect_ack: r.got_connect_ack,
        got_connection_successful: r.got_connection_successful,
        event_counts: r.event_counts.clone(),
        stations,
        last_status: r.last_status.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reporter with no socket. Every send is `let _ =`, so the dangling
    /// channel ends do not matter; these tests exercise state, not I/O.
    fn reporter(call: &str, grid: &str) -> Reporter {
        let (feed_tx, _) = crossbeam_channel::unbounded();
        let (event_tx, _) = crossbeam_channel::unbounded();
        let (_, ctrl_rx) = crossbeam_channel::unbounded();
        Reporter::new(
            FreeDvReporterConfig::default(),
            call.into(),
            grid.into(),
            feed_tx,
            event_tx,
            ctrl_rx,
        )
    }

    fn connect_ev(sid: &str, call: &str, grid: &str) -> Value {
        json!({
            "sid": sid,
            "last_update": "2026-07-26T15:00:00Z",
            "callsign": call,
            "grid_square": grid,
            "version": "FreeDV 2.3.1",
            "rx_only": false,
            "connect_time": "2026-07-26T14:00:00Z",
        })
    }

    #[test]
    fn new_connection_then_freq_change_yields_a_spot() {
        let mut r = Roster::default();
        assert!(r.on_event("new_connection", &connect_ev("s1", "OE1TKJ", "JN88dd")));
        // Without a frequency there is nowhere to put the marker.
        assert!(r.spots(100, None).is_empty());

        assert!(r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 })));
        let spots = r.spots(100, None);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "OE1TKJ");
        assert_eq!(spots[0].freq_hz, 14_236_000.0);
        assert_eq!(spots[0].kind, SpotKind::FreeDv);
        assert_eq!(spots[0].grid.as_deref(), Some("JN88dd"));
        assert_eq!(spots[0].when_utc, 100);
        assert_eq!(spots[0].mode, "RADEV1");
    }

    #[test]
    fn remove_connection_drops_the_spot() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        assert_eq!(r.spots(0, None).len(), 1);

        assert!(r.on_event("remove_connection", &json!({ "sid": "s1" })));
        assert!(r.spots(0, None).is_empty());
        // Removing an unknown station is not a change.
        assert!(!r.on_event("remove_connection", &json!({ "sid": "nope" })));
    }

    #[test]
    fn our_own_sid_is_never_spotted() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("mine", "OE1TKJ", "JN88dd"));
        r.on_event("freq_change", &json!({ "sid": "mine", "freq": 14_236_000u64 }));
        r.on_event("new_connection", &connect_ev("other", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "other", "freq": 14_236_000u64 }));

        let spots = r.spots(0, Some("mine"));
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "K1ABC");
    }

    #[test]
    fn a_station_seen_only_through_freq_change_still_gets_a_callsign() {
        let mut r = Roster::default();
        r.on_event(
            "freq_change",
            &json!({ "sid": "s1", "callsign": "vk7xyz", "grid_square": "QE38", "freq": 7_177_000u64 }),
        );
        let spots = r.spots(0, None);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "VK7XYZ", "callsigns are normalised to upper case");
    }

    #[test]
    fn tx_report_sets_mode_and_flags_transmitting() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        assert!(r.on_event(
            "tx_report",
            &json!({ "sid": "s1", "mode": "RADEV1", "transmitting": true, "last_tx": Value::Null }),
        ));
        let spots = r.spots(0, None);
        assert_eq!(spots[0].mode, "RADEV1");
        assert!(spots[0].comment.contains("TX"));
    }

    #[test]
    fn rx_report_attaches_snr_and_spotter() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));

        assert!(r.on_event(
            "rx_report",
            &json!({
                "sid": "s2",
                "last_update": "now",
                "receiver_callsign": "oe1tkj",
                "receiver_grid_square": "JN88dd",
                "callsign": "K1ABC",
                "snr": -3,
                "mode": "RADEV1",
            }),
        ));
        let spots = r.spots(0, None);
        assert_eq!(spots[0].spotter, "OE1TKJ");
        assert_eq!(spots[0].snr_db, Some(-3));
    }

    #[test]
    fn rx_report_accepts_a_float_snr() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        r.on_event(
            "rx_report",
            &json!({ "receiver_callsign": "W1AW", "callsign": "K1ABC", "snr": -2.6 }),
        );
        assert_eq!(r.spots(0, None)[0].snr_db, Some(-3));
    }

    #[test]
    fn an_rx_report_without_a_callsign_is_not_a_sighting() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        // What a RADE station sends while it hears something it cannot name,
        // and what the server sends to clear RX data on a QSY.
        assert!(!r.on_event(
            "rx_report",
            &json!({ "sid": "s2", "receiver_callsign": "W1AW", "callsign": "", "snr": 0 }),
        ));
        assert_eq!(r.spots(0, None)[0].spotter, "qso.freedv.org");
        assert_eq!(r.spots(0, None)[0].snr_db, None);
    }

    #[test]
    fn message_update_reaches_the_comment() {
        let mut r = Roster::default();
        r.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        r.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        r.on_event("message_update", &json!({ "sid": "s1", "message": "CQ FreeDV" }));
        assert!(r.spots(0, None)[0].comment.starts_with("CQ FreeDV"));
    }

    #[test]
    fn wrong_types_are_dropped_not_panicked() {
        let mut r = Roster::default();
        // Frequency as a string, sid missing, snr as a string, args not an object.
        assert!(!r.on_event("freq_change", &json!({ "sid": "s1", "freq": "14074000" })));
        assert!(!r.on_event("freq_change", &json!({ "freq": 14_074_000u64 })));
        assert!(!r.on_event("new_connection", &json!({ "callsign": "K1ABC" })));
        assert!(!r.on_event("tx_report", &json!({ "sid": "s1", "transmitting": "yes" })));
        assert!(!r.on_event("rx_report", &json!({ "callsign": "K1ABC", "snr": "-3" })));
        assert!(!r.on_event("message_update", &Value::Null));
        assert!(!r.on_event("unknown_event", &json!({ "sid": "s1" })));
        assert!(r.spots(0, None).is_empty());
    }

    #[test]
    fn bulk_update_is_dispatched_pairwise() {
        let mut rep = reporter("", "");
        rep.on_event(
            "bulk_update",
            &json!([
                ["new_connection", connect_ev("s1", "K1ABC", "FN42")],
                ["freq_change", { "sid": "s1", "freq": 14_236_000u64 }],
                ["tx_report", { "sid": "s1", "mode": "RADEV1", "transmitting": true }],
            ]),
            0,
        );
        let spots = rep.roster.spots(0, None);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "K1ABC");
        assert!(spots[0].comment.contains("TX"));
        assert!(rep.roster_dirty);
        assert_eq!(rep.event_counts["bulk_update"], 1);
        assert_eq!(rep.event_counts["freq_change"], 1);
    }

    #[test]
    fn nested_bulk_update_is_refused() {
        let mut rep = reporter("", "");
        rep.on_event(
            "bulk_update",
            &json!([["bulk_update", [["new_connection", connect_ev("s1", "K1ABC", "FN42")]]]]),
            0,
        );
        assert!(rep.roster.by_sid.is_empty());
    }

    #[test]
    fn auth_uses_the_report_role_only_with_a_full_identity() {
        let full = reporter("oe1tkj", "JN88dd").auth();
        assert_eq!(full["role"], json!("report"));
        assert_eq!(full["callsign"], json!("OE1TKJ"), "callsign is upper-cased");
        assert_eq!(full["grid_square"], json!("JN88dd"));
        assert_eq!(full["protocol_version"], json!(2));
        assert_eq!(full["rx_only"], json!(false));
        assert!(full["version"].as_str().unwrap().starts_with("SDRoxide "));
        assert!(full.get("os").is_some());

        for (call, grid) in [("", "JN88dd"), ("OE1TKJ", ""), ("", "")] {
            let v = reporter(call, grid).auth();
            assert_eq!(v["role"], json!("view"), "call={call:?} grid={grid:?}");
            assert_eq!(v["protocol_version"], json!(2));
            assert!(v.get("callsign").is_none(), "view role leaks no identity");
        }
    }

    #[test]
    fn software_version_is_branded() {
        assert_eq!(software_version(), format!("SDRoxide {}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn an_unidentified_reception_carries_an_empty_callsign() {
        // The presence report is distinguished from a decoded one only by its
        // empty callsign: that is what tells the server — and every listener —
        // that nobody has been identified, so it can never mis-credit a decode.
        let presence = rx_report_frame("", -7);
        assert!(
            presence.contains("\"callsign\":\"\"") && presence.contains("\"snr\":-7"),
            "presence frame was {presence}"
        );
        let named = rx_report_frame("k1abc", -7);
        assert!(
            named.contains("\"callsign\":\"K1ABC\"") && named.contains("\"mode\":\"RADEV1\""),
            "named frame was {named}"
        );
        assert_ne!(presence, named);
    }

    #[test]
    fn nothing_is_emitted_before_connection_successful() {
        let mut rep = reporter("OE1TKJ", "JN88dd");
        // No socket at all: these must store, not send, and must not panic.
        rep.apply(Ctrl::Freq(14_236_000));
        rep.apply(Ctrl::Tx(true));
        rep.apply(Ctrl::Message("CQ".into()));
        rep.apply(Ctrl::Visible(true));
        assert_eq!(rep.freq, 14_236_000);
        assert!(rep.tx);
        assert_eq!(rep.message, "CQ");
        assert!(rep.visible);
        assert!(!rep.fully_connected);
    }

    #[test]
    fn we_start_hidden() {
        // Nothing may appear on the public site until the engine confirms the
        // radio is in RADE.
        assert!(!reporter("OE1TKJ", "JN88dd").visible);
    }

    #[test]
    fn watchdog_comes_from_the_open_frame() {
        let mut rep = reporter("", "");
        rep.on_frame(Frame::Open {
            sid: "x".into(),
            ping_interval_ms: 5000,
            ping_timeout_ms: 5000,
        });
        assert_eq!(rep.watchdog, Duration::from_secs(10));
        assert!(rep.got_open);
    }

    #[test]
    fn a_quiet_visible_station_keeps_itself_alive_once_a_minute() {
        let mut rep = reporter("OE1TKJ", "JN88dd");
        rep.fully_connected = true;
        rep.visible = true;
        let t0 = rep.last_report;

        assert!(!rep.keepalive_due(t0));
        assert!(!rep.keepalive_due(t0 + KEEPALIVE_INTERVAL - Duration::from_millis(1)));
        assert!(rep.keepalive_due(t0 + KEEPALIVE_INTERVAL));
    }

    #[test]
    fn a_station_with_real_traffic_never_pays_for_a_keepalive() {
        let mut rep = reporter("OE1TKJ", "JN88dd");
        rep.fully_connected = true;
        rep.visible = true;
        let t0 = rep.last_report;

        // Any report the server counts as an update from us postpones it...
        rep.apply(Ctrl::Freq(14_236_000));
        assert!(!rep.keepalive_due(t0 + KEEPALIVE_INTERVAL));
        // ...as does the keepalive itself, so it can never run back to back.
        rep.send_keepalive();
        assert!(!rep.keepalive_due(t0 + KEEPALIVE_INTERVAL));
        assert!(rep.keepalive_due(rep.last_report + KEEPALIVE_INTERVAL));
    }

    #[test]
    fn nothing_that_cannot_be_seen_is_kept_alive() {
        let mut rep = reporter("OE1TKJ", "JN88dd");
        rep.fully_connected = true;
        rep.visible = true;
        let due = rep.last_report + KEEPALIVE_INTERVAL;
        assert!(rep.keepalive_due(due));

        // Hidden (the radio left RADE), disconnected, and the view-only role
        // each have nothing to keep alive.
        rep.visible = false;
        assert!(!rep.keepalive_due(due));
        rep.visible = true;
        rep.fully_connected = false;
        assert!(!rep.keepalive_due(due));

        let mut view = reporter("", "");
        view.fully_connected = true;
        view.visible = true;
        assert!(!view.keepalive_due(view.last_report + KEEPALIVE_INTERVAL));
    }

    #[test]
    fn the_engine_io_pong_is_not_a_report() {
        // The pong keeps the *socket* alive and says nothing about our station,
        // so it must not postpone the keepalive.
        let mut rep = reporter("OE1TKJ", "JN88dd");
        rep.fully_connected = true;
        rep.visible = true;
        let due = rep.last_report + KEEPALIVE_INTERVAL;
        rep.on_frame(Frame::Ping);
        assert!(rep.keepalive_due(due));
    }

    #[test]
    fn spots_are_suppressed_when_show_spots_is_off() {
        let mut rep = reporter("", "");
        rep.cfg.show_spots = false;
        let (feed_tx, feed_rx) = crossbeam_channel::unbounded();
        rep.feed = feed_tx;
        rep.roster.on_event("new_connection", &connect_ev("s1", "K1ABC", "FN42"));
        rep.roster.on_event("freq_change", &json!({ "sid": "s1", "freq": 14_236_000u64 }));
        rep.publish_spots();
        let (kind, spots) = feed_rx.try_recv().expect("a batch is still published");
        assert_eq!(kind, SpotKind::FreeDv);
        assert!(spots.is_empty(), "an empty batch clears any stale markers");
    }
}
