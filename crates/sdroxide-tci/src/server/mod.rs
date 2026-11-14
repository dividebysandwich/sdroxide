//! The TCI **server**: sdroxide acting as a TCI rig so third-party clients
//! (WSJT-X's TCI rig type, JTDX, MSHV, skimmer tools) can drive it.
//!
//! Shape mirrors `SkimmerController`: the DSP engine owns a
//! [`TciServerController`], pushes realtime IQ and RX audio into it over bounded
//! channels (dropping on backpressure so a slow client can never stall the
//! engine), and drains [`ServerRequest`]s non-blocking each tick. All socket
//! work happens on a hub thread plus one thread per client.
//!
//! **Model.** sdroxide has a single tunable VFO pair, so the server advertises
//! one transceiver (`trx_count:1`) with two channels (`channels_count:2`,
//! A and B). Streams — wideband IQ, RX audio, TX audio — exist on receiver 0
//! only. `dds:` is our hardware centre frequency and `if:` the VFO's offset
//! from it, matching how sdroxide's own TCI client drives a rig.
//!
//! **Multiple clients** all see the same radio: last writer wins, and every
//! client receives the resulting status broadcast (including the one that
//! caused it — real TCI servers echo unconditionally). Only one client at a
//! time may hold the transmit key.
//!
//! **Every command must be echoed back, even when it changes nothing.** Clients
//! use the echo as the acknowledgement and block on it: WSJT-X sends
//! `audio_start:0;`, waits, and aborts the whole connection with "TCI Audio
//! could not be switched on" if the echo never arrives. Suppressing no-op
//! echoes as redundant is therefore a connection-breaking bug, not an
//! optimisation. The state diff in [`text::diff_lines`] is only for *unsolicited*
//! changes (the operator moved the dial); acknowledgements come from the hub.
//!
//! To debug a client that won't talk to us, `RUST_LOG=sdroxide_tci=debug` logs
//! the whole conversation in both directions.

mod client;
mod hub;
mod text;

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use sdroxide_types::{Command, DeviceCaps, Mode, TciServerConfig, TxTelemetry};

/// TCI protocol version we claim in the `protocol:` line.
const TCI_VERSION: &str = "1.9";
/// Both audio streams (RX out and TX in) run at 48 kHz, as TCI expects.
pub const AUDIO_RATE_HZ: u32 = 48_000;
/// Receiver index the streams live on.
const RX: u32 = 0;

/// TX audio ring depth: one second at 48 kHz. Deep enough that a client which
/// races ahead of the chrono pacing isn't truncated, shallow enough that a
/// disconnect can't leave many seconds of stale audio queued.
const TX_RING: usize = AUDIO_RATE_HZ as usize;

/// Something a client asked for that only the engine can carry out.
// `Command` is large, but boxing it here would buy an allocation per client
// command and force the engine to unbox before `apply` — these arrive a few
// times a second at most, and every other channel in sdroxide carries
// `Command` by value.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ServerRequest {
    /// Apply this command. The engine runs it through `Engine::apply`, so every
    /// safety rail and the usual state broadcast come along for free.
    Cmd(Command),
    /// A client keyed (`true`) or unkeyed (`false`) the transmitter.
    Key(bool),
    /// The connected-client count changed.
    Clients(usize),
}

/// The slice of radio state TCI clients can see. The hub diffs successive
/// snapshots, so only genuinely-changed values reach the wire.
#[derive(Debug, Clone, PartialEq)]
pub struct TciStateSnapshot {
    pub vfo_a_hz: f64,
    pub vfo_b_hz: f64,
    /// Centre of the wideband IQ stream — TCI's `dds`.
    pub center_hz: f64,
    /// How far the VFO may sit from the centre, i.e. half the IQ span.
    pub if_span_hz: f64,
    pub mode: Mode,
    pub split: bool,
    pub ptt: bool,
    pub tune: bool,
    pub drive_pct: u32,
    pub tune_drive_pct: u32,
    pub muted: bool,
    pub volume_db: i32,
    /// Actual IQ stream rate. Zero means no wideband IQ is available (an
    /// audio-mode source), and the stream is then not advertised at all.
    pub iq_rate: u32,
    pub vfo_lo_hz: f64,
    pub vfo_hi_hz: f64,
    /// Whether this radio can transmit at all — reported as `receive_only:`.
    pub can_tx: bool,
}

