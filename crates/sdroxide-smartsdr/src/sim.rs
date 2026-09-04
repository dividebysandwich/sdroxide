//! A wire-level FlexRadio simulator: enough of a real radio to develop and test
//! a client against when there is no radio in the room.
//!
//! This is **not** a mock inside the client. It binds real sockets and speaks
//! the real protocol — discovery broadcasts, the TCP command port, VITA-49 DAX
//! IQ and meter streams — so the client under test runs its ordinary connect
//! path, ordinary parsing and ordinary teardown against it. A mock at the API
//! seam would have proved that the client calls its own functions; this proves
//! the bytes are right, which is the part nobody can check without hardware.
//!
//! # What it models
//!
//! - Discovery broadcasts (optional; off by default so a test run doesn't
//!   announce a phantom radio to everybody else on the LAN).
//! - The `V`/`H` handshake, sequenced commands with `R` replies, and the status
//!   objects a client needs: `radio`, `slice`, `display pan`, `stream`, `meter`,
//!   `transmit`, `interlock`.
//! - A DAX IQ stream carrying a synthetic scene — a noise floor plus carriers at
//!   fixed *RF* frequencies, so tuning the simulated radio genuinely moves them
//!   through the passband instead of dragging them along.
//! - Meter packets for `SWR` and `FWDPWR`, published while keyed.
//! - DAX TX audio received back from the client, with a running sample count and
//!   peak level a test can assert on.
//!
//! # What it does not model
//!
//! Panadapter FFT and waterfall tiles, Opus audio, profiles, memories, ATU, the
//! amplifier and TNF objects. A client that needs those needs a radio.

use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::protocol as p;

/// The simulated radio's identity and behaviour.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// TCP command port. `0` picks a free one — use that in tests, and 4992
    /// when standing in for a radio a real client should find.
    pub port: u16,
    /// Bind address. Loopback by default: a simulator that answered on the LAN
    /// could be picked up by somebody else's SmartSDR.
    pub bind: IpAddr,
    pub model: String,
    pub serial: String,
    pub nickname: String,
    pub callsign: String,
    pub version: String,
    /// Broadcast discovery packets on UDP 4992. Off by default — see the module
    /// docs.
    pub announce: bool,
    /// Where the simulated slice starts, in Hz.
    pub start_freq_hz: f64,
    /// Carriers in the synthetic scene, as `(RF frequency Hz, amplitude)`.
    /// Anchored in RF, not in the passband, so tuning moves through them.
    pub carriers: Vec<(f64, f32)>,
    /// Noise floor amplitude.
    pub noise: f32,
    /// Refuse GUI registration, to exercise the client's error path.
    pub refuse_gui: bool,
    /// GUI clients already registered when ours arrives, as
    /// `(client_id, station)`. A `client gui` naming one of these ids evicts it
    /// and says so, exactly as a radio does — which is the fault a client that
    /// derives its id from a default station name walks into every time.
    pub existing_clients: Vec<(String, String)>,
    /// Refuse the TCP `client udpport` command and learn the client's UDP
    /// endpoint only from a datagram it sends us.
    ///
    /// Not a hypothetical: the v1.4.0.0 protocol a FLEX-6600 speaks may answer
    /// `client udpport` with "command not supported", leaving the registration
    /// datagram as the only thing that tells the radio where to stream. A
    /// simulator that always honours `client udpport` cannot fail that way, and
    /// so cannot catch a client that never sends the datagram.
    pub require_udp_register: bool,
    /// Swallow the first `display pan set … daxiq_channel=`, as a radio does
    /// when the panadapter it names was created a moment ago and is not ready to
    /// be configured yet.
    ///
    /// It is acknowledged and it does nothing, so the DAX IQ stream comes up
    /// bound to no panadapter: created, accepting a rate, reporting itself
    /// active, and carrying nothing at all. The silent half of "connects fine,
    /// no spectrum", and the reason a client has to read `pan=` back instead of
    /// assuming the bind landed.
    pub unbound_dax_iq: bool,
    /// Evict this client for a duplicate client id once it has been registered
    /// for this long, as a radio does when a second client claims its id.
    pub evict_after: Option<Duration>,
    /// The radio still holds a panadapter for this client from an earlier
    /// session, as one does when a GUI client comes back under an id it has
    /// used before — it is reported in the `sub pan all` burst before anything
    /// is created.
    ///
    /// The state issue #309 arrives in one half of: a client that always
    /// creates another leaves a second one behind every session, and a
    /// FLEX-6400 has two.
    pub restored_pan: bool,
    /// Every panadapter the radio has is already in use — by SmartSDR, or by
    /// this client's own abandoned sessions — so `display panafall create` is
    /// refused. The other half of issue #309, `0x50000009` on a 6400.
    pub no_free_pan: bool,
    /// The client handle the existing panadapter belongs to when it is not
    /// ours. Only meaningful with [`Self::no_free_pan`].
    pub foreign_pan_handle: u32,
    /// Stream to FlexLib's default client port (4991) rather than to whatever
    /// address the client's datagrams came from.
    ///
    /// What a radio does with a client whose own port it never recorded. The
    /// client is on the right machine and listening on the wrong socket, which
    /// looks exactly like a firewall from the inside and is not one.
    pub stream_to_default_port: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            port: 0,
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            model: "FLEX-6600".into(),
            serial: "0000-0000-6600-0000".into(),
            nickname: "sdroxide-sim".into(),
            callsign: "SIM".into(),
            version: "3.3.28.0".into(),
            announce: false,
            start_freq_hz: 14_074_000.0,
            // A scene spread across 20 m: one carrier on the FT8 watering hole,
            // others far enough out that a 192 kHz span holds several and a
            // 24 kHz span holds one.
            carriers: vec![
                (14_074_000.0, 0.20),
                (14_080_000.0, 0.08),
                (14_100_000.0, 0.30),
                (14_020_000.0, 0.05),
            ],
            noise: 0.002,
            refuse_gui: false,
            existing_clients: Vec::new(),
            require_udp_register: false,
            unbound_dax_iq: false,
            evict_after: None,
            restored_pan: false,
            no_free_pan: false,
            foreign_pan_handle: 0xDEAD_BEEF,
            stream_to_default_port: false,
        }
    }
}

