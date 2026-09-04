//! The TCI WebSocket client: one blocking thread per *connection* that streams
//! each attached receiver's IQ into its own ring, sends TX audio from a ring
//! (paced by the engine's fill rate / `TxChrono`), and carries
//! frequency/mode/PTT control. Mirrors the `sdroxide-hpsdr` net thread.
//!
//! A TCI rig can carry several independent receivers (a SunSDR2DX reports
//! `trx_count:2`), and sdroxide runs each as a radio of its own — so the
//! connection is split from the streams: [`TciDevice`] owns the WebSocket and
//! the thread, and vends one [`TciRx`] per receiver. Streams attach and detach
//! at runtime (a radio tab opening or closing) without disturbing the others;
//! the connection closes when the last handle to it drops. The transmitter,
//! the audio stream and the power levels belong to receiver 0 throughout —
//! TCI has one transmitter however many receivers it has.

use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};
use sdroxide_types::{Mode, TxTelemetry};
use tungstenite::{Message, WebSocket};

use crate::protocol as p;
use crate::split_addr;

/// TX audio rate (TCI audio streams are 48 kHz).
pub const TX_RATE_HZ: u32 = 48_000;
/// The receiver that owns the transmitter, the audio stream and the power
/// levels. TCI's `trx`/`drive`/`audio_*` take a receiver index, but the rig
/// has one transmitter — receiver 0's.
const TX_RX: u32 = 0;
/// The VFO channel we drive (A).
const CH: u32 = 0;

#[derive(Debug, thiserror::Error)]
pub enum TciError {
    #[error("{0}")]
    Msg(String),
}

/// A frequency, mode or power change reported by the rig (for two-way sync).
#[derive(Debug, Clone)]
pub enum TciUpdate {
    Freq(f64),
    Mode(Mode),
    /// TX drive as a 0..1 fraction of the rig's `drive:` percentage.
    Drive(f32),
    /// TUNE drive as a 0..1 fraction of the rig's `tune_drive:` percentage.
    TuneDrive(f32),
}

/// Control messages to the WebSocket thread.
enum Ctrl {
    /// A receiver stream coming up: store its ring + update channel, then ask
    /// the rig to enable it and start its IQ.
    Attach {
        rx: u32,
        ring: Producer<f32>,
        updates: Sender<TciUpdate>,
    },
    /// A receiver stream going away (its radio tab closed, or its source is
    /// being rebuilt). Receiver 0 keeps its `rx_enable` — it is the rig's own
    /// main receiver — the others are switched off entirely.
    Detach {
        rx: u32,
    },
    SetCenter {
        rx: u32,
        hz: f64,
    },
    /// Offset (Hz) of the operator's VFO from the IQ centre — keeps the rig's
    /// own VFO on our dial.
    SetIf {
        rx: u32,
        hz: f64,
    },
    SetMode {
        rx: u32,
        mode: Mode,
    },
    TxOn(f64),
    TxOff,
    /// TX drive / tune-drive as a 0..1 fraction.
    SetDrive(f64),
    SetTuneDrive(f64),
    Shutdown,
}

type Ws = WebSocket<TcpStream>;

fn send_text(ws: &mut Ws, s: String) {
    let _ = ws.write(Message::Text(s.into()));
}

/// What every stream of one connection shares. Dropping the last handle shuts
/// the thread down and closes the WebSocket.
struct DevInner {
    ctrl: Sender<Ctrl>,
    /// Cleared when the streaming thread exits (the rig closed the socket, or
    /// a read failed), so owners can tell a live link from a dead one.
    alive: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
    device: String,
    sample_rate_hz: f64,
    /// Independent receivers the rig announced (`trx_count:`). 1 when the
    /// server never said — every server names what it has, and refusing a
    /// receiver it never offered beats streaming silence from it.
    trx_count: u32,
    /// The TX endpoints, claimable exactly once — by receiver 0's stream.
    tx_endpoint: Mutex<Option<(Producer<f32>, Receiver<TxTelemetry>)>>,
    /// Which receivers have a live [`TciRx`], so one cannot be vended twice:
    /// two engines draining one ring would each get half the samples.
    attached: Mutex<std::collections::HashSet<u32>>,
}

impl Drop for DevInner {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.lock().expect("join lock").take() {
            let _ = j.join();
        }
    }
}

/// A live TCI connection, shared by every receiver stream on it. Cheap to
/// clone; the WebSocket closes when the last clone (and every [`TciRx`]) is
/// gone.
#[derive(Clone)]
pub struct TciDevice {
    inner: Arc<DevInner>,
}

impl TciDevice {
    /// Connect to `address` (`host:port`) and negotiate the IQ rate. No
    /// streams yet — each receiver's starts when [`TciDevice::rx`] vends it.
    pub fn connect(address: &str, iq_rate_hz: f64) -> Result<TciDevice, TciError> {
        let (host, port) = split_addr(address).map_err(TciError::Msg)?;
        let sockaddr = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| TciError::Msg(format!("resolve {host}:{port}: {e}")))?
            .next()
            .ok_or_else(|| TciError::Msg(format!("no address for {host}:{port}")))?;
        let stream = crate::connect_socket(&sockaddr, &host, port, Duration::from_secs(3))
            .map_err(TciError::Msg)?;
        // The upgrade's whole budget in one read — the rig can take tens of
        // milliseconds to answer while it is busy with its own DSP, and on
        // Windows a read that expires mid-upgrade cannot be resumed from (see
        // `ws_handshake`) — then the short streaming timeout once the socket
        // is a WebSocket.
        stream
            .set_read_timeout(Some(crate::HANDSHAKE_TIMEOUT))
            .map_err(|e| TciError::Msg(e.to_string()))?;
        let url = format!("ws://{host}:{port}/");
        let mut ws =
            crate::ws_handshake(stream, url.as_str(), Instant::now() + crate::HANDSHAKE_TIMEOUT)
                .map_err(TciError::Msg)?;
        ws.get_ref()
            .set_read_timeout(Some(Duration::from_millis(20)))
            .map_err(|e| TciError::Msg(e.to_string()))?;

