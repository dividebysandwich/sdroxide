//! A wire-level Icom, for testing a client without a radio.
//!
//! This is not a mock — nothing is stubbed at a Rust boundary. It binds real
//! UDP sockets, runs the real handshake, negotiates real ports, and streams
//! real PCM and real CI-V frames. A client that talks to it is exercising
//! every byte it would put on the wire to a transceiver.
//!
//! That matters more here than it usually would: this backend was written
//! against a published packet layout and a CI-V manual, with no hardware to
//! check it against. The simulator is what makes "the handshake completes and
//! audio flows" a thing that can be asserted rather than hoped.
//!
//! # What it deliberately does not model
//!
//! Latency, jitter and reordering. Packet *loss* is modelled, because the
//! retransmit path is otherwise unreachable ([`SimOptions::drop_every`]), but
//! a loopback socket delivers in order and instantly, so nothing here proves
//! anything about behaviour on a congested link.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::protocol::{self as proto, AudioCodec, Header, ctrl};

/// How the simulated radio should behave.
#[derive(Debug, Clone)]
pub struct SimOptions {
    pub radio_name: String,
    /// `0xB6` gives an IC-7300MK2 and `0xB2` an IC-7760 — which is worth
    /// reaching for, since it is the model whose scope is neither 475 bins nor
    /// a 0..=160 scale. Anything else exercises the unknown-model path.
    pub civ_address: u8,
    pub username: String,
    pub password: String,
    pub can_transmit: bool,
    /// Emit a sine at this audio frequency. `None` is silence.
    ///
    /// For a 12 kHz IF test, ask for 12 kHz plus the offset the tone should
    /// land at once the client has mixed the IF down.
    pub tone_hz: Option<f64>,
    pub sample_rate: u32,
    /// Send a `27 00` scope sweep about ten times a second.
    pub scope: bool,
    /// Drop every Nth outgoing tracked packet, so the client's retransmit path
    /// is actually reached. 0 never drops.
    pub drop_every: u32,
    /// The dial the simulated radio reports, in Hz.
    pub freq_hz: f64,
    /// Control port. 0 takes an ephemeral one, which is what the tests want so
    /// they can run in parallel; the standalone simulator asks for 50001 so a
    /// client can be pointed at it without being told the port.
    pub port: u16,
    /// Which interface to answer on. Loopback by default — answering on the LAN
    /// puts a phantom radio on somebody else's network, so it takes a flag.
    pub bind: Ipv4Addr,
    /// Ignore everything on the CI-V and audio ports while still naming them in
    /// the status packet. This is what a radio wedged by an earlier session
    /// looks like from outside: control answers, login succeeds, the data ports
    /// are named — and nothing ever answers on them.
    pub mute_data_ports: bool,
    /// Ignore this many set-frequency commands before honouring any, as a
    /// radio whose CI-V datagram was lost on the way looks from the client's
    /// side: acknowledged by nothing, refused by nothing, and still on the old
    /// frequency. This is the shape of issue #297.
    pub ignore_tunes: u32,
    /// Keep reporting USB despite mode writes, to exercise stale/refused modes.
    pub ignore_modes: bool,
    /// What every Set-mode menu item holds before the client writes anything
    /// (`1A 05`). `0x00` is MIC on an IC-7300MK2's modulation inputs, and on no
    /// model is it the value that means LAN — so a test can tell "the client
    /// put this back" from "it was never changed".
    pub menu_default: u8,
}

impl Default for SimOptions {
    fn default() -> Self {
        SimOptions {
            radio_name: "IC-7300MK2".into(),
            civ_address: 0xB6,
            username: "operator".into(),
            password: "hunter2".into(),
            can_transmit: true,
            tone_hz: None,
            sample_rate: 48_000,
            scope: true,
            drop_every: 0,
            freq_hz: 14_074_000.0,
            port: 0,
            bind: Ipv4Addr::LOCALHOST,
            mute_data_ports: false,
            ignore_tunes: 0,
            ignore_modes: false,
            menu_default: 0x00,
        }
    }
}