/// Counters a test can assert on without reaching into the simulator's threads.
#[derive(Debug, Default)]
pub struct SimStats {
    /// Commands received, by their first word (`slice`, `stream`, `display`, …).
    pub commands: Mutex<Vec<String>>,
    /// DAX TX audio frames received from the client.
    pub tx_frames: AtomicU64,
    /// Peak absolute sample seen in that audio, scaled by 1e6 so it fits an
    /// integer — enough resolution to tell silence from signal.
    pub tx_peak_micro: AtomicU64,
    /// IQ packets sent to the client.
    pub iq_packets: AtomicU64,
    /// Whether the radio is currently keyed.
    pub keyed: AtomicBool,
    /// The slice frequency, in Hz, times 1000 (so it fits an integer).
    pub slice_milli_hz: AtomicU64,
}

impl SimStats {
    /// Every command the simulator has been sent, in order.
    pub fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Whether any command starting with `prefix` has been received.
    pub fn saw_command(&self, prefix: &str) -> bool {
        self.commands().iter().any(|c| c.starts_with(prefix))
    }

    pub fn slice_freq_hz(&self) -> f64 {
        self.slice_milli_hz.load(Ordering::Relaxed) as f64 / 1000.0
    }

    pub fn tx_peak(&self) -> f32 {
        self.tx_peak_micro.load(Ordering::Relaxed) as f32 / 1e6
    }
}

/// A running simulator. Dropping it shuts every thread down.
pub struct SimRadio {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    joins: Vec<JoinHandle<()>>,
    pub stats: Arc<SimStats>,
}

impl SimRadio {
    /// Bind the sockets and start serving.
    pub fn start(cfg: SimConfig) -> std::io::Result<SimRadio> {
        let listener = TcpListener::bind(SocketAddr::new(cfg.bind, cfg.port))?;
        let addr = listener.local_addr()?;
        listener.set_nonblocking(true)?;

        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(SimStats::default());
        stats.slice_milli_hz.store((cfg.start_freq_hz * 1000.0) as u64, Ordering::Relaxed);

        let mut joins = Vec::new();

        if cfg.announce {
            let payload = discovery_payload(&cfg, addr.port());
            let stop_a = Arc::clone(&stop);
            joins.push(std::thread::Builder::new().name("flexsim-announce".into()).spawn(
                move || {
                    announce_loop(&payload, &stop_a);
                },
            )?);
        }

        let stop_l = Arc::clone(&stop);
        let stats_l = Arc::clone(&stats);
        joins.push(std::thread::Builder::new().name("flexsim-accept".into()).spawn(move || {
            accept_loop(listener, cfg, stop_l, stats_l);
        })?);

        Ok(SimRadio { addr, stop, joins, stats })
    }

    /// The address a client should connect to (`host:port`).
    pub fn address(&self) -> String {
        format!("{}:{}", self.addr.ip(), self.addr.port())
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Block until `f` is true, or `timeout` expires. Returns whether it became
    /// true — which is what a test asserts on, rather than sleeping a guessed
    /// interval and hoping.
    pub fn wait_for(&self, timeout: Duration, f: impl Fn(&SimStats) -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if f(&self.stats) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        f(&self.stats)
    }
}

impl Drop for SimRadio {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for j in self.joins.drain(..) {
            let _ = j.join();
        }
    }
}

/// The key/value payload a radio broadcasts.
fn discovery_payload(cfg: &SimConfig, port: u16) -> String {
    format!(
        "discovery_protocol_version=3.0.0.1 model={} serial={} version={} nickname={} \
         callsign={} ip=127.0.0.1 port={port} status=Available max_licensed_version=v3 \
         mf_enable=1 radio_license_id=00-00-00-00-00-00 requires_additional_license=0",
        cfg.model, cfg.serial, cfg.version, cfg.nickname, cfg.callsign
    )
}