        // Read the status burst until `ready;` (or time out). The burst
        // carries the rig's current settings: the receiver count that decides
        // what [`TciDevice::rx`] may vend, and the power levels we adopt
        // rather than overwrite (see `TciUpdate::Drive`).
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut device = String::new();
        let mut trx_count = 1u32;
        let mut drive_pct = None;
        let mut tune_drive_pct = None;
        loop {
            if Instant::now() > deadline {
                return Err(TciError::Msg("timed out waiting for 'ready;'".into()));
            }
            match ws.read() {
                Ok(Message::Text(t)) => {
                    let mut ready = false;
                    for (cmd, args) in p::parse_status(t.as_str()) {
                        match cmd.as_str() {
                            "device" => device = args,
                            "trx_count" => {
                                trx_count = args.trim().parse().unwrap_or(1).max(1);
                            }
                            "drive" => drive_pct = parse_pct(&args).or(drive_pct),
                            "tune_drive" => tune_drive_pct = parse_pct(&args).or(tune_drive_pct),
                            "ready" => ready = true,
                            _ => {}
                        }
                    }
                    if ready {
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(TciError::Msg("server closed during handshake".into()));
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(TciError::Msg(format!("handshake read: {e}"))),
            }
        }

        // The IQ rate is a property of the connection — TCI has one
        // `iq_samplerate` for every stream — so it is set here, once, and
        // every receiver that attaches later runs at it. The TX-audio path
        // rides the (separate) audio stream; its rate goes up front too, and
        // the stream itself is started only while transmitting. TX telemetry
        // (forward power + SWR) is off by default in ExpertSDR2/TCI, so
        // subscribe now.
        let iq_rate = iq_rate_hz.round() as u32;
        send_text(&mut ws, p::iq_samplerate(iq_rate));
        send_text(&mut ws, p::audio_samplerate(TX_RATE_HZ));
        send_text(&mut ws, p::tx_sensors_enable(true));
        let _ = ws.flush();

        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(1 << 15);
        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let (telem_tx, telem_rx) = crossbeam_channel::unbounded();

        // Adopt the rig's own power settings instead of imposing ours:
        // whatever the operator set in ExpertSDR3 is what the first key-down
        // should use. Queued in the thread until receiver 0 attaches — the
        // updates channel they land on does not exist yet.
        let mut adopted = Vec::new();
        if let Some(pct) = drive_pct {
            adopted.push(TciUpdate::Drive(pct as f32 / 100.0));
        }
        if let Some(pct) = tune_drive_pct {
            adopted.push(TciUpdate::TuneDrive(pct as f32 / 100.0));
        }

        let thread = NetThread {
            ws,
            slots: HashMap::new(),
            tx: tx_cons,
            ctrl: ctrl_rx,
            telem: telem_tx,
            ptt: false,
            tx_pkts: 0,
            tx_fwd_w: None,
            tx_swr: None,
            drive_pct,
            tune_drive_pct,
            adopted,
            tx_on_at: None,
            chrono_seen: false,
            tx_primed: false,
        };
        let alive = Arc::new(AtomicBool::new(true));
        let thread_alive = Arc::clone(&alive);
        let join = std::thread::Builder::new()
            .name("sdroxide-tci".into())
            .spawn(move || {
                thread.run();
                thread_alive.store(false, Ordering::Relaxed);
            })
            .map_err(|e| TciError::Msg(format!("spawn thread: {e}")))?;

        Ok(TciDevice {
            inner: Arc::new(DevInner {
                ctrl: ctrl_tx,
                alive,
                join: Mutex::new(Some(join)),
                device,
                sample_rate_hz: iq_rate_hz,
                trx_count,
                tx_endpoint: Mutex::new(Some((tx_prod, telem_rx))),
                attached: Mutex::new(std::collections::HashSet::new()),
            }),
        })
    }

    /// Whether the streaming thread is still running. It stops when the rig
    /// closes the WebSocket (ExpertSDR3 shut down) or a read fails, at which
    /// point this connection is dead and only a fresh [`TciDevice::connect`]
    /// revives it.
    pub fn is_alive(&self) -> bool {
        self.inner.alive.load(Ordering::Relaxed)
    }

    /// The rig's name from the connect burst ("SunSDR2DX", "Thetis", …).
    pub fn device(&self) -> &str {
        &self.inner.device
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.inner.sample_rate_hz
    }

    /// Independent receivers the rig announced.
    pub fn trx_count(&self) -> u32 {
        self.inner.trx_count
    }

    /// Attach receiver `rx` and start its IQ stream. Refused for a receiver
    /// the rig never announced, and for one that already has a live stream —
    /// two engines draining one ring would each get half the samples.
    pub fn rx(&self, rx: u32) -> Result<TciRx, TciError> {
        if rx >= self.inner.trx_count {
            return Err(TciError::Msg(format!(
                "receiver {} does not exist: {} reports {} receiver(s)",
                rx + 1,
                self.inner.device,
                self.inner.trx_count
            )));
        }
        {
            let mut attached = self.inner.attached.lock().expect("attached lock");
            if !attached.insert(rx) {
                return Err(TciError::Msg(format!(
                    "receiver {} is already running as another radio",
                    rx + 1
                )));
            }
        }
        let rx_cap =
            ((self.inner.sample_rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (ring_prod, ring_cons) = RingBuffer::<f32>::new(rx_cap);
        let (upd_tx, upd_rx) = crossbeam_channel::unbounded();
        // Receiver 0's stream owns the transmitter; there is exactly one set
        // of TX endpoints, so a second rx-0 stream (already refused above)
        // could never claim them anyway.
        let (tx, telem) = if rx == TX_RX {
            match self.inner.tx_endpoint.lock().expect("tx lock").take() {
                Some((t, m)) => (Some(t), Some(m)),
                None => (None, None),
            }
        } else {
            (None, None)
        };
        let _ = self.inner.ctrl.send(Ctrl::Attach { rx, ring: ring_prod, updates: upd_tx });
        Ok(TciRx { dev: Arc::clone(&self.inner), rx, ring: ring_cons, updates: upd_rx, tx, telem })
    }
}

/// One receiver's stream on a shared [`TciDevice`]: its IQ ring, its VFO and
/// mode, and — on receiver 0 — the transmitter. Dropping it stops this
/// stream; the connection lives on for whoever else holds it.
pub struct TciRx {
    dev: Arc<DevInner>,
    rx: u32,
    ring: Consumer<f32>,
    updates: Receiver<TciUpdate>,
    /// TX audio into the net thread — receiver 0 only.
    tx: Option<Producer<f32>>,
    /// TX telemetry (SWR / forward power) — receiver 0 only.
    telem: Option<Receiver<TxTelemetry>>,
}

impl std::fmt::Debug for TciRx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TciRx")
            .field("rx", &self.rx)
            .field("device", &self.dev.device)
            .finish_non_exhaustive()
    }
}

impl Drop for TciRx {
    fn drop(&mut self) {
        self.dev.attached.lock().expect("attached lock").remove(&self.rx);
        // Hand the TX endpoints back so a rebuilt receiver-0 stream (Settings
        // → Apply) can claim them again on the same connection.
        if let (Some(tx), Some(telem)) = (self.tx.take(), self.telem.take()) {
            *self.dev.tx_endpoint.lock().expect("tx lock") = Some((tx, telem));
        }
        let _ = self.dev.ctrl.send(Ctrl::Detach { rx: self.rx });
    }
}

impl TciRx {
    /// Which receiver this stream is (0-based, as the wire counts them).
    pub fn rx_index(&self) -> u32 {
        self.rx
    }