#[derive(Debug, Default)]
struct Recorded {
    mode: u8,
    /// PCM the client transmitted, decoded to mono.
    tx_audio: Vec<f32>,
    /// CI-V frames the client sent.
    civ: Vec<Vec<u8>>,
    /// Set once the client has completed the stream request.
    streaming: bool,
    /// Set when the client asked for something to be resent.
    served_retransmit: bool,
    /// Set when the client said goodbye on any stream.
    disconnected: bool,
    /// Set when the client gave its token back (the `0x01` token request).
    token_returned: bool,
    /// Set when the client pinged the audio stream — the keepalive that stops
    /// a radio timing out a receive-only session's silent uplink.
    audio_pinged: bool,
    /// Set-mode menu items the client has written, by item number. The radio
    /// keeps these across a session, which is the whole of issue #252: what is
    /// left here when the client goes is what the operator finds on the rig.
    menu: BTreeMap<u16, u8>,
    /// Where the client has tuned the radio, or `None` while it is still on
    /// [`SimOptions::freq_hz`].
    dial: Option<f64>,
}

/// A running simulated radio. Dropping it stops the thread.
pub struct Sim {
    port: u16,
    alive: Arc<AtomicBool>,
    recorded: Arc<Mutex<Recorded>>,
    /// What an item nobody has written holds — see [`SimOptions::menu_default`].
    menu_default: u8,
    /// The dial the radio started on — see [`Sim::dial`].
    start_dial: f64,
    /// Whether the scope is streaming — `27 11` sets it, and a test can clear
    /// it. See [`Sim::stall_scope`].
    scope_out: Arc<AtomicBool>,
    ptt: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Sim {
    /// Bind on the loopback interface and start serving. The control port is
    /// ephemeral so tests can run in parallel.
    pub fn start(opts: SimOptions) -> io::Result<Sim> {
        let control = bind_on(opts.bind, opts.port)?;
        let port = control.local_addr()?.port();
        // The data ports are always ephemeral: the radio names them in its
        // status packet, so nothing needs to predict them.
        let civ = bind_on(opts.bind, 0)?;
        let audio = bind_on(opts.bind, 0)?;
        let alive = Arc::new(AtomicBool::new(true));
        let recorded = Arc::new(Mutex::new(Recorded { mode: 0x01, ..Default::default() }));
        // Off until the client asks for it, as on a real radio: a scope running
        // on the radio's own display streams nothing until `27 11`.
        let scope_out = Arc::new(AtomicBool::new(false));
        let ptt = Arc::new(AtomicBool::new(false));

        let menu_default = opts.menu_default;
        let start_dial = opts.freq_hz;
        let ignore_tunes = opts.ignore_tunes;
        let mut civ = SimStream::new(civ, "civ");
        let mut audio_stream = SimStream::new(audio, "audio");
        civ.mute = opts.mute_data_ports;
        audio_stream.mute = opts.mute_data_ports;
        let radio = SimRadio {
            opts,
            control: SimStream::new(control, "control"),
            civ,
            audio: audio_stream,
            alive: alive.clone(),
            recorded: recorded.clone(),
            scope_out: scope_out.clone(),
            ptt: ptt.clone(),
            token: 0x1234_5678,
            token_request: 0,
            civ_open: false,
            audio_started: None,
            audio_sent: 0,
            next_scope: Instant::now(),
            phase: 0.0,
            drop_counter: 0,
            dial: start_dial,
            ignore_tunes,
        };
        let join =
            std::thread::Builder::new().name("icomnet-sim".into()).spawn(move || radio.run())?;
        Ok(Sim {
            port,
            alive,
            recorded: recorded.clone(),
            menu_default,
            start_dial,
            scope_out,
            ptt,
            join: Some(join),
        })
    }

    /// The control port to point a client at.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Everything the client has transmitted so far.
    pub fn tx_audio(&self) -> Vec<f32> {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).tx_audio.clone()
    }