impl Default for TciStateSnapshot {
    fn default() -> Self {
        TciStateSnapshot {
            vfo_a_hz: 0.0,
            vfo_b_hz: 0.0,
            center_hz: 0.0,
            if_span_hz: 0.0,
            mode: Mode::Usb,
            split: false,
            ptt: false,
            tune: false,
            drive_pct: 0,
            tune_drive_pct: 0,
            muted: false,
            volume_db: 0,
            iq_rate: 0,
            vfo_lo_hz: 0.0,
            vfo_hi_hz: 0.0,
            can_tx: false,
        }
    }
}

impl TciStateSnapshot {
    /// Convert a 0..1 volume factor to the dB figure TCI reports.
    pub fn volume_db_from(v: f32) -> i32 {
        text::volume_to_db(v)
    }
}

/// Control traffic to the hub. Never dropped.
pub(crate) enum Ctl {
    Config(TciServerConfig),
    State(Box<TciStateSnapshot>),
    Telemetry(TxTelemetry),
    /// Ask the keying client for `frames` more stereo frames of TX audio.
    Chrono(u32),
    /// The engine took the transmitter back. Release the key and tell the
    /// client, without asking the engine to unkey — it already has.
    Unkey,
    Stop,
}

/// One block of realtime data, dropped when the hub is behind.
pub(crate) struct IqBlock {
    pub rate: u32,
    pub data: Vec<f32>,
}
pub(crate) struct AudioBlock(pub Vec<f32>);

/// Subscription state for one client, written by its own thread and read by the
/// hub before every fan-out push. Atomics rather than a lock so the hub never
/// blocks on a client that is mid-parse.
#[derive(Default)]
pub(crate) struct Subs {
    pub iq_on: AtomicBool,
    pub audio_on: AtomicBool,
    pub tx_sensors: AtomicBool,
    /// Realtime frames dropped because this client's data lane was full.
    pub dropped: AtomicUsize,
}

/// Shared totals the engine reads directly, without waiting for an event.
#[derive(Default)]
pub(crate) struct Shared {
    pub clients: AtomicUsize,
    /// Newest IQ rate any client asked for; 0 when nobody wants IQ.
    pub iq_rate: AtomicU32,
    /// Whether at least one client is subscribed to RX audio.
    pub audio_on: AtomicBool,
    /// Whether a client currently holds the transmit key.
    pub tx_keyed: AtomicBool,
    /// IQ blocks the engine had to drop because the hub had not yet taken
    /// the earlier ones — counted so a short stream shows up in the log.
    pub iq_dropped: std::sync::atomic::AtomicU64,
}

/// The engine's handle on the running server.
pub struct TciServerController {
    ctl_tx: Sender<Ctl>,
    iq_tx: Sender<IqBlock>,
    aud_tx: Sender<AudioBlock>,
    req_rx: Receiver<ServerRequest>,
    /// 48 kHz mono TX audio from whichever client holds the key.
    tx_audio: rtrb::Consumer<f32>,
    shared: Arc<Shared>,
    addr: String,
    hub: Option<JoinHandle<()>>,
}