    /// See [`TciDevice::is_alive`].
    pub fn is_alive(&self) -> bool {
        self.dev.alive.load(Ordering::Relaxed)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.dev.sample_rate_hz
    }

    pub fn device(&self) -> &str {
        &self.dev.device
    }

    /// Retune: set this receiver's DDC center (its IQ stream centers here).
    pub fn set_center(&self, hz: f64) {
        let _ = self.dev.ctrl.send(Ctrl::SetCenter { rx: self.rx, hz });
    }
    /// Keep the rig's VFO `hz` above the IQ centre (our software DDC offset).
    pub fn set_if_offset(&self, hz: f64) {
        let _ = self.dev.ctrl.send(Ctrl::SetIf { rx: self.rx, hz });
    }
    pub fn set_mode(&self, mode: Mode) {
        let _ = self.dev.ctrl.send(Ctrl::SetMode { rx: self.rx, mode });
    }

    /// Begin transmitting at `tx_freq_hz`; returns the TX audio rate. The
    /// transmitter is receiver 0's — a no-op on any other stream, which the
    /// engine never asks anyway (their capabilities carry no TX).
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        if self.tx.is_some() {
            let _ = self.dev.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        }
        TX_RATE_HZ as f64
    }
    pub fn tx_end(&self) {
        if self.tx.is_some() {
            let _ = self.dev.ctrl.send(Ctrl::TxOff);
        }
    }

    /// Set TX drive (`0..1`) — mapped to the rig's `drive:` percentage.
    pub fn set_drive(&self, frac: f64) {
        if self.tx.is_some() {
            let _ = self.dev.ctrl.send(Ctrl::SetDrive(frac));
        }
    }
    /// Set TUNE drive (`0..1`) — mapped to the rig's `tune_drive:` percentage.
    pub fn set_tune_drive(&self, frac: f64) {
        if self.tx.is_some() {
            let _ = self.dev.ctrl.send(Ctrl::SetTuneDrive(frac));
        }
    }