    /// Every CI-V frame the client has sent.
    pub fn civ_frames(&self) -> Vec<Vec<u8>> {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).civ.clone()
    }

    /// Where the radio's dial is now: what the client last set with `0x05`, or
    /// the frequency it was started on.
    pub fn dial(&self) -> f64 {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).dial.unwrap_or(self.start_dial)
    }

    /// Key the radio locally, without a client CI-V PTT command.
    pub fn set_local_ptt(&self, on: bool) {
        self.ptt.store(on, Ordering::Relaxed);
    }

    /// Whether the client got as far as opening the streams.
    pub fn is_streaming(&self) -> bool {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).streaming
    }

    /// Whether the client noticed a dropped packet and asked for it back.
    pub fn served_retransmit(&self) -> bool {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).served_retransmit
    }

    /// Whether the client said goodbye — on any stream.
    pub fn client_disconnected(&self) -> bool {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).disconnected
    }

    /// Whether the client gave its token back on the way out.
    pub fn token_returned(&self) -> bool {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).token_returned
    }

    /// What Set-mode menu item `item` holds now: the value the client last
    /// wrote into it, or [`SimOptions::menu_default`] if it never did.
    ///
    /// This is the state a radio is *left* in — what the operator walks up to
    /// after the program has gone (issue #252).
    pub fn menu_item(&self, item: u16) -> u8 {
        self.recorded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .menu
            .get(&item)
            .copied()
            .unwrap_or(self.menu_default)
    }

    /// Change a menu item from the radio's front panel.
    pub fn set_menu_item(&self, item: u16, value: u8) {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).menu.insert(item, value);
    }

    /// Whether the client has pinged the audio stream.
    pub fn audio_pinged(&self) -> bool {
        self.recorded.lock().unwrap_or_else(|e| e.into_inner()).audio_pinged
    }

    /// Stop the sweeps without saying so, the way a radio does when its scope
    /// screen closes — and the way a session looks when the `27 11` that should
    /// have started them was lost. Only a fresh enable starts them again, which
    /// is what makes a client's recovery testable.
    pub fn stall_scope(&self) {
        self.scope_out.store(false, Ordering::Relaxed);
    }

    /// Whether the radio is streaming sweeps right now.
    pub fn scope_streaming(&self) -> bool {
        self.scope_out.load(Ordering::Relaxed)
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn bind_on(ip: Ipv4Addr, port: u16) -> io::Result<UdpSocket> {
    let s = UdpSocket::bind(SocketAddrV4::new(ip, port))?;
    s.set_nonblocking(true)?;
    Ok(s)
}

/// What one [`SimStream::poll`] observed besides an application packet.
#[derive(Default)]
struct PollFlags {
    /// The client asked for a retransmit.
    served: bool,
    /// The client said goodbye.
    disconnected: bool,
    /// The client sent a ping probe.
    pinged: bool,
}

/// The radio's half of one stream: enough sequencing to be a credible peer.
struct SimStream {
    sock: UdpSocket,
    peer: Option<SocketAddr>,
    my_id: u32,
    peer_id: u32,
    send_seq: u16,
    sent: VecDeque<(u16, Vec<u8>)>,
    /// Read and discard everything — a port wedged by an earlier session.
    mute: bool,
    tag: &'static str,
}

impl SimStream {
    fn new(sock: UdpSocket, tag: &'static str) -> SimStream {
        let local = sock.local_addr().ok();
        let port = local.map(|a| a.port()).unwrap_or(0);
        let ip = match local {
            Some(SocketAddr::V4(v4)) => *v4.ip(),
            _ => Ipv4Addr::LOCALHOST,
        };
        SimStream {
            sock,
            peer: None,
            my_id: proto::session_id(ip, port),
            peer_id: 0,
            send_seq: 0,
            sent: VecDeque::new(),
            mute: false,
            tag,
        }
    }

    fn port(&self) -> u16 {
        self.sock.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    fn write(&self, pkt: &[u8]) {
        if let Some(p) = self.peer {
            let _ = self.sock.send_to(pkt, p);
        }
    }

    /// Send with a sequence number, keeping a copy so a retransmit request can
    /// be answered. `drop_it` simulates the packet never arriving — the copy is
    /// still kept, which is exactly what a real radio does.
    fn send_tracked(&mut self, mut pkt: Vec<u8>, drop_it: bool) {
        pkt[6..8].copy_from_slice(&self.send_seq.to_le_bytes());
        if !drop_it {
            self.write(&pkt);
        }
        if self.sent.len() >= 500 {
            self.sent.pop_front();
        }
        self.sent.push_back((self.send_seq, pkt));
        self.send_seq = self.send_seq.wrapping_add(1);
    }

    fn control(&self, kind: u16, seq: u16) -> Vec<u8> {
        proto::control(kind, seq, self.my_id, self.peer_id)
    }

    /// Deal with the handshake and reliability layer. Returns an application
    /// packet for the caller when there is one.
    fn poll(&mut self, out: &mut Vec<u8>, flags: &mut PollFlags) -> bool {
        let mut buf = [0u8; 2048];
        let (n, from) = match self.sock.recv_from(&mut buf) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if self.mute {
            // Drained but never answered: the wedged-port case.
            return false;
        }
        self.peer = Some(from);
        let pkt = &buf[..n];
        let Some(h) = Header::parse(pkt) else { return false };
        if h.sent_id != 0 {
            self.peer_id = h.sent_id;
        }

        match h.kind {
            ctrl::ARE_YOU_THERE => {
                let p = self.control(ctrl::I_AM_HERE, 0);
                self.write(&p);
                tracing::trace!(target: "icomnet::sim", stream = self.tag, "are you there → I am here");
            }
            ctrl::READY => {
                let p = self.control(ctrl::READY, 0);
                self.write(&p);
            }
            ctrl::PING => {
                if let Some(ping) = proto::parse_ping(pkt).filter(|p| !p.reply) {
                    flags.pinged = true;
                    let r = proto::ping(ping.seq, true, ping.time, self.my_id, self.peer_id);
                    self.write(&r);
                }
            }
            ctrl::RETRANSMIT => {
                flags.served = true;
                let mut wanted = Vec::new();
                if n == proto::CONTROL_SIZE {
                    wanted.push(h.seq);
                } else {
                    let mut at = proto::HEADER;
                    while at + 4 <= n {
                        let first = proto::u16_at(pkt, at);
                        let last = proto::u16_at(pkt, at + 2);
                        for i in 0..=last.wrapping_sub(first).min(64) {
                            wanted.push(first.wrapping_add(i));
                        }
                        at += 4;
                    }
                }
                for seq in wanted {
                    if let Some((_, d)) = self.sent.iter().find(|(s, _)| *s == seq) {
                        let d = d.clone();
                        self.write(&d);
                    }
                }
            }
            ctrl::DISCONNECT if n == proto::CONTROL_SIZE => {
                flags.disconnected = true;
            }
            ctrl::IDLE if n == proto::CONTROL_SIZE => {}
            _ => {
                // A real radio drops a payload addressed to a session id it
                // never issued, and a client that writes into a stream before
                // the handshake has finished loses everything it sent. The
                // simulator has to be just as unforgiving: accepting those
                // frames hid exactly that bug — one that never showed on
                // loopback and always showed on a radio across a WiFi hop.
                if n > proto::CONTROL_SIZE && h.rcvd_id == self.my_id {
                    out.clear();
                    out.extend_from_slice(pkt);
                    return true;
                }
            }
        }
        false
    }
}

struct SimRadio {
    opts: SimOptions,
    control: SimStream,
    civ: SimStream,
    audio: SimStream,
    alive: Arc<AtomicBool>,
    recorded: Arc<Mutex<Recorded>>,
    token: u32,
    token_request: u16,
    civ_open: bool,
    scope_out: Arc<AtomicBool>,
    ptt: Arc<AtomicBool>,
    audio_started: Option<Instant>,
    audio_sent: u64,
    next_scope: Instant,
    phase: f64,
    drop_counter: u32,
    /// The dial, which `0x05` moves. A simulator whose frequency is a constant
    /// cannot show a tune failing to stick, which is the one thing issues #285
    /// and #297 were about.
    dial: f64,
    /// Tunes still to be swallowed — see [`SimOptions::ignore_tunes`].
    ignore_tunes: u32,
}

impl SimRadio {
    fn run(mut self) {
        let mut buf = Vec::<u8>::with_capacity(2048);
        while self.alive.load(Ordering::Relaxed) {
            let mut worked = false;
            let mut flags = PollFlags::default();

            while self.control.poll(&mut buf, &mut flags) {
                worked = true;
                let pkt = std::mem::take(&mut buf);
                self.on_control(&pkt);
                buf = pkt;
            }
            while self.civ.poll(&mut buf, &mut flags) {
                worked = true;
                if let Some(frame) = proto::parse_civ_data(&buf) {
                    let frame = frame.to_vec();
                    self.record(|r| r.civ.push(frame.clone()));
                    self.on_civ(&frame);
                } else if buf.len() == proto::OPENCLOSE_SIZE {
                    self.civ_open = buf[0x15] == 0x04;
                }
            }
            // An open/close packet is the same length as a retransmit-range
            // packet, so it arrives through the application path above only
            // when it is long enough; catch the short form here.
            let mut audio_flags = PollFlags::default();
            while self.audio.poll(&mut buf, &mut audio_flags) {
                worked = true;
                if let Some(pcm) = proto::parse_audio_data(&buf) {
                    let mut decoded = Vec::new();
                    proto::decode_pcm(AudioCodec::Lpcm16Mono, pcm, &mut decoded);
                    self.record(|r| r.tx_audio.extend_from_slice(&decoded));
                }
            }
            if audio_flags.pinged {
                self.record(|r| r.audio_pinged = true);
            }
            flags.served |= audio_flags.served;
            flags.disconnected |= audio_flags.disconnected;
            if flags.served {
                self.record(|r| r.served_retransmit = true);
            }
            if flags.disconnected {
                self.record(|r| r.disconnected = true);
            }

            if self.audio_started.is_some() {
                self.pump_audio();
                self.pump_scope();
            }
            if !worked {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    fn record(&self, f: impl FnOnce(&mut Recorded)) {
        f(&mut self.recorded.lock().unwrap_or_else(|e| e.into_inner()));
    }

    fn on_control(&mut self, pkt: &[u8]) {
        match pkt.len() {
            proto::LOGIN_SIZE => {
                let user_ok = pkt[0x40..0x50] == proto::passcode(&self.opts.username);
                let pass_ok = pkt[0x50..0x60] == proto::passcode(&self.opts.password);
                self.token_request = proto::u16_at(pkt, 0x1a);
                let mut r = vec![0u8; proto::LOGIN_RESPONSE_SIZE];
                r[0..4].copy_from_slice(&(proto::LOGIN_RESPONSE_SIZE as u32).to_le_bytes());
                r[8..12].copy_from_slice(&self.control.my_id.to_le_bytes());
                r[12..16].copy_from_slice(&self.control.peer_id.to_le_bytes());
                r[0x1a..0x1c].copy_from_slice(&self.token_request.to_le_bytes());
                r[0x1c..0x20].copy_from_slice(&self.token.to_le_bytes());
                let err: u32 =
                    if user_ok && pass_ok { 0 } else { proto::LoginResponse::BAD_CREDENTIALS };
                r[0x30..0x34].copy_from_slice(&err.to_le_bytes());
                r[0x40..0x44].copy_from_slice(b"FTTH");
                let drop_it = self.should_drop();
                self.control.send_tracked(r, drop_it);
            }
            proto::TOKEN_SIZE => {
                let magic = pkt[0x15];
                if magic == 0x01 {
                    // The client is giving its token back on the way out.
                    self.record(|r| r.token_returned = true);
                } else if magic == 0x02 {
                    // The token is claimed; introduce ourselves.
                    let caps = self.capabilities();
                    let drop_it = self.should_drop();
                    self.control.send_tracked(caps, drop_it);
                } else if magic == 0x05 {
                    let mut r = proto::token(
                        0x05,
                        0,
                        self.token_request,
                        self.token,
                        self.control.my_id,
                        self.control.peer_id,
                    );
                    r[0x14] = 0x02; // reply
                    r[0x30..0x34].copy_from_slice(&0u32.to_le_bytes());
                    let drop_it = self.should_drop();
                    self.control.send_tracked(r, drop_it);
                }
            }
            proto::CONNINFO_SIZE => {
                // A stream request: answer with the data ports.
                let mut r = vec![0u8; proto::STATUS_SIZE];
                r[0..4].copy_from_slice(&(proto::STATUS_SIZE as u32).to_le_bytes());
                r[8..12].copy_from_slice(&self.control.my_id.to_le_bytes());
                r[12..16].copy_from_slice(&self.control.peer_id.to_le_bytes());
                r[0x42..0x44].copy_from_slice(&self.civ.port().to_be_bytes());
                r[0x46..0x48].copy_from_slice(&self.audio.port().to_be_bytes());
                let drop_it = self.should_drop();
                self.control.send_tracked(r, drop_it);
                self.audio_started = Some(Instant::now());
                self.audio_sent = 0;
                self.record(|x| x.streaming = true);
            }
            _ => {}
        }
    }

    /// The capabilities packet, with this radio as its only entry.
    fn capabilities(&self) -> Vec<u8> {
        let len = proto::CAPABILITIES_SIZE + proto::RADIO_CAP_SIZE;
        let mut p = vec![0u8; len];
        p[0..4].copy_from_slice(&(len as u32).to_le_bytes());
        p[8..12].copy_from_slice(&self.control.my_id.to_le_bytes());
        p[12..16].copy_from_slice(&self.control.peer_id.to_le_bytes());
        p[0x40..0x42].copy_from_slice(&1u16.to_le_bytes());

        let c = proto::CAPABILITIES_SIZE;
        p[c + 0x07..c + 0x09].copy_from_slice(&proto::RadioCap::CAP_MAC.to_be_bytes());
        p[c + 0x0a..c + 0x10].copy_from_slice(&[0x00, 0x90, 0xc7, 0x01, 0x02, 0x03]);
        let name = self.opts.radio_name.as_bytes();
        let n = name.len().min(32);
        p[c + 0x10..c + 0x10 + n].copy_from_slice(&name[..n]);
        p[c + 0x30..c + 0x34].copy_from_slice(b"ICOM");
        p[c + 0x52] = self.opts.civ_address;
        p[c + 0x53..c + 0x55].copy_from_slice(&0x000fu16.to_be_bytes());
        let tx: u16 = if self.opts.can_transmit { 0x000f } else { 0 };
        p[c + 0x55..c + 0x57].copy_from_slice(&tx.to_be_bytes());
        p[c + 0x5a..c + 0x5e].copy_from_slice(&115_200u32.to_be_bytes());
        p
    }

    /// Answer the CI-V queries a client actually makes at startup.
    fn on_civ(&mut self, frame: &[u8]) {
        // FE FE <to> <from> <cmd> … FD
        if frame.len() < 6 || frame[0] != 0xfe || frame[1] != 0xfe {
            return;
        }
        let cmd = frame[4];
        let addr = self.opts.civ_address;
        let reply = |body: Vec<u8>| {
            let mut f = vec![0xfe, 0xfe, 0xe0, addr];
            f.extend_from_slice(&body);
            f.push(0xfd);
            f
        };
        let out = match cmd {
            // Read frequency.
            0x03 => Some(reply({
                let mut b = vec![0x03];
                b.extend_from_slice(&bcd_freq(self.dial));
                b
            })),
            // Set frequency: the radio goes there and says so from then on.
            0x05 => {
                if self.ignore_tunes > 0 {
                    self.ignore_tunes -= 1;
                    return;
                }
                if let Some(hz) = unbcd_freq(&frame[5..frame.len() - 1]) {
                    self.dial = hz;
                    self.recorded.lock().unwrap_or_else(|e| e.into_inner()).dial = Some(hz);
                }
                Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
            }
            0x04 => {
                let mode = self.recorded.lock().unwrap_or_else(|e| e.into_inner()).mode;
                Some(reply(vec![0x04, mode, 0x01]))
            }
            0x06 if frame.len() >= 7 => {
                if !self.opts.ignore_modes {
                    self.recorded.lock().unwrap_or_else(|e| e.into_inner()).mode = frame[5];
                }
                Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
            }
            0x15 => match frame.get(5) {
                // S-meter, mid scale.
                Some(0x02) => Some(reply(vec![0x15, 0x02, 0x01, 0x20])),
                // SWR, bottom of scale.
                Some(0x12) => Some(reply(vec![0x15, 0x12, 0x00, 0x00])),
                // ALC, half of 255.
                Some(0x13) => Some(reply(vec![0x15, 0x13, 0x01, 0x28])),
                // Power output, full scale.
                Some(0x11) => Some(reply(vec![0x15, 0x11, 0x02, 0x13])),
                _ => None,
            },
            0x1c if frame.get(5) == Some(&0x00) && frame.len() >= 7 => {
                match &frame[6..frame.len() - 1] {
                    [] => Some(reply(vec![0x1c, 0x00, u8::from(self.ptt.load(Ordering::Relaxed))])),
                    [on @ (0 | 1)] => {
                        self.ptt.store(*on != 0, Ordering::Relaxed);
                        Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
                    }
                    _ => None,
                }
            }
            // The Set-mode menu, `1A 05 <hi> <lo> [value]`. A bare item number
            // is a question and is answered with what the item holds; anything
            // after it is a write, and the radio keeps it — which is what lets
            // a test see what state the client left the rig in (issue #252).
            0x1a if frame.get(5) == Some(&0x06) && frame.len() >= 7 => {
                let mut recorded = self.recorded.lock().unwrap_or_else(|e| e.into_inner());
                if frame.len() == 7 {
                    Some(reply(vec![0x1a, 0x06, recorded.mode, 0x01]))
                } else {
                    recorded.mode = frame[6];
                    Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
                }
            }
            0x1a if frame.get(5) == Some(&0x05) && frame.len() >= 9 => {
                let item = (u16::from(frame[6]) << 8) | u16::from(frame[7]);
                let value = &frame[8..frame.len() - 1];
                match value.first() {
                    Some(&v) => {
                        self.recorded
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .menu
                            .insert(item, v);
                        Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
                    }
                    None => {
                        let held = self.menu_item(item);
                        Some(reply(vec![0x1a, 0x05, frame[6], frame[7], held]))
                    }
                }
            }
            // Anything we are told to set is simply acknowledged.
            0x06 | 0x0f | 0x14 | 0x16 | 0x1a | 0x1c | 0x21 | 0x27 => {
                Some(vec![0xfe, 0xfe, 0xe0, addr, 0xfb, 0xfd])
            }
            _ => None,
        };
        if let Some(f) = out {
            self.send_civ(f);
        }
        // `27 11` starts the sweeps, and stops them again on `00`.
        if cmd == 0x27 && frame.get(5) == Some(&0x11) {
            self.scope_out.store(frame.get(6) == Some(&0x01), Ordering::Relaxed);
            self.next_scope = Instant::now();
        }
    }

    /// What a Set-mode menu item holds: whatever the client last wrote there,
    /// or the radio's own starting value.
    fn menu_item(&self, item: u16) -> u8 {
        self.recorded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .menu
            .get(&item)
            .copied()
            .unwrap_or(self.opts.menu_default)
    }

    fn send_civ(&mut self, frame: Vec<u8>) {
        let p = proto::civ_data(&frame, 0, self.civ.my_id, self.civ.peer_id);
        let drop_it = self.should_drop();
        self.civ.send_tracked(p, drop_it);
    }

    /// One `27 00` sweep, in the single-frame form a LAN Icom sends.
    fn pump_scope(&mut self) {
        if !self.opts.scope
            || !self.civ_open
            || !self.scope_out.load(Ordering::Relaxed)
            || Instant::now() < self.next_scope
        {
            return;
        }
        self.next_scope = Instant::now() + Duration::from_millis(100);
        let addr = self.opts.civ_address;
        let mut f = vec![0xfe, 0xfe, 0xe0, addr, 0x27, 0x00];
        f.push(0x00); // selected VFO
        f.push(0x01); // division 1
        f.push(0x01); // of 1 — the LAN form
        f.push(0x00); // centre mode
        f.extend_from_slice(&bcd_freq(self.dial));
        f.extend_from_slice(&bcd_freq(50_000.0)); // ±50 kHz span
        f.push(0x00); // in range
        // As many bins, on the scale, this model actually sweeps: an IC-7760
        // sends 689 on a 0..=200 scale where the IC-7300 generation sends 475
        // on 0..=160. A simulator that always sent the one shape would let a
        // client hard-code it — see the note in `protocol::Model`.
        let model = proto::model_for_radio(&self.opts.radio_name, addr);
        let bins = model.scope_bins as i32;
        let full = i32::from(model.scope_full_scale);
        let peak = bins / 2;
        // A noise floor with a peak in the middle, so a client can tell it
        // apart from a flat buffer.
        for i in 0..bins {
            let v = if (i - peak).abs() < 4 { full * 15 / 16 } else { 20 + i % 7 };
            f.push(v.clamp(0, full) as u8);
        }
        f.push(0xfd);
        self.send_civ(f);
    }

    /// Pace PCM off a wall clock so the client sees a real-time stream.
    fn pump_audio(&mut self) {
        let Some(start) = self.audio_started else { return };
        let rate = f64::from(self.opts.sample_rate);
        let due = (start.elapsed().as_secs_f64() * rate) as u64;
        if due <= self.audio_sent {
            return;
        }
        // One packet at a time; the loop comes back round immediately.
        let want = ((due - self.audio_sent) as usize).min(proto::AUDIO_CHUNK / 2);
        let mut samples = Vec::with_capacity(want);
        match self.opts.tone_hz {
            Some(hz) => {
                let step = std::f64::consts::TAU * hz / rate;
                for _ in 0..want {
                    samples.push((self.phase.sin() * 0.5) as f32);
                    self.phase += step;
                    if self.phase > std::f64::consts::TAU {
                        self.phase -= std::f64::consts::TAU;
                    }
                }
            }
            None => samples.resize(want, 0.0),
        }
        self.audio_sent += want as u64;
        let mut pcm = Vec::with_capacity(want * 2);
        proto::encode_pcm(AudioCodec::Lpcm16Mono, &samples, &mut pcm);
        let p = proto::audio_data(&pcm, 0, self.audio.my_id, self.audio.peer_id);
        let drop_it = self.should_drop();
        self.audio.send_tracked(p, drop_it);
    }

    fn should_drop(&mut self) -> bool {
        if self.opts.drop_every == 0 {
            return false;
        }
        self.drop_counter += 1;
        self.drop_counter.is_multiple_of(self.opts.drop_every)
    }
}

/// Icom's 5-byte little-endian BCD frequency, as used by both `03` and the
/// scope sweep's edge and span fields.
fn bcd_freq(hz: f64) -> [u8; 5] {
    let mut v = hz.max(0.0).round() as u64;
    let mut out = [0u8; 5];
    for byte in out.iter_mut() {
        let lo = (v % 10) as u8;
        v /= 10;
        let hi = (v % 10) as u8;
        v /= 10;
        *byte = lo | (hi << 4);
    }
    out
}

/// The inverse of [`bcd_freq`]: five little-endian BCD bytes back to Hz.
fn unbcd_freq(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 5 {
        return None;
    }
    let mut hz: u64 = 0;
    for &b in bytes[..5].iter().rev() {
        let (hi, lo) = ((b >> 4) as u64, (b & 0x0f) as u64);
        if hi > 9 || lo > 9 {
            return None;
        }
        hz = hz * 100 + hi * 10 + lo;
    }
    Some(hz as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tune_moves_the_simulated_dial_and_comes_back_out_again() {
        assert_eq!(unbcd_freq(&bcd_freq(7_074_000.0)), Some(7_074_000.0));
        assert_eq!(unbcd_freq(&[0x00, 0x00]), None, "a truncated frame is not a frequency");
        assert_eq!(unbcd_freq(&[0xff, 0x00, 0x00, 0x00, 0x00]), None, "nor is a bad nibble");
    }

    #[test]
    fn bcd_matches_icoms_little_endian_nibble_order() {
        // 14.074 MHz → 00 40 07 14 00
        assert_eq!(bcd_freq(14_074_000.0), [0x00, 0x40, 0x07, 0x14, 0x00]);
        assert_eq!(bcd_freq(50_000.0), [0x00, 0x00, 0x05, 0x00, 0x00]);
    }

    #[test]
    fn the_simulator_binds_and_stops_cleanly() {
        let sim = Sim::start(SimOptions::default()).unwrap();
        assert_ne!(sim.port(), 0);
        assert!(!sim.is_streaming());
        sim.stop();
    }
}