impl TciServerController {
    /// Bind and start serving. The listener is bound **before** the hub thread
    /// is spawned, so a bind failure (the usual one being a local ExpertSDR3
    /// already holding port 50001) surfaces here as an `Err` the settings
    /// dialog can show, rather than disappearing into a thread.
    pub fn start(
        cfg: &TciServerConfig,
        caps: &DeviceCaps,
        state: TciStateSnapshot,
    ) -> Result<Self, String> {
        let addr = cfg.addr();
        let listener = TcpListener::bind(&addr).map_err(|e| format!("bind {addr}: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| format!("{addr}: {e}"))?;

        let (ctl_tx, ctl_rx) = unbounded();
        // The engine hands IQ over in small blocks — about 1,800 a second at
        // 2 Msps — and the hub collects them every few milliseconds at best and
        // now and then after a pause of 100 ms or more. Eight blocks were far
        // too few to cover such a pause: measured 14.11. on a RSP1A, TCI clients
        // received 90 % of the IQ they asked for, at every rate, while RX audio
        // from the same samples arrived complete. 512 blocks cover about a
        // quarter of a second at that rate.
        // Why: HDZ 2026-11-14, TCI_transcode against sdroxide: rtl_433 decoded
        //      nothing from the gappy stream.
        let (iq_tx, iq_rx) = bounded(512);
        let (aud_tx, aud_rx) = bounded(16);
        let (req_tx, req_rx) = unbounded();
        let (tx_prod, tx_audio) = rtrb::RingBuffer::<f32>::new(TX_RING);

        let shared = Arc::new(Shared::default());
        let hub = hub::spawn(hub::HubParams {
            listener,
            cfg: cfg.clone(),
            device_name: cfg.device_name.clone(),
            caps: caps.clone(),
            state,
            ctl_rx,
            iq_rx,
            aud_rx,
            req_tx,
            tx_prod,
            shared: shared.clone(),
        });

        Ok(TciServerController {
            ctl_tx,
            iq_tx,
            aud_tx,
            req_rx,
            tx_audio,
            shared,
            addr,
            hub: Some(hub),
        })
    }

    /// The `host:port` we are bound to.
    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn clients(&self) -> usize {
        self.shared.clients.load(Ordering::Relaxed)
    }

    /// Apply a changed configuration that doesn't move the socket (client
    /// limit, transmit permission). Rebinding is the caller's job.
    pub fn set_config(&self, cfg: TciServerConfig) {
        let _ = self.ctl_tx.send(Ctl::Config(cfg));
    }

    // --- Realtime, engine → clients. Both drop rather than block. ---

    /// Hand over a block of interleaved I,Q at `rate`. No-op when no client is
    /// subscribed, so an unsubscribed stream costs one atomic load.
    pub fn on_rx_iq(&self, iq: &[f32], rate: u32) {
        if self.shared.iq_rate.load(Ordering::Relaxed) == 0 || iq.is_empty() {
            return;
        }
        if self.iq_tx.try_send(IqBlock { rate, data: iq.to_vec() }).is_err() {
            let n = self.shared.iq_dropped.fetch_add(1, Ordering::Relaxed) + 1;
            if n.is_power_of_two() {
                tracing::warn!(
                    dropped = n,
                    "TCI server: IQ blocks dropped before the hub took them"
                );
            }
        }
    }

    /// Hand over a block of 48 kHz mono RX audio.
    pub fn on_rx_audio(&self, mono: &[f32]) {
        if !self.shared.audio_on.load(Ordering::Relaxed) || mono.is_empty() {
            return;
        }
        let _ = self.aud_tx.try_send(AudioBlock(mono.to_vec()));
    }

    // --- Subscription queries, polled by the engine each tick. ---

    /// The IQ rate clients are asking for, or `None` when nobody wants IQ.
    pub fn wants_iq(&self) -> Option<u32> {
        match self.shared.iq_rate.load(Ordering::Relaxed) {
            0 => None,
            r => Some(r),
        }
    }

    pub fn wants_audio(&self) -> bool {
        self.shared.audio_on.load(Ordering::Relaxed)
    }

    /// Whether a client currently holds the transmit key.
    pub fn tx_keyed(&self) -> bool {
        self.shared.tx_keyed.load(Ordering::Relaxed)
    }

    // --- Transmit ---

    /// Pop up to `out.len()` samples of client TX audio. Returns how many were
    /// available; the caller pads the remainder.
    pub fn read_tx_audio(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0;
        while n < out.len() {
            match self.tx_audio.pop() {
                Ok(s) => {
                    out[n] = s;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Samples queued from the keying client.
    pub fn tx_audio_pending(&self) -> usize {
        self.tx_audio.slots()
    }

    /// Throw away anything queued — on unkey, so the next over starts clean.
    pub fn drain_tx_audio(&mut self) {
        while self.tx_audio.pop().is_ok() {}
    }

    /// Ask the keying client for `frames` more stereo frames of TX audio.
    pub fn request_chrono(&self, frames: u32) {
        let _ = self.ctl_tx.send(Ctl::Chrono(frames));
    }

    /// Take the transmitter back from the keying client: release the key and
    /// tell it we are not transmitting on its behalf. Used when the request is
    /// refused, when the safety rails deny it, when the client stops feeding
    /// audio, and when the operator keys up locally.
    pub fn deny_tx(&self) {
        let _ = self.ctl_tx.send(Ctl::Unkey);
    }

    // --- Outbound state ---

    /// Publish the current radio state. The hub diffs it against the last one
    /// and broadcasts only the lines that changed.
    pub fn broadcast_state(&self, snap: TciStateSnapshot) {
        let _ = self.ctl_tx.send(Ctl::State(Box::new(snap)));
    }

    /// Publish TX metering, forwarded to clients that asked for `tx_sensors`.
    pub fn push_telemetry(&self, t: TxTelemetry) {
        let _ = self.ctl_tx.send(Ctl::Telemetry(t));
    }

    /// Drain everything clients have asked for since the last call. Non-blocking.
    pub fn poll(&self) -> Vec<ServerRequest> {
        self.req_rx.try_iter().collect()
    }
}

impl Drop for TciServerController {
    fn drop(&mut self) {
        let _ = self.ctl_tx.send(Ctl::Stop);
        if let Some(h) = self.hub.take() {
            let _ = h.join();
        }
    }
}