fn announce_loop(payload: &str, stop: &AtomicBool) {
    let Ok(sock) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else { return };
    if sock.set_broadcast(true).is_err() {
        return;
    }
    while !stop.load(Ordering::Relaxed) {
        let _ = crate::discovery::send_broadcast(&sock, payload, p::RADIO_PORT);
        // A real radio announces about once a second.
        for _ in 0..20 {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn accept_loop(listener: TcpListener, cfg: SimConfig, stop: Arc<AtomicBool>, stats: Arc<SimStats>) {
    let mut sessions: Vec<JoinHandle<()>> = Vec::new();
    let mut next_handle: u32 = 0x51_00_00_01;

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((sock, _peer)) => {
                let handle = next_handle;
                next_handle = next_handle.wrapping_add(1);
                let cfg = cfg.clone();
                let stop = Arc::clone(&stop);
                let stats = Arc::clone(&stats);
                if let Ok(j) = std::thread::Builder::new()
                    .name("flexsim-session".into())
                    .spawn(move || Session::new(sock, cfg, handle, stop, stats).run())
                {
                    sessions.push(j);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
        sessions.retain(|j| !j.is_finished());
    }
    for j in sessions {
        let _ = j.join();
    }
}

/// Stream ids the simulator hands out. Chosen in the ranges a real radio uses,
/// so a client that (wrongly) infers stream type from the id still behaves the
/// same way here as it would on hardware.
const IQ_STREAM_BASE: u32 = 0x0400_0000;
const TX_STREAM_BASE: u32 = 0x0B00_0000;
const PAN_ID: u32 = 0x4000_0000;
const WATERFALL_ID: u32 = 0x4200_0000;
const METER_STREAM: u32 = 0x0000_0700;

/// Meter ids the simulator publishes definitions for.
const METER_SWR: u16 = 12;
const METER_FWD: u16 = 13;
const METER_LEVEL: u16 = 7;

/// One connected client.
struct Session {
    sock: TcpStream,
    cfg: SimConfig,
    handle: u32,
    stop: Arc<AtomicBool>,
    stats: Arc<SimStats>,
    /// Where the client asked us to send VITA-49 packets.
    client_udp: Option<SocketAddr>,
    /// Our UDP socket: both the source of streams to the client and the port
    /// its DAX TX audio arrives on.
    udp: Option<UdpSocket>,
    gui_registered: bool,
    /// When [`SimConfig::evict_after`] starts counting.
    registered_at: Option<Instant>,
    /// Whether the eviction notice has already gone out.
    evicted: bool,
    /// Other GUI clients the radio believes are connected, as
    /// `(handle, client_id, station)`.
    others: Vec<(u32, String, String)>,
    /// Whether this client has a panadapter on the radio — none until one is
    /// created, unless [`SimConfig::restored_pan`] says the radio kept one from
    /// an earlier session.
    pan_exists: bool,
    /// Whether a panadapter is bound to our DAX IQ channel.
    iq_pan_bound: bool,
    /// Whether [`SimConfig::unbound_dax_iq`] has already eaten a bind.
    swallowed_bind: bool,
    iq_stream: Option<u32>,
    iq_channel: u32,
    iq_rate: u32,
    tx_stream: Option<u32>,
    pan_center_hz: f64,
    pan_bw_hz: f64,
    slice_hz: f64,
    slice_mode: String,
    keyed: bool,
    rfpower: u32,
    tunepower: u32,
    /// Phase accumulators, one per carrier, so the scene is continuous across
    /// packets rather than restarting each time.
    phases: Vec<f64>,
    iq_count: u8,
    meter_count: u8,
    samples_due: f64,
    last_tick: Instant,
    rng: u64,
}

impl Session {
    fn new(
        sock: TcpStream,
        cfg: SimConfig,
        handle: u32,
        stop: Arc<AtomicBool>,
        stats: Arc<SimStats>,
    ) -> Session {
        let phases = vec![0.0; cfg.carriers.len()];
        let slice_hz = cfg.start_freq_hz;
        // A radio restores a GUI client's panadapters when it comes back under
        // an id it has used before, and one it has none left to give reports
        // whoever is holding them.
        let cfg_restored_pan = cfg.restored_pan || cfg.no_free_pan;
        // Handles for the pre-existing clients, kept clear of ours.
        let others: Vec<(u32, String, String)> = cfg
            .existing_clients
            .iter()
            .enumerate()
            .map(|(i, (id, station))| (0x1000_0000 + i as u32, id.clone(), station.clone()))
            .collect();
        Session {
            sock,
            cfg,
            handle,
            stop,
            stats,
            client_udp: None,
            udp: None,
            gui_registered: false,
            registered_at: None,
            evicted: false,
            others,
            pan_exists: cfg_restored_pan,
            iq_pan_bound: false,
            swallowed_bind: false,
            iq_stream: None,
            iq_channel: 0,
            iq_rate: 48_000,
            tx_stream: None,
            pan_center_hz: slice_hz,
            pan_bw_hz: 192_000.0,
            slice_hz,
            slice_mode: "USB".into(),
            keyed: false,
            rfpower: 25,
            tunepower: 10,
            phases,
            iq_count: 0,
            meter_count: 0,
            samples_due: 0.0,
            last_tick: Instant::now(),
            rng: 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn run(mut self) {
        let Ok(writer) = self.sock.try_clone() else { return };
        if self.sock.set_read_timeout(Some(Duration::from_millis(10))).is_err() {
            return;
        }
        self.sock.set_nodelay(true).ok();
        let mut reader = BufReader::new(writer);

        // The radio speaks first.
        self.send_line(&format!("V{}\n", self.cfg.version));
        self.send_line(&format!("H{:08X}\n", self.handle));

        // A real radio uses one port number for both TCP and UDP (they are
        // separate address spaces, so 4992 can be both), and a client that
        // assumes that must find something listening here. Fall back to an
        // ephemeral port when a second session already holds it — the client
        // learns the real endpoint from the packets it receives.
        let tcp_port = self.sock.local_addr().map(|a| a.port()).unwrap_or(0);
        let udp = match UdpSocket::bind(SocketAddr::new(self.cfg.bind, tcp_port))
            .or_else(|_| UdpSocket::bind(SocketAddr::new(self.cfg.bind, 0)))
        {
            Ok(u) => u,
            Err(_) => return,
        };
        udp.set_read_timeout(Some(Duration::from_millis(1))).ok();
        self.udp = Some(udp);

        self.last_tick = Instant::now();
        let mut rx_buf = vec![0u8; 16 * 1024];

        while !self.stop.load(Ordering::Relaxed) {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // client hung up
                Ok(_) => self.on_line(line.trim_end()),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => break,
            }

            self.maybe_evict();
            self.pump_iq();
            self.pump_meters();
            self.drain_client_udp(&mut rx_buf);
        }

        self.stats.keyed.store(false, Ordering::Relaxed);
    }

    fn send_line(&mut self, line: &str) {
        let _ = self.sock.write_all(line.as_bytes());
        let _ = self.sock.flush();
    }

    fn reply(&mut self, seq: u32, code: u32, body: &str) {
        self.send_line(&format!("R{seq}|{code:X}|{body}\n"));
    }

    fn status(&mut self, body: &str) {
        let h = self.handle;
        self.send_line(&format!("S{h:08X}|{body}\n"));
    }

    /// A client only ever sends `C<seq>|<command>`; anything else is silently
    /// dropped, which is what a radio does with a line it cannot place.
    fn on_line(&mut self, raw: &str) {
        let Some(rest) = raw.strip_prefix('C') else { return };
        let Some((seq_s, cmd)) = rest.split_once('|') else { return };
        let seq: u32 = seq_s.parse().unwrap_or(0);
        self.stats.commands.lock().unwrap_or_else(|e| e.into_inner()).push(cmd.to_string());
        self.dispatch(seq, cmd);
    }

    fn dispatch(&mut self, seq: u32, cmd: &str) {
        let mut words = cmd.split_whitespace();
        let head = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();

        match (head, rest.first().copied()) {
            ("client", Some("program")) => self.reply(seq, 0, ""),
            ("client", Some("gui")) => {
                if self.cfg.refuse_gui {
                    // The code a radio returns when multiFLEX is off and
                    // somebody else already has it.
                    self.send_line("MF3000001|Radio is in use by another client\n");
                    self.reply(seq, 0xF300_0001, "in use");
                    return;
                }
                // A radio does not refuse a duplicate client id — it hands the
                // session to the newcomer and throws the incumbent off. That
                // silence is the whole problem: nothing in the reply says
                // somebody was just disconnected on our behalf.
                let wanted = rest.get(1).copied().unwrap_or("");
                if let Some(pos) =
                    self.others.iter().position(|(_, id, _)| id.eq_ignore_ascii_case(wanted))
                {
                    let (handle, _, _) = self.others.remove(pos);
                    self.status(&format!(
                        "client 0x{handle:08X} disconnected forced=0                          wan_validation_failed=0 duplicate_client_id=1"
                    ));
                }
                self.gui_registered = true;
                self.registered_at = Some(Instant::now());
                self.reply(seq, 0, wanted);
                let h = self.handle;
                self.status(&format!(
                    "client 0x{h:08X} connected local_ptt=1 client_id={wanted} program=sdroxide"
                ));
                self.send_radio_status();
                self.send_meter_defs();
                self.send_slice_status();
                self.send_transmit_status();
            }
            ("client", Some("station")) => self.reply(seq, 0, ""),
            ("client", Some("udpport")) if self.cfg.require_udp_register => {
                // What a v1.4.0.0 radio can answer. The client must fall back to
                // teaching us its address with a datagram of its own.
                self.reply(seq, 0x5000_1000, "command not supported");
            }
            ("client", Some("set")) => self.reply(seq, 0, ""),
            ("client", Some("udpport")) => {
                match rest.get(1).and_then(|p| p.parse::<u16>().ok()) {
                    Some(port) => {
                        // The client is on the far end of the TCP socket, so
                        // that is where its UDP port is too.
                        let peer = self.sock.peer_addr().map(|a| a.ip());
                        self.client_udp = peer.ok().map(|ip| SocketAddr::new(ip, port));
                        self.reply(seq, 0, "");
                    }
                    None => self.reply(seq, 0x5000_0001, "bad port"),
                }
            }
            ("sub", _) => {
                // Subscribing is what makes a radio dump the current state of
                // that topic — the burst *is* the subscription's payload, and a
                // client that only ever sees future changes never learns which
                // slice it owns.
                //
                // The burst goes out BEFORE the reply, which is the order a real
                // radio uses (a field trace shows every `S…|slice` arriving
                // ahead of its `R6|0|`). Replying first looks equivalent and is
                // not: a client that reads up to the reply and then acts on what
                // it collected sees an empty burst, which is how a duplicate
                // client-id check passed against a radio that had one.
                match rest.first().copied() {
                    Some("slice") => self.send_slice_status(),
                    Some("pan") => self.send_pan_status(),
                    Some("tx") => self.send_transmit_status(),
                    Some("meter") => self.send_meter_defs(),
                    Some("radio") => self.send_radio_status(),
                    Some("client") => self.send_client_status(),
                    _ => {}
                }
                self.reply(seq, 0, "");
            }
            ("keepalive", _) | ("ping", _) => self.reply(seq, 0, ""),
            ("display", Some("panafall")) => match rest.get(1).copied() {
                // A radio has a fixed number of panadapters and refuses to make
                // one past it. The code is the one a FLEX-6400 answered with in
                // issue #309, and it comes with an empty body, which is what
                // made it so hard to read.
                Some("create") if self.cfg.no_free_pan => self.reply(seq, 0x5000_0009, ""),
                Some("create") => {
                    self.pan_exists = true;
                    self.reply(seq, 0, &format!("{PAN_ID:X},{WATERFALL_ID:X}"));
                    self.send_pan_status();
                }
                Some("remove") => {
                    self.pan_exists = false;
                    self.reply(seq, 0, "");
                }
                _ => self.reply(seq, 0x5000_1000, "unknown panafall verb"),
            },
            ("display", Some("pan")) => {
                if rest.get(1).copied() != Some("set") {
                    self.reply(seq, 0x5000_1000, "unknown pan verb");
                    return;
                }
                let kvs = p::parse_kvs(&rest[2..].join(" "));
                if let Some(c) = kvs.get("center").and_then(|v| v.parse::<f64>().ok()) {
                    self.pan_center_hz = c * 1e6;
                }
                if let Some(b) = kvs.get("bandwidth").and_then(|v| v.parse::<f64>().ok()) {
                    self.pan_bw_hz = b * 1e6;
                }
                if let Some(ch) = kvs.get("daxiq_channel").and_then(|v| v.parse::<u32>().ok()) {
                    if self.cfg.unbound_dax_iq && !self.swallowed_bind {
                        // Acknowledged, and dropped on the floor.
                        self.swallowed_bind = true;
                        self.reply(seq, 0, "");
                        return;
                    }
                    self.iq_channel = ch;
                    self.iq_pan_bound = true;
                    // The binding is what an IQ stream needs to carry anything,
                    // so report it — the client is entitled to read `pan=` back.
                    self.send_iq_stream_status();
                }
                self.reply(seq, 0, "");
                self.send_pan_status();
            }
            ("slice", Some("tune")) => {
                let hz = rest.get(2).and_then(|v| v.parse::<f64>().ok()).map(|m| m * 1e6);
                if let Some(hz) = hz {
                    self.slice_hz = hz;
                    self.stats.slice_milli_hz.store((hz * 1000.0) as u64, Ordering::Relaxed);
                }
                self.reply(seq, 0, "");
                self.send_slice_status();
            }
            ("slice", Some("set")) => {
                let kvs = p::parse_kvs(&rest[2..].join(" "));
                if let Some(m) = kvs.get("mode") {
                    self.slice_mode = m.to_ascii_uppercase();
                }
                self.reply(seq, 0, "");
                self.send_slice_status();
            }
            ("stream", Some("create")) => {
                let kvs = p::parse_kvs(&rest[1..].join(" "));
                match kvs.get("type").map(|s| s.as_str()) {
                    Some("dax_iq") => {
                        let ch = kvs
                            .get("daxiq_channel")
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(1);
                        let id = IQ_STREAM_BASE | ch;
                        self.iq_stream = Some(id);
                        self.iq_channel = ch;
                        // A real radio always creates at 48 kHz and moves only
                        // on a follow-up `stream set daxiq_rate` — a client that
                        // skips that command must see the same 48 kHz here.
                        self.iq_rate = 48_000;
                        self.reply(seq, 0, &p::hex_id(id));
                        self.send_iq_stream_status();
                    }
                    Some("dax_tx") => {
                        let id = TX_STREAM_BASE | 2;
                        self.tx_stream = Some(id);
                        self.reply(seq, 0, &p::hex_id(id));
                        let h = self.handle;
                        self.status(&format!(
                            "stream {} type=dax_tx client_handle=0x{h:08X} tx=0",
                            p::hex_id(id)
                        ));
                    }
                    _ => self.reply(seq, 0x5000_1000, "unsupported stream type"),
                }
            }
            ("stream", Some("set")) => {
                let id = rest.get(1).copied().and_then(p::parse_hex_id);
                let kvs = p::parse_kvs(&rest[2..].join(" "));
                if let Some(rate) = kvs.get("daxiq_rate").and_then(|v| v.parse::<u32>().ok())
                    && id == self.iq_stream
                {
                    if p::dax_iq_rate(p::dax_iq_class_code(rate)) == Some(rate) {
                        self.iq_rate = rate;
                        self.reply(seq, 0, "");
                        self.send_iq_stream_status();
                        return;
                    }
                    self.reply(seq, 0x5000_0020, "unsupported daxiq_rate");
                    return;
                }
                self.reply(seq, 0, "");
                if let (Some(id), Some(tx)) = (id, kvs.get("tx"))
                    && Some(id) == self.tx_stream
                {
                    let h = self.handle;
                    self.status(&format!(
                        "stream {} type=dax_tx client_handle=0x{h:08X} tx={tx}",
                        p::hex_id(id)
                    ));
                }
            }
            ("stream", Some("remove")) => {
                let id = rest.get(1).copied().and_then(p::parse_hex_id);
                if id == self.iq_stream {
                    self.iq_stream = None;
                }
                if id == self.tx_stream {
                    self.tx_stream = None;
                }
                self.reply(seq, 0, "");
                if let Some(id) = id {
                    self.status(&format!("stream {} removed", p::hex_id(id)));
                }
            }
            ("transmit", Some("set")) => {
                let kvs = p::parse_kvs(&rest[1..].join(" "));
                if let Some(v) = kvs.get("rfpower").and_then(|v| v.parse::<u32>().ok()) {
                    self.rfpower = v.min(100);
                }
                if let Some(v) = kvs.get("tunepower").and_then(|v| v.parse::<u32>().ok()) {
                    self.tunepower = v.min(100);
                }
                self.reply(seq, 0, "");
                self.send_transmit_status();
            }
            ("xmit", _) => {
                self.keyed = rest.first().copied() == Some("1");
                self.stats.keyed.store(self.keyed, Ordering::Relaxed);
                self.reply(seq, 0, "");
                let state = if self.keyed { "TRANSMITTING" } else { "READY" };
                self.status(&format!("interlock state={state} source=SW"));
            }
            _ => {
                // Unknown but well-formed: a radio answers "no such object"
                // rather than closing the session, and a client must cope.
                self.reply(seq, 0x5000_1000, "unknown command");
            }
        }
    }

    // ── Status objects ──────────────────────────────────────────────────────

    fn send_radio_status(&mut self) {
        let (m, n, c) =
            (self.cfg.model.clone(), self.cfg.nickname.clone(), self.cfg.callsign.clone());
        self.status(&format!(
            "radio slices=4 panadapters=4 lineout_gain=50 lineout_mute=0 \
             model={m} nickname={n} callsign={c} mf_enable=1 daxiq_capacity=16 daxiq_available=16"
        ));
    }

    fn send_pan_status(&mut self) {
        if !self.pan_exists {
            return;
        }
        // A radio with none free is holding them for somebody else, so the one
        // it reports is not under our handle — which is what decides whether a
        // client may take it up or is only borrowing it.
        let h = if self.cfg.no_free_pan { self.cfg.foreign_pan_handle } else { self.handle };
        let (c, b, ch) = (self.pan_center_hz, self.pan_bw_hz, self.iq_channel);
        self.status(&format!(
            "display pan {} client_handle=0x{h:08X} wnb=0 wnb_level=0 band=20 \
             center={:.6} bandwidth={:.6} min_dbm=-135.0 max_dbm=-40.0 \
             x_pixels=1024 y_pixels=700 fps=25 average=30 weighted_average=0 \
             rfgain=0 rxant=ANT1 ant_list=ANT1,ANT2,RX_A daxiq_channel={ch} \
             waterfall={}",
            p::hex_id(PAN_ID),
            c / 1e6,
            b / 1e6,
            p::hex_id(WATERFALL_ID),
        ));
    }

    fn send_slice_status(&mut self) {
        let h = self.handle;
        let (f, m) = (self.slice_hz, self.slice_mode.clone());
        self.status(&format!(
            "slice 0 in_use=1 client_handle=0x{h:08X} pan={} RF_frequency={:.6} \
             mode={m} filter_lo=100 filter_hi=2900 active=1 tx=1 \
             mode_list=LSB,USB,AM,SAM,CW,DIGL,DIGU,FM,NFM,DFM,RTTY \
             rxant=ANT1 ant_list=ANT1,ANT2,RX_A agc_mode=med agc_threshold=65",
            p::hex_id(PAN_ID),
            f / 1e6,
        ));
    }

    fn send_transmit_status(&mut self) {
        let (rf, tune) = (self.rfpower, self.tunepower);
        self.status(&format!(
            "transmit rfpower={rf} tunepower={tune} am_carrier_level=100 mic_selection=MIC \
             mic_level=50 mic_boost=1 dax=0 met_in_rx=0 monitor=0 vox_enable=0 tune=0"
        ));
    }

    /// Meter definitions, in the `#`-separated form only this object uses.
    fn send_meter_defs(&mut self) {
        self.status(&format!(
            "meter {METER_LEVEL}.src=SLC#{METER_LEVEL}.num=0#{METER_LEVEL}.nam=LEVEL#\
             {METER_LEVEL}.low=-150.0#{METER_LEVEL}.hi=20.0#{METER_LEVEL}.desc=Signal level#\
             {METER_LEVEL}.unit=dBm#{METER_LEVEL}.fps=10"
        ));
        self.status(&format!(
            "meter {METER_SWR}.src=TX-#{METER_SWR}.num=0#{METER_SWR}.nam=SWR#\
             {METER_SWR}.low=1.0#{METER_SWR}.hi=10.0#{METER_SWR}.desc=SWR#\
             {METER_SWR}.unit=SWR#{METER_SWR}.fps=20"
        ));
        self.status(&format!(
            "meter {METER_FWD}.src=TX-#{METER_FWD}.num=0#{METER_FWD}.nam=FWDPWR#\
             {METER_FWD}.low=0.0#{METER_FWD}.hi=100.0#{METER_FWD}.desc=Forward power#\
             {METER_FWD}.unit=dBm#{METER_FWD}.fps=20"
        ));
    }

    /// The connected-client list, which is what `sub client all` is for: a
    /// client can only avoid stealing an id it can see somebody else holding.
    fn send_client_status(&mut self) {
        for (handle, id, station) in self.others.clone() {
            self.status(&format!(
                "client 0x{handle:08X} connected local_ptt=1 client_id={id} \
                 program=SmartSDR station={station}"
            ));
        }
    }

    fn send_iq_stream_status(&mut self) {
        let Some(id) = self.iq_stream else { return };
        let h = self.handle;
        let (ch, rate) = (self.iq_channel, self.iq_rate);
        let bound = self.iq_pan_bound || !self.cfg.unbound_dax_iq;
        let pan = if bound { p::hex_id(PAN_ID) } else { "0x0".to_string() };
        self.status(&format!(
            "stream {} type=dax_iq daxiq_channel={ch} pan={pan} daxiq_rate={rate} \
             client_handle=0x{h:08X} active=1",
            p::hex_id(id),
        ));
    }

    // ── Data plane ──────────────────────────────────────────────────────────

    /// Generate and send however much IQ real time has come due since the last
    /// call, so the stream runs at its declared rate rather than as fast as the
    /// loop spins.
    fn pump_iq(&mut self) {
        // A stream bound to no panadapter has no baseband to carry. It exists,
        // it answers commands, and it is silent.
        if self.cfg.unbound_dax_iq && !self.iq_pan_bound {
            self.last_tick = Instant::now();
            return;
        }
        let dest = match (self.client_udp, self.cfg.stream_to_default_port) {
            (Some(a), true) => Some(SocketAddr::new(a.ip(), crate::net::VITA_PORT)),
            (a, _) => a,
        };
        let (Some(stream), Some(dest)) = (self.iq_stream, dest) else {
            // Nothing to send yet: reset the clock so the first packet after the
            // stream comes up isn't a burst covering the whole setup handshake.
            self.last_tick = Instant::now();
            return;
        };

        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.samples_due += dt * self.iq_rate as f64;
        // Don't try to make up an unbounded backlog after a stall: a client that
        // was paused wants current samples, not a burst of stale ones.
        self.samples_due = self.samples_due.min(self.iq_rate as f64 * 0.25);

        // 128 I/Q pairs per packet keeps every datagram well inside an Ethernet
        // MTU at all four rates.
        const PAIRS: usize = 128;
        let class_code = p::dax_iq_class_code(self.iq_rate);
        let mut floats = Vec::with_capacity(PAIRS * 2);
        let mut payload = Vec::with_capacity(PAIRS * 8);

        while self.samples_due >= PAIRS as f64 {
            self.samples_due -= PAIRS as f64;
            floats.clear();
            self.generate_iq(PAIRS, &mut floats);
            payload.clear();
            p::encode_dax_iq(&floats, &mut payload);

            let mut pkt = Vec::with_capacity(p::VITA_HEADER_BYTES + payload.len());
            p::write_vita_header(&mut pkt, 3, stream, class_code, self.iq_count, payload.len() / 4);
            pkt.extend_from_slice(&payload);
            self.iq_count = (self.iq_count + 1) & 0x0F;

            if self.udp.as_ref().is_some_and(|u| u.send_to(&pkt, dest).is_ok()) {
                self.stats.iq_packets.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Fill `out` with `pairs` interleaved I/Q samples of the synthetic scene.
    ///
    /// The carriers are at fixed RF frequencies, so their offset in the baseband
    /// is `carrier - pan_center`. That is the whole point: tuning the simulated
    /// radio has to move signals through the passband the way a real one does,
    /// or a client's tuning bugs are invisible against it.
    fn generate_iq(&mut self, pairs: usize, out: &mut Vec<f32>) {
        use std::f64::consts::TAU;
        let rate = self.iq_rate as f64;
        let center = self.pan_center_hz;
        // Keying mutes the receiver, as a half-duplex radio does.
        let gain = if self.keyed { 0.0 } else { 1.0 };

        for _ in 0..pairs {
            let mut i = self.noise() * self.cfg.noise;
            let mut q = self.noise() * self.cfg.noise;
            for (n, &(freq, amp)) in self.cfg.carriers.iter().enumerate() {
                let offset = freq - center;
                // A carrier outside the span would alias into it — a real
                // receiver's filter has already removed it.
                if offset.abs() > rate / 2.0 {
                    continue;
                }
                let ph = self.phases[n];
                i += (ph.cos() * amp as f64) as f32;
                q += (ph.sin() * amp as f64) as f32;
                self.phases[n] = (ph + TAU * offset / rate).rem_euclid(TAU);
            }
            out.push(i * gain);
            out.push(q * gain);
        }
    }

    /// Publish SWR and forward power while keyed, at roughly the 20 fps a real
    /// radio uses for those meters.
    fn pump_meters(&mut self) {
        let (Some(dest), Some(udp)) = (self.client_udp, self.udp.as_ref()) else { return };
        if !self.keyed {
            return;
        }
        // Tie the cadence to the IQ packet counter rather than a second clock:
        // at 128 pairs a packet, every 16th is ~10 ms at 192 kHz and ~85 ms at
        // 24 kHz, both close enough to a meter update.
        if !self.iq_count.is_multiple_of(16) {
            return;
        }
        // Plausible readings: SWR just above unity, power tracking the drive
        // setting. Fixed rather than random so a test can assert on them.
        let swr = 1.3f32;
        let fwd_dbm = 10.0 * (self.rfpower.max(1) as f32).log10() + 30.0;
        let meters = [
            p::RawMeter { id: METER_SWR, raw: (swr * 128.0) as i16 },
            p::RawMeter { id: METER_FWD, raw: (fwd_dbm * 128.0) as i16 },
        ];
        let mut payload = Vec::with_capacity(8);
        p::encode_meters(&meters, &mut payload);
        let mut pkt = Vec::with_capacity(p::VITA_HEADER_BYTES + payload.len());
        p::write_vita_header(
            &mut pkt,
            3,
            METER_STREAM,
            p::pcc::METER,
            self.meter_count,
            payload.len() / 4,
        );
        pkt.extend_from_slice(&payload);
        self.meter_count = (self.meter_count + 1) & 0x0F;
        let _ = udp.send_to(&pkt, dest);
    }

    /// Receive whatever the client sends us — DAX TX audio, and the priming
    /// datagram it opens with.
    fn drain_client_udp(&mut self, buf: &mut [u8]) {
        let Some(udp) = self.udp.as_ref() else { return };
        while let Ok((n, from)) = udp.recv_from(buf) {
            // Any datagram from the client teaches us its return address. On a
            // radio that never answered `client udpport` this is the only thing
            // that does — which is why it is learned from *every* datagram and
            // not only from a registration one.
            if self.client_udp != Some(from) {
                self.client_udp = Some(from);
            }
            let Some(h) = p::parse_vita_header(&buf[..n]) else { continue };
            if h.class_code != p::pcc::IF_NARROW || Some(h.stream_id) != self.tx_stream {
                continue;
            }
            let payload = p::vita_payload(&buf[..n], &h);
            let mut frames = 0u64;
            let mut peak = 0f32;
            // Stereo float32, big-endian: take the left channel.
            for frame in payload.chunks_exact(8) {
                let l = f32::from_be_bytes(frame[0..4].try_into().unwrap());
                peak = peak.max(l.abs());
                frames += 1;
            }
            self.stats.tx_frames.fetch_add(frames, Ordering::Relaxed);
            let micro = (peak * 1e6) as u64;
            self.stats.tx_peak_micro.fetch_max(micro, Ordering::Relaxed);
        }
    }

    /// Throw this client off for a duplicate client id, once
    /// [`SimConfig::evict_after`] has elapsed since it registered.
    ///
    /// The notice and the socket close are both the radio's: a client that only
    /// notices the TCP close learns nothing about *why*, and reconnects into the
    /// same fight.
    fn maybe_evict(&mut self) {
        if self.evicted {
            return;
        }
        let Some(after) = self.cfg.evict_after else { return };
        if self.registered_at.is_none_or(|t| t.elapsed() < after) {
            return;
        }
        self.evicted = true;
        let h = self.handle;
        self.status(&format!(
            "client 0x{h:08X} disconnected forced=0 wan_validation_failed=0 \
             duplicate_client_id=1"
        ));
        // Let the notice reach the client before the socket goes.
        std::thread::sleep(Duration::from_millis(50));
        self.stop.store(true, Ordering::Relaxed);
    }

    /// xorshift64* — deterministic, and good enough for a noise floor.
    fn noise(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let v = (self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as i32;
        v as f32 / (1 << 23) as f32 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_payload_parses_back() {
        let cfg = SimConfig::default();
        let info = crate::discovery::parse_broadcast(discovery_payload(&cfg, 4992).as_bytes());
        assert_eq!(info.model, "FLEX-6600");
        assert_eq!(info.nickname, "sdroxide-sim");
        assert_eq!(info.port, 4992);
        assert!(info.joinable());
    }

    #[test]
    fn the_simulator_binds_and_stops_cleanly() {
        let sim = SimRadio::start(SimConfig::default()).expect("start");
        assert!(sim.port() != 0);
        assert!(sim.address().starts_with("127.0.0.1:"));
        // Drop runs the shutdown; a hang here is the failure.
        drop(sim);
    }
}