    /// Push mono 48 kHz TX audio, with bounded back-pressure.
    pub fn tx_write(&mut self, audio: &[f32]) {
        let Some(tx) = self.tx.as_mut() else { return };
        for &v in audio {
            let mut val = v;
            let mut tries = 0u32;
            loop {
                match tx.push(val) {
                    Ok(()) => break,
                    Err(rtrb::PushError::Full(x)) => {
                        if tries > 2000 {
                            return;
                        }
                        tries += 1;
                        val = x;
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
            }
        }
    }

    /// TX audio still queued for the rig (samples). Zero once everything the
    /// engine wrote has gone out on the wire.
    pub fn tx_pending(&self) -> usize {
        self.tx.as_ref().map_or(0, |tx| tx.buffer().capacity() - tx.slots())
    }

    /// Drain interleaved I,Q floats from the RX ring (always an even count).
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        let take = self.ring.slots().min(out.len()) & !1;
        let mut n = 0;
        while n < take {
            match self.ring.pop() {
                Ok(v) => {
                    out[n] = v;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Drop whatever the network thread queued in the RX ring. The rig keeps
    /// streaming I/Q for the whole over, so `rx_read` would otherwise replay
    /// a stale backlog after `tx_end`.
    pub fn discard_pending_rx(&mut self) {
        while self.ring.pop().is_ok() {}
    }

    /// Drain any rig-reported frequency/mode changes for this receiver.
    pub fn poll_updates(&self) -> Vec<TciUpdate> {
        self.updates.try_iter().collect()
    }

    /// Latest TX telemetry (SWR) the rig reported, or `None` if nothing new
    /// arrived since the last call. A default (all-`None`) value is pushed
    /// when TX stops, so the reading clears on unkey.
    pub fn poll_telemetry(&self) -> Option<TxTelemetry> {
        self.telem.as_ref().and_then(|t| t.try_iter().last())
    }
}

/// The single-receiver view of a TCI connection: receiver 0 of a
/// [`TciDevice`] of its own. What every caller used before receivers were
/// split from the connection, kept for them — the tests and the live example
/// drive exactly one receiver.
pub struct TciHandle {
    rx0: TciRx,
    pub device: String,
    pub sample_rate_hz: f64,
}

impl TciHandle {
    /// Connect to `address` (`host:port`), start receiver 0's IQ stream at
    /// `iq_rate_hz`, and spawn the streaming thread.
    pub fn connect(address: &str, iq_rate_hz: f64) -> Result<TciHandle, TciError> {
        let dev = TciDevice::connect(address, iq_rate_hz)?;
        let rx0 = dev.rx(0)?;
        Ok(TciHandle {
            device: dev.device().to_string(),
            sample_rate_hz: dev.sample_rate_hz(),
            rx0,
        })
    }

    pub fn is_alive(&self) -> bool {
        self.rx0.is_alive()
    }
    pub fn set_center(&self, hz: f64) {
        self.rx0.set_center(hz);
    }
    pub fn set_if_offset(&self, hz: f64) {
        self.rx0.set_if_offset(hz);
    }
    pub fn set_mode(&self, mode: Mode) {
        self.rx0.set_mode(mode);
    }
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        self.rx0.tx_begin(tx_freq_hz)
    }
    pub fn tx_end(&self) {
        self.rx0.tx_end();
    }
    pub fn set_drive(&self, frac: f64) {
        self.rx0.set_drive(frac);
    }
    pub fn set_tune_drive(&self, frac: f64) {
        self.rx0.set_tune_drive(frac);
    }
    pub fn tx_write(&mut self, audio: &[f32]) {
        self.rx0.tx_write(audio);
    }
    pub fn tx_pending(&self) -> usize {
        self.rx0.tx_pending()
    }
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        self.rx0.rx_read(out)
    }
    pub fn discard_pending_rx(&mut self) {
        self.rx0.discard_pending_rx();
    }
    pub fn poll_updates(&self) -> Vec<TciUpdate> {
        self.rx0.poll_updates()
    }
    pub fn poll_telemetry(&self) -> Option<TxTelemetry> {
        self.rx0.poll_telemetry()
    }
}

/// One attached receiver's state inside the net thread.
struct RxSlot {
    ring: Producer<f32>,
    updates: Sender<TciUpdate>,
    center: f64,
    /// Current rig IF offset (rig VFO = center + if_hz).
    if_hz: f64,
    /// The receive IF offset, restored when leaving TX (receiver 0 only —
    /// the others never transmit).
    rx_if: f64,
    /// When we last commanded this receiver's frequency (dds/if). The rig
    /// echoes each change with WS latency; during a rapid waterfall drag
    /// those delayed echoes reflect stale positions, so `vfo:` reports are
    /// ignored for a short while after our own commands rather than mistaken
    /// for operator dial moves.
    if_cmd_at: Option<Instant>,
    mode: String,
    /// When this receiver's IQ last arrived — the stream watchdog's clock.
    /// A rig can stop one stream while the WebSocket stays healthy (seen live
    /// on a SunSDR2DX after receivers were toggled), and a stalled stream on a
    /// live connection would otherwise be silence forever: nothing marks the
    /// connection dead, so nothing reconnects.
    last_iq_at: Instant,
    /// When the watchdog last asked the rig to start this stream again, so a
    /// genuinely stopped stream is nudged periodically, not flooded.
    kicked_at: Option<Instant>,
}

impl RxSlot {
    fn new(ring: Producer<f32>, updates: Sender<TciUpdate>) -> RxSlot {
        RxSlot {
            ring,
            updates,
            center: 0.0,
            if_hz: 0.0,
            rx_if: 0.0,
            if_cmd_at: None,
            mode: String::new(),
            last_iq_at: Instant::now(),
            kicked_at: None,
        }
    }
}

struct NetThread {
    ws: Ws,
    /// The attached receivers, by wire index.
    slots: HashMap<u32, RxSlot>,
    tx: Consumer<f32>,
    ctrl: Receiver<Ctrl>,
    /// TX telemetry (SWR) parsed from rig status text.
    telem: Sender<TxTelemetry>,
    ptt: bool,
    /// TX-audio packets sent this key-down (for diagnostics).
    tx_pkts: u64,
    /// Latest TX telemetry, aggregated across `tx_sensors`/`tx_swr`/`tx_power`
    /// (each may arrive separately), emitted as a combined snapshot. Reset per
    /// key-down.
    tx_fwd_w: Option<f32>,
    tx_swr: Option<f32>,
    /// Last known rig power levels (percent), updated both when we command
    /// them and when the rig reports them — so our own echoes aren't mistaken
    /// for operator changes.
    drive_pct: Option<u32>,
    tune_drive_pct: Option<u32>,
    /// Power levels adopted from the connect burst, delivered to receiver 0's
    /// update channel the moment it attaches (it does not exist earlier).
    adopted: Vec<TciUpdate>,
    /// When the current key-down started, and whether the rig has asked for
    /// TX audio with a `TxChrono` since. Until one arrives we fall back to
    /// sending self-paced packets, for servers that never chrono.
    tx_on_at: Option<Instant>,
    chrono_seen: bool,
    /// Whether the TX ring has filled enough to start feeding chronos from it.
    tx_primed: bool,
}

/// How long an attached receiver may go without IQ — on a connection that is
/// otherwise healthy, and not transmitting — before the stream counts as
/// stalled and the watchdog asks the rig to start it again. Generous next to
/// the tens of frames a second a live stream carries.
const IQ_STARVE_AFTER: Duration = Duration::from_secs(2);
/// How often a stalled stream is nudged while it stays silent.
const IQ_KICK_EVERY: Duration = Duration::from_secs(3);

/// How long to wait for the rig's first `TxChrono` before assuming this server
/// doesn't send them and self-pacing the TX audio instead.
const CHRONO_GRACE: Duration = Duration::from_millis(150);

/// Stereo frames per `TxAudio` packet, bounded by the rig's 16 KiB stream
/// buffer (4096 f32 values). Chronos ask for far less than this.
const MAX_TX_FRAMES: usize = 2048;

impl NetThread {
    fn run(mut self) {
        let mut iq_scratch: Vec<f32> = Vec::with_capacity(1024);
        loop {
            let mut dirty = false;

            // 1) Control messages.
            while let Ok(msg) = self.ctrl.try_recv() {
                match msg {
                    Ctrl::Attach { rx, ring, updates } => {
                        // Receiver 0 inherits the power levels read during the
                        // handshake — they describe the one transmitter, which
                        // is its.
                        if rx == TX_RX {
                            for u in self.adopted.drain(..) {
                                let _ = updates.send(u);
                            }
                        }
                        self.slots.insert(rx, RxSlot::new(ring, updates));
                        send_text(&mut self.ws, p::rx_enable(rx, true));
                        send_text(&mut self.ws, p::iq_start(rx));
                        dirty = true;
                        tracing::debug!(rx, "TCI receiver attached");
                    }
                    Ctrl::Detach { rx } => {
                        if self.slots.remove(&rx).is_some() {
                            if rx == TX_RX && self.ptt {
                                // The stream that owns the transmitter is
                                // going away mid-over: unkey rather than
                                // leaving the rig on the air unattended.
                                send_text(&mut self.ws, p::trx(TX_RX, false, false));
                                send_text(&mut self.ws, p::audio_stop(TX_RX));
                                self.ptt = false;
                                self.tx_on_at = None;
                                while self.tx.pop().is_ok() {}
                            }
                            send_text(&mut self.ws, p::iq_stop(rx));
                            // Only the stream is stopped — `rx_enable` is
                            // deliberately left alone. Disabling the rig's
                            // second receiver here (as this once did) toggles
                            // the whole software receiver off under its
                            // operator, and a SunSDR2DX seen live could come
                            // out of that churn with streams stalled on a
                            // healthy connection. We only ever switch
                            // receivers *on*; the rig's own state is the
                            // operator's.
                            dirty = true;
                            tracing::debug!(rx, "TCI receiver detached");
                        }
                    }
                    Ctrl::SetCenter { rx, hz } => {
                        // Move this receiver's IQ centre. The IF offset (VFO
                        // within the band) is re-asserted separately by
                        // `SetIf`.
                        if let Some(s) = self.slots.get_mut(&rx) {
                            s.center = hz;
                            let rx_if = s.rx_if;
                            s.if_hz = rx_if;
                            s.if_cmd_at = Some(Instant::now());
                            send_text(&mut self.ws, p::dds(rx, hz));
                            send_text(&mut self.ws, p::if_offset(rx, CH, rx_if));
                            dirty = true;
                        }
                    }
                    Ctrl::SetIf { rx, hz } => {
                        // Our software DDC tunes the VFO within the IQ band;
                        // mirror it onto the rig's own VFO so its display
                        // tracks the dial and returning from TX doesn't snap
                        // back to the centre.
                        if let Some(s) = self.slots.get_mut(&rx) {
                            s.rx_if = hz;
                            let tx_holds_vfo = self.ptt && rx == TX_RX;
                            if !tx_holds_vfo && (hz - s.if_hz).abs() > 0.5 {
                                s.if_hz = hz;
                                s.if_cmd_at = Some(Instant::now());
                                send_text(&mut self.ws, p::if_offset(rx, CH, hz));
                                dirty = true;
                            }
                        }
                    }
                    Ctrl::SetMode { rx, mode } => {
                        if let Some(s) = self.slots.get_mut(&rx) {
                            s.mode = p::mode_to_tci(mode).to_string();
                            let m = s.mode.clone();
                            send_text(&mut self.ws, p::modulation(rx, &m));
                            dirty = true;
                        }
                    }
                    Ctrl::TxOn(tx_freq) => {
                        // Place the rig VFO at the TX frequency via the IF
                        // offset (keeps dds/IQ centered), then key with the
                        // TCI source. The transmitter is receiver 0's.
                        let Some(s) = self.slots.get_mut(&TX_RX) else { continue };
                        let off = tx_freq - s.center;
                        if (off - s.if_hz).abs() > 0.5 {
                            send_text(&mut self.ws, p::if_offset(TX_RX, CH, off));
                        }
                        s.if_hz = off;
                        s.if_cmd_at = Some(Instant::now());
                        if !s.mode.is_empty() {
                            let m = s.mode.clone();
                            send_text(&mut self.ws, p::modulation(TX_RX, &m));
                        }
                        // Start the audio stream so the rig accepts our
                        // TxAudio, then key with the TCI audio source.
                        send_text(&mut self.ws, p::audio_start(TX_RX));
                        send_text(&mut self.ws, p::trx(TX_RX, true, true));
                        self.ptt = true;
                        self.tx_on_at = Some(Instant::now());
                        self.chrono_seen = false;
                        self.tx_primed = false;
                        // Start this key-down with no stale telemetry.
                        self.tx_fwd_w = None;
                        self.tx_swr = None;
                        dirty = true;
                        tracing::debug!(tx_freq, off, "TCI TX on");
                    }
                    Ctrl::TxOff => {
                        send_text(&mut self.ws, p::trx(TX_RX, false, false));
                        send_text(&mut self.ws, p::audio_stop(TX_RX));
                        // Restore the receive IF offset (the VFO within the
                        // IQ) — NOT zero — so the rig VFO stays on the
                        // operator's dial.
                        if let Some(s) = self.slots.get_mut(&TX_RX) {
                            if (s.rx_if - s.if_hz).abs() > 0.5 {
                                send_text(&mut self.ws, p::if_offset(TX_RX, CH, s.rx_if));
                            }
                            s.if_hz = s.rx_if;
                            s.if_cmd_at = Some(Instant::now());
                        }
                        self.ptt = false;
                        self.tx_on_at = None;
                        // Drop whatever the engine queued past the unkey, so
                        // the next over can't start with the tail of this one.
                        while self.tx.pop().is_ok() {}
                        // Clear telemetry so the meter drops the reading on
                        // unkey.
                        self.tx_fwd_w = None;
                        self.tx_swr = None;
                        let _ = self.telem.send(TxTelemetry::default());
                        // The watchdog was paused for the over and a rig may
                        // legitimately have paused RX streams during it: every
                        // stream gets a fresh grace period, not a stale clock.
                        let now = Instant::now();
                        for slot in self.slots.values_mut() {
                            slot.last_iq_at = now;
                        }
                        dirty = true;
                        tracing::debug!("TCI TX off");
                    }
                    Ctrl::SetDrive(frac) => {
                        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u32;
                        self.drive_pct = Some(pct);
                        send_text(&mut self.ws, p::drive(TX_RX, pct));
                        dirty = true;
                        tracing::debug!(pct, "TCI drive");
                    }
                    Ctrl::SetTuneDrive(frac) => {
                        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u32;
                        self.tune_drive_pct = Some(pct);
                        send_text(&mut self.ws, p::tune_drive(TX_RX, pct));
                        dirty = true;
                    }
                    Ctrl::Shutdown => {
                        if self.ptt {
                            send_text(&mut self.ws, p::trx(TX_RX, false, false));
                        }
                        for (&rx, _) in self.slots.iter() {
                            send_text(&mut self.ws, p::iq_stop(rx));
                        }
                        let _ = self.ws.flush();
                        let _ = self.ws.close(None);
                        return;
                    }
                }
            }

            // 2) One inbound WebSocket message.
            match self.ws.read() {
                Ok(Message::Binary(b)) => {
                    if let Some(h) = p::parse_header(&b) {
                        match h.dtype {
                            // Each receiver's IQ goes to its own ring; a
                            // stream nobody attached (the rig can enable
                            // receivers on its own) falls on the floor.
                            p::DataType::Iq => {
                                if let Some(slot) = self.slots.get_mut(&h.receiver) {
                                    slot.last_iq_at = Instant::now();
                                    slot.kicked_at = None;
                                    iq_scratch.clear();
                                    p::decode_f32_payload(&b, &h, &mut iq_scratch);
                                    // Whole I/Q pairs or nothing. Pushing float
                                    // by float and letting the ring refuse
                                    // wherever it happens to fill leaves it an
                                    // odd number of floats deep, and from then
                                    // on every pair is one float out of step —
                                    // I read as Q for the rest of the session,
                                    // a mirrored and unusable spectrum. The
                                    // ring reliably does fill: the engine stops
                                    // reading for the length of an over.
                                    for pair in iq_scratch.chunks_exact(2) {
                                        if slot.ring.slots() < 2 {
                                            break;
                                        }
                                        let _ = slot.ring.push(pair[0]);
                                        let _ = slot.ring.push(pair[1]);
                                    }
                                }
                            }
                            // The rig meters the TX audio itself: each chrono
                            // asks for exactly `length` floats, and one packet
                            // of that size is played per chrono. Answering
                            // with our own cadence leaves the rest of the
                            // rig's buffer silent, which costs average power.
                            p::DataType::TxChrono if h.receiver == TX_RX && self.ptt => {
                                self.chrono_seen = true;
                                self.answer_chrono(h.length as usize);
                                dirty = true;
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Message::Text(t)) => self.on_text(t.as_str()),
                Ok(Message::Close(_)) => {
                    tracing::warn!("TCI server closed the connection");
                    return;
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    tracing::warn!("TCI read error: {e}; stopping");
                    return;
                }
            }

            // 3) The stream watchdog: a rig can stop one receiver's IQ while
            //    the WebSocket stays healthy (a SunSDR2DX was seen doing so
            //    live, after receivers were toggled), and nothing else would
            //    ever notice — the connection is alive, so no reconnect runs.
            //    Ask for the stream again until it flows. Not while keyed:
            //    a rig is allowed to pause RX during its own TX, and poking
            //    it mid-over would be worse than waiting it out.
            if !self.ptt {
                self.kick_starved_streams(&mut dirty);
            }

            // 4) While keyed, TX audio normally goes out one packet per
            //    `TxChrono` (handled above). Only if the rig never chronos do
            //    we fall back to self-paced packets at the engine's ~48 kHz
            //    fill.
            if self.ptt {
                let in_grace = self.tx_on_at.is_some_and(|t| t.elapsed() < CHRONO_GRACE);
                if !self.chrono_seen && !in_grace {
                    let avail = self.tx.slots();
                    if avail >= 480 {
                        self.send_tx_audio(avail.min(MAX_TX_FRAMES), true);
                        dirty = true;
                    }
                }
            } else {
                self.tx_pkts = 0;
            }

            if dirty {
                let _ = self.ws.flush();
            }
        }
    }

    /// Re-ask the rig to stream any attached receiver whose IQ has gone
    /// silent (see the call site in [`NetThread::run`] for why). `rx_enable`
    /// is re-sent along with `iq_start`: a stall born of the receiver being
    /// toggled off needs both, and for one that is already enabled the extra
    /// command is an idempotent echo.
    fn kick_starved_streams(&mut self, dirty: &mut bool) {
        for (&rx, slot) in self.slots.iter_mut() {
            if slot.last_iq_at.elapsed() < IQ_STARVE_AFTER {
                continue;
            }
            if slot.kicked_at.is_some_and(|t| t.elapsed() < IQ_KICK_EVERY) {
                continue;
            }
            if slot.kicked_at.is_none() {
                tracing::warn!(rx, "TCI receiver stopped streaming; asking the rig again");
            }
            slot.kicked_at = Some(Instant::now());
            send_text(&mut self.ws, p::rx_enable(rx, true));
            send_text(&mut self.ws, p::iq_start(rx));
            *dirty = true;
        }
    }

    /// Answer one `TxChrono`, which asks for `floats` stereo values (i.e.
    /// `floats / 2` frames — the rig divides by the channel count).
    ///
    /// A chrono covers ~21 ms while the engine feeds in 10 ms blocks, so the
    /// first chronos of a key-down are answered with silence until the ring
    /// holds a whole buffer. That standing cushion is what keeps a late block
    /// from punching a hole in the audio; draining the ring dry from the first
    /// chrono would leave it with no margin at all.
    fn answer_chrono(&mut self, floats: usize) {
        let frames = (floats / 2).min(MAX_TX_FRAMES);
        if !self.tx_primed {
            if self.tx.slots() < frames {
                self.send_tx_audio(frames, false);
                return;
            }
            self.tx_primed = true;
        }
        self.send_tx_audio(frames, true);
    }

    /// Send one `TxAudio` packet of exactly `frames` stereo frames, filled
    /// from the TX ring when `from_ring` is set (zero-padded if the engine
    /// hasn't produced enough yet — the spec's preferred way to say "nothing
    /// to play") and silent otherwise.
    fn send_tx_audio(&mut self, frames: usize, from_ring: bool) {
        if frames == 0 {
            return;
        }
        let mut mono = Vec::with_capacity(frames);
        while from_ring && mono.len() < frames {
            match self.tx.pop() {
                Ok(v) => mono.push(v),
                Err(_) => break,
            }
        }
        let short = frames - mono.len();
        mono.resize(frames, 0.0);
        let peak = mono.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
        let pkt = p::build_tx_audio(TX_RATE_HZ, TX_RX, &mono);
        match self.ws.write(Message::Binary(pkt.into())) {
            Ok(()) => {
                self.tx_pkts += 1;
                if self.tx_pkts == 1 || self.tx_pkts % 50 == 0 {
                    tracing::debug!(
                        pkts = self.tx_pkts,
                        frames,
                        short,
                        peak,
                        chrono = self.chrono_seen,
                        "TCI TX audio sent"
                    );
                }
            }
            Err(e) => tracing::warn!("TCI TX audio write failed: {e}"),
        }
    }

    /// Parse rig status echoes; emit freq/mode changes the operator made on
    /// the rig (filtered against what we last set, so our own echoes don't
    /// loop), routed to the receiver they belong to.
    fn on_text(&mut self, text: &str) {
        for (cmd, args) in p::parse_status(text) {
            match cmd.as_str() {
                "vfo" => {
                    // vfo:<rx>,<ch>,<hz>. The rig VFO we expect (from our own
                    // dds + if commands) is that receiver's `center + if_hz`;
                    // echoes matching it are our own and must be ignored, or a
                    // TX IF move would loop back as a dial change. Only a
                    // genuine operator dial move differs.
                    let f: Vec<&str> = args.split(',').collect();
                    if f.len() == 3
                        && f[1].trim() == "0"
                        && let Ok(rx) = f[0].trim().parse::<u32>()
                        && let Some(slot) = self.slots.get_mut(&rx)
                        && let Ok(hz) = f[2].trim().parse::<f64>()
                    {
                        // Ignore echoes right after our own dds/if commands —
                        // during a fast drag they arrive late and reflect
                        // stale positions, so following them fights the drag.
                        let ours = slot
                            .if_cmd_at
                            .is_some_and(|t| t.elapsed() < Duration::from_millis(300));
                        let expected = slot.center + slot.if_hz;
                        if !ours && (hz - expected).abs() > 1.0 {
                            // Operator turned the rig dial: adopt it as this
                            // receiver's centre (VFO == centre, IF back to
                            // zero).
                            slot.center = hz;
                            slot.if_hz = 0.0;
                            slot.rx_if = 0.0;
                            let _ = slot.updates.send(TciUpdate::Freq(hz));
                        }
                    }
                }
                "modulation" => {
                    // modulation:<rx>,<mode>
                    if let Some((rx, m)) = args.split_once(',')
                        && let Ok(rx) = rx.trim().parse::<u32>()
                        && let Some(slot) = self.slots.get_mut(&rx)
                    {
                        let ml = m.trim().to_lowercase();
                        if ml != slot.mode
                            && let Some(mode) = p::tci_to_mode(&ml)
                        {
                            slot.mode = ml;
                            let _ = slot.updates.send(TciUpdate::Mode(mode));
                        }
                    }
                }
                // Power levels the operator changed in ExpertSDR3. They
                // describe the one transmitter, so they go to receiver 0.
                // Echoes of our own commands carry the value we already know,
                // so only a real change is reported onwards.
                "drive" => {
                    if let Some(pct) = parse_pct(&args)
                        && self.drive_pct != Some(pct)
                    {
                        self.drive_pct = Some(pct);
                        if let Some(slot) = self.slots.get(&TX_RX) {
                            let _ = slot.updates.send(TciUpdate::Drive(pct as f32 / 100.0));
                        }
                    }
                }
                "tune_drive" => {
                    if let Some(pct) = parse_pct(&args)
                        && self.tune_drive_pct != Some(pct)
                    {
                        self.tune_drive_pct = Some(pct);
                        if let Some(slot) = self.slots.get(&TX_RX) {
                            let _ = slot.updates.send(TciUpdate::TuneDrive(pct as f32 / 100.0));
                        }
                    }
                }
                "tx_sensors" => {
                    // tx_sensors:<trx>,<mic_db>,<fwd_pwr_w>,<rev_pwr_w>,<swr>.
                    // Broadcast at ~4-10 Hz during TX once tx_sensors_enable
                    // is on.
                    let f: Vec<&str> = args.split(',').collect();
                    if f.first() == Some(&"0") && f.len() >= 5 {
                        tracing::debug!(args = %args, "TCI tx_sensors");
                        self.tx_fwd_w = parse_watts(f[2]);
                        self.tx_swr = parse_swr(f[4]);
                        self.emit_tx_telem();
                    }
                }
                // Some builds send SWR / forward power as separate messages,
                // in addition to or instead of the `tx_sensors` fields.
                "tx_swr" => {
                    if let Some(v) = parse_swr(args.rsplit(',').next().unwrap_or("")) {
                        self.tx_swr = Some(v);
                        self.emit_tx_telem();
                    }
                }
                "tx_power" | "tx_forward_power" => {
                    if let Some(v) = parse_watts(args.rsplit(',').next().unwrap_or("")) {
                        self.tx_fwd_w = Some(v);
                        self.emit_tx_telem();
                    }
                }
                _ => {}
            }
        }
    }

    /// Emit the current aggregated TX telemetry snapshot.
    fn emit_tx_telem(&self) {
        let _ = self.telem.send(TxTelemetry {
            fwd_w: self.tx_fwd_w,
            swr: self.tx_swr,
            alc: None,
            po: None,
        });
    }
}

/// Parse a `drive:`/`tune_drive:` payload (`<trx>,<percent>` since TCI 1.5, or
/// a bare `<percent>` on older servers) for TRX 0, as a 0..=100 percentage.
fn parse_pct(args: &str) -> Option<u32> {
    let f: Vec<&str> = args.split(',').collect();
    let raw = match f.as_slice() {
        [pct] => pct,
        [trx, pct] if trx.trim() == "0" => pct,
        _ => return None,
    };
    raw.trim().parse::<f32>().ok().filter(|v| (0.0..=100.0).contains(v)).map(|v| v.round() as u32)
}

/// Parse a forward/reverse power field (watts): finite and non-negative.
fn parse_watts(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok().filter(|w| w.is_finite() && *w >= 0.0)
}

/// Parse an SWR field (ratio): finite and physically valid (>= 1.0).
fn parse_swr(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok().filter(|v| v.is_finite() && (1.0..=100.0).contains(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_sensors_fields_parse() {
        // tx_sensors:<trx>,<mic_db>,<fwd_pwr>,<rev_pwr>,<swr>
        let f: Vec<&str> = "0,-12.0,4.8,0.2,1.4".split(',').collect();
        assert_eq!(parse_watts(f[2]), Some(4.8)); // forward power (W)
        assert_eq!(parse_swr(f[4]), Some(1.4)); // SWR ratio
    }

    #[test]
    fn drive_percentages_parse() {
        assert_eq!(parse_pct("0,75"), Some(75)); // TCI >= 1.5: <trx>,<percent>
        assert_eq!(parse_pct("40"), Some(40)); // older servers: bare percent
        assert_eq!(parse_pct(" 0 , 100 "), Some(100));
        assert_eq!(parse_pct("1,75"), None); // another TRX — not ours
        assert_eq!(parse_pct("0,120"), None); // out of range
        assert_eq!(parse_pct("0,x"), None);
        assert_eq!(parse_pct(""), None);
    }

    #[test]
    fn rejects_out_of_range_and_garbage() {
        assert_eq!(parse_swr("0.5"), None); // SWR < 1.0 is unphysical
        assert_eq!(parse_swr("nan"), None);
        assert_eq!(parse_swr("  1.8 "), Some(1.8)); // tolerates whitespace
        assert_eq!(parse_watts("-1.0"), None); // negative power rejected
        assert_eq!(parse_watts("x"), None);
        assert_eq!(parse_watts("0"), Some(0.0)); // zero forward power is valid
    }
}
