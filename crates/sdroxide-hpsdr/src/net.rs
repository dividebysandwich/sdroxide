//! Shared HPSDR connection handle and the protocol-dispatch open path. Each
//! protocol (1 = Metis, 2 = new) runs its own blocking UDP thread; both stream
//! RX I/Q into a ring, packetize TX I/Q from a ring, and keep the radio alive.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::discovery;
use crate::{protocol1, protocol2};
use sdroxide_types::{HpsdrIoRxInput, HpsdrOcPlan};

/// Host→radio TX I/Q rate. **Both** protocols transmit at 48 kHz: Protocol 2
/// feeds the DUC directly, and Protocol 1's EP2 stream (speaker audio + TX I/Q)
/// is fixed at 48 kHz by the spec regardless of the RX/DDC rate — the radio
/// drains it at 48 ksps no matter how fast EP6 comes back.
pub const TX_RATE_HZ: u32 = 48_000;
/// Resend keep-alive/high-priority state at least this often so the radio's
/// watchdog does not stop the stream.
pub(crate) const WATCHDOG: Duration = Duration::from_millis(50);
/// How often a protocol thread emits an RX throughput line (`RUST_LOG=…=debug`).
pub(crate) const STATS_INTERVAL: Duration = Duration::from_secs(2);

/// Front-end LNA gain range of the Hermes-Lite 2 (the AD9866's LNA/PGA), in dB.
/// The wire value is `dB + 12`, so 0..=60 spans this range in 1 dB steps.
pub const LNA_GAIN_MIN_DB: f64 = -12.0;
pub const LNA_GAIN_MAX_DB: f64 = 48.0;
/// LNA gain used when nothing else is configured. Mid-scale: enough sensitivity
/// on a quiet band without clipping the ADC on a real antenna at night.
pub const LNA_GAIN_DEFAULT_DB: f64 = 20.0;
/// Name of the RX gain element exposed to the UI for that LNA. Defined in
/// `sdroxide-types` so the wasm-safe settings UI can name the same element.
pub const LNA_GAIN_ELEMENT: &str = sdroxide_types::HpsdrConfig::LNA_GAIN_ELEMENT;

/// Whether a board name reported by discovery is a Hermes-Lite.
///
/// The Hermes-Lite gateware repurposes several Protocol 1 register fields that
/// mean something else entirely on a Metis/Hermes/Angelia board — the front-end
/// gain in register `0x0A` and the PA/T-R bits in register `0x09` among them —
/// so every one of those writes is gated on this.
pub fn board_is_hermes_lite(board: &str) -> bool {
    board.starts_with("Hermes-Lite")
}

/// Whether a board has a front-end gain this crate knows how to command
/// (register `0x0A`, C4). Other P1 boards use that register for Alex/attenuator
/// settings, so we must not write it there.
pub fn board_has_lna_gain(board: &str) -> bool {
    board_is_hermes_lite(board)
}

/// Clamp a dB value to the LNA range and encode it as the 6-bit wire value.
pub(crate) fn lna_gain_code(db: f64) -> u8 {
    (db.clamp(LNA_GAIN_MIN_DB, LNA_GAIN_MAX_DB) - LNA_GAIN_MIN_DB).round() as u8
}

/// What each radio said the last time it answered a discovery probe, so a probe
/// that goes unanswered can fall back to fact instead of guessing. See the
/// `None` arm of [`HpsdrBoard::open`] for why probes get lost.
static LAST_PROBE: Mutex<Option<HashMap<Ipv4Addr, (String, u8)>>> = Mutex::new(None);

fn remember_probe(ip: Ipv4Addr, board: &str, protocol: u8) {
    let mut g = LAST_PROBE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(HashMap::new).insert(ip, (board.to_string(), protocol));
}

fn recall_probe(ip: Ipv4Addr) -> Option<(String, u8)> {
    let g = LAST_PROBE.lock().unwrap_or_else(|e| e.into_inner());
    g.as_ref()?.get(&ip).cloned()
}

/// Which connection currently owns each radio.
///
/// Apply/reconnect builds the replacement connection *before* releasing the old
/// one — that ordering is deliberate, so a configuration that cannot be opened
/// leaves the working radio running. For a single-client board like a Metis or
/// Hermes-Lite it means the two overlap: the new connection sends its run
/// command and the board re-targets its stream to the new socket, and only then
/// does the old connection get dropped. If that departing connection sends the
/// protocol's "stop streaming" command on its way out, it switches the radio off
/// underneath the connection that just replaced it — the stream never starts,
/// and the operator has to quit and relaunch.
///
/// So each connection takes a ticket when it opens, and only the holder of the
/// newest ticket for that radio is allowed to stop the stream.
static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);
static CURRENT_CONNECTION: Mutex<Option<HashMap<IpAddr, u64>>> = Mutex::new(None);

/// Register a newly opened connection as the owner of `radio`, returning its
/// ticket. Any connection opened earlier is superseded from this moment.
fn claim_connection(radio: IpAddr) -> u64 {
    let id = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);
    let mut g = CURRENT_CONNECTION.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(HashMap::new).insert(radio, id);
    id
}

/// Whether `id` still owns `radio` — i.e. whether this connection may tell the
/// radio to stop streaming.
pub(crate) fn owns_connection(radio: IpAddr, id: u64) -> bool {
    let g = CURRENT_CONNECTION.lock().unwrap_or_else(|e| e.into_inner());
    g.as_ref().and_then(|m| m.get(&radio)).copied() == Some(id)
}

/// Format the first `n` bytes of a datagram as spaced uppercase hex, for the
/// diagnostic logs that let a remote tester compare on-wire bytes against the
/// OpenHPSDR spec (the wire offsets in this crate are not hardware-verified).
pub(crate) fn hex_head(bytes: &[u8], n: usize) -> String {
    bytes.iter().take(n).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// Periodic RX throughput/health accounting for a protocol thread. Counts
/// decoded I/Q datagrams and unrecognized datagrams, emitting one `debug` line
/// per [`STATS_INTERVAL`] so a tester can see whether I/Q is actually flowing —
/// and at what rate — without a per-packet log flood. A wrong sample rate or a
/// bad decode offset shows up immediately as an implausible ksps figure.
pub(crate) struct RxStats {
    proto: u8,
    /// Nominal RX rate, so the measured long-run rate can be quoted as an error
    /// in ppm — that number is the board's master clock measured against the
    /// host's, and it tells you whether a tuning error scales with frequency
    /// (a clock problem) or is a fixed offset (everything else).
    nominal_hz: f64,
    started: Instant,
    since: Instant,
    win_datagrams: u64,
    win_samples: u64,
    win_other: u64,
    win_lost: u64,
    win_dropped: u64,
    /// Samples discarded while the receiver that owns this ring was keyed.
    /// Counted apart from `win_dropped` because they are not a fault: see
    /// [`RxStats::on_dropped_keyed`].
    win_keyed: u64,
    total_datagrams: u64,
    total_samples: u64,
    total_lost: u64,
    total_dropped: u64,
    total_keyed: u64,
}

impl RxStats {
    pub(crate) fn new(proto: u8, nominal_hz: f64) -> Self {
        RxStats {
            proto,
            nominal_hz,
            started: Instant::now(),
            since: Instant::now(),
            win_datagrams: 0,
            win_samples: 0,
            win_other: 0,
            win_lost: 0,
            win_dropped: 0,
            win_keyed: 0,
            total_datagrams: 0,
            total_samples: 0,
            total_lost: 0,
            total_dropped: 0,
            total_keyed: 0,
        }
    }

    /// Record a decoded I/Q datagram carrying `pairs` complex samples.
    pub(crate) fn on_iq(&mut self, pairs: usize) {
        self.win_datagrams += 1;
        self.total_datagrams += 1;
        self.win_samples += pairs as u64;
        self.total_samples += pairs as u64;
    }

    /// Record a datagram that was not a recognized I/Q frame.
    pub(crate) fn on_other(&mut self) {
        self.win_other += 1;
    }

    /// Record `n` datagrams the radio sent that never arrived (sequence gap).
    pub(crate) fn on_lost(&mut self, n: u64) {
        self.win_lost += n;
        self.total_lost += n;
    }

    /// Record `pairs` complex samples discarded because the RX ring was full
    /// while the receiver was being read — a genuine overrun.
    pub(crate) fn on_dropped(&mut self, pairs: usize) {
        self.win_dropped += pairs as u64;
        self.total_dropped += pairs as u64;
    }

    /// Record `pairs` complex samples discarded because the RX ring was full
    /// while this receiver was keyed.
    ///
    /// The engine stops reading a keyed receiver for the length of the over and
    /// throws the backlog away on unkey (`IqSource::discard_pending_rx`), while
    /// the board keeps streaming RX regardless — neither protocol can pause it.
    /// So the ring fills within its own depth of keying down and every datagram
    /// after that is discarded: expected, at exactly the RX rate, for as long as
    /// the operator transmits. Counting these as overruns turned a normal over
    /// into a warning that blamed the DSP thread and advised a lower sample
    /// rate, and left the running total reading as transmit time.
    pub(crate) fn on_dropped_keyed(&mut self, pairs: usize) {
        self.win_keyed += pairs as u64;
        self.total_keyed += pairs as u64;
    }

    /// The board's sample clock measured against the host's, in ppm, once
    /// enough time has passed for the figure to mean anything. The same
    /// oscillator drives the NCO, so this is also the tuning error in ppm: a
    /// board reading a few ppm here cannot be more than a few Hz off frequency
    /// at HF, which rules a clock fault in or out on the spot.
    fn clock_error(&self) -> String {
        let dt = self.started.elapsed().as_secs_f64();
        // Below ~20 s the host's own scheduling jitter dominates the figure.
        if dt < 20.0 || self.total_samples == 0 || self.nominal_hz <= 0.0 {
            return "clock: measuring".to_string();
        }
        let measured = self.total_samples as f64 / dt;
        let ppm = (measured / self.nominal_hz - 1.0) * 1e6;
        format!(
            "clock: {measured:.0} sps, {ppm:+.0} ppm vs nominal (≈{:+.0} Hz at 14 MHz)",
            14.0e6 * ppm / 1e6
        )
    }

    /// What was discarded while keyed in this window, phrased so it cannot be
    /// read as a fault. Empty when the operator did not transmit.
    fn keyed_note(&self) -> String {
        if self.win_keyed == 0 {
            return String::new();
        }
        format!(
            "; {} sample(s) discarded while keyed (expected — RX is not read during \
             transmit); {} discarded while keyed in total",
            self.win_keyed, self.total_keyed,
        )
    }

    /// Emit a throughput line if the reporting interval has elapsed. Lost or
    /// dropped samples raise it to `warn`: both corrupt the audio and the
    /// waterfall, and neither is otherwise visible from the outside. Samples
    /// discarded while keyed do not (see [`RxStats::on_dropped_keyed`]) — they
    /// ride along on whichever line is emitted.
    pub(crate) fn tick(&mut self) {
        let dt = self.since.elapsed();
        if dt < STATS_INTERVAL {
            return;
        }
        let ksps = self.win_samples as f64 / dt.as_secs_f64() / 1000.0;
        if self.win_lost > 0 || self.win_dropped > 0 {
            tracing::warn!(
                "HPSDR P{} RX: {} datagrams, {} samples ({:.1} ksps) over {:.2}s; \
                 {} datagram(s) LOST on the network, {} sample(s) DROPPED (RX ring full — \
                 the DSP thread is not keeping up; try a lower sample rate); \
                 totals {} lost / {} dropped{}",
                self.proto,
                self.win_datagrams,
                self.win_samples,
                ksps,
                dt.as_secs_f64(),
                self.win_lost,
                self.win_dropped,
                self.total_lost,
                self.total_dropped,
                self.keyed_note(),
            );
        } else {
            tracing::debug!(
                "HPSDR P{} RX: {} datagrams, {} samples ({:.1} ksps) over {:.2}s; \
                 {} unrecognized; totals {} datagrams / {} samples; {}{}",
                self.proto,
                self.win_datagrams,
                self.win_samples,
                ksps,
                dt.as_secs_f64(),
                self.win_other,
                self.total_datagrams,
                self.total_samples,
                self.clock_error(),
                self.keyed_note(),
            );
        }
        self.since = Instant::now();
        self.win_datagrams = 0;
        self.win_samples = 0;
        self.win_other = 0;
        self.win_lost = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
    }
}

/// Follows a radio→host datagram sequence counter and reports how many
/// datagrams went missing. UDP loss on a busy 100 Mbit link is the usual cause
/// of a stream that runs at the right rate but sounds torn up, and nothing else
/// in the pipeline can tell a gap from real signal.
pub(crate) struct SeqTracker {
    next: Option<u32>,
}

impl SeqTracker {
    pub(crate) fn new() -> Self {
        SeqTracker { next: None }
    }

    /// Feed the sequence number of a freshly received datagram; returns the
    /// number of datagrams missing before it. A backwards or wildly forward jump
    /// (radio restarted its counter, or a reordered datagram) resynchronizes
    /// silently rather than reporting a nonsense gap.
    pub(crate) fn observe(&mut self, seq: u32) -> u32 {
        let lost = match self.next {
            None => 0,
            Some(expected) => {
                let gap = seq.wrapping_sub(expected);
                if gap > u32::MAX / 2 { 0 } else { gap }
            }
        };
        self.next = Some(seq.wrapping_add(1));
        lost
    }
}

/// Push one datagram's interleaved I/Q into the RX ring, keeping I and Q
/// paired: if the ring cannot take the whole datagram, the datagram is dropped
/// whole. Pushing what fits would leave the ring one float out of step, which
/// swaps I with Q for the rest of the session — a mirrored, unusable spectrum
/// that looks like a protocol bug rather than the buffer overrun it is.
///
/// `keyed` says whether *this ring's* receiver is transmitting, which decides
/// how a full ring is accounted for — a fault, or the normal cost of an over.
/// It is deliberately not a reason to skip the push: the samples are still
/// offered, and it is the reader's business whether it wants them.
///
/// Returns whether the datagram was taken, which is how a caller tells that the
/// reader has come back and started draining again.
pub(crate) fn push_iq(
    rx: &mut Producer<f32>,
    iq: &[f32],
    stats: &mut RxStats,
    keyed: bool,
) -> bool {
    // A chunk write is all-or-nothing, so the ring can never end up holding a
    // partial datagram however the timing falls.
    let Ok(mut chunk) = rx.write_chunk(iq.len()) else {
        if keyed {
            stats.on_dropped_keyed(iq.len() / 2);
        } else {
            stats.on_dropped(iq.len() / 2);
        }
        return false;
    };
    let (head, tail) = chunk.as_mut_slices();
    head.copy_from_slice(&iq[..head.len()]);
    tail.copy_from_slice(&iq[head.len()..]);
    chunk.commit_all();
    true
}

#[derive(Debug, thiserror::Error)]
pub enum HpsdrError {
    #[error("network I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

/// Control messages from the handles to the network thread.
pub(crate) enum Ctrl {
    /// A DDC stream coming up: store its ring + liveness clock, enable it on
    /// the board and (re)state the DDC configuration. Protocol 1 has exactly
    /// one stream, DDC 0.
    Attach {
        ddc: u8,
        ring: Producer<f32>,
        last_rx_ms: RxClock,
        /// Set while *this stream's* engine is transmitting and therefore not
        /// reading it — see `IqSource::set_rx_paused`. Per DDC rather than per
        /// connection: a second radio tab on another DDC is a different engine
        /// keying at a different time, and one of them being on the air says
        /// nothing about whether the other's ring is being drained.
        rx_paused: Arc<AtomicBool>,
    },
    /// A DDC stream going away (its radio tab closed, or its source is being
    /// rebuilt).
    Detach {
        ddc: u8,
    },
    RxFreq {
        ddc: u8,
        hz: f64,
    },
    /// Where the *dial* is, when a transverter has left the radio somewhere
    /// else. Only the accessory board's band code reads it: a decoder driving
    /// filters and antenna relays has to follow the band on the air, not the
    /// I.F. the radio is tuned to (issue #278). `None` puts it back on the
    /// hardware frequency, which is the ordinary case.
    BandDial(Option<f64>),
    /// Front-end LNA gain in dB (Hermes-Lite 2 only; ignored elsewhere).
    RxGain(f64),
    /// Where the radio *would* transmit, sent while receiving so an accessory
    /// board can switch bands before the operator keys. Loads the TX NCO
    /// register too, which is harmless: it does nothing until MOX.
    TxFreq(f64),
    TxOn(f64),
    TxOff,
    Shutdown,
}

/// Liveness shared between a protocol thread and its handle: milliseconds since
/// the connection opened at the moment the radio last delivered I/Q, or 0 if it
/// never has. Written by the network thread rather than by the reader, so a long
/// transmission — during which nothing drains the RX ring — is not mistaken for
/// a radio that has gone away.
pub(crate) type RxClock = Arc<AtomicU64>;

/// Everything a protocol thread needs: the socket, the radio address, the
/// rates, the TX ring and the control channel. The RX rings arrive per DDC
/// with [`Ctrl::Attach`] — streams come and go while the connection runs.
pub(crate) struct ThreadCtx {
    pub socket: UdpSocket,
    pub radio: IpAddr,
    /// Epoch for every stream's liveness clock.
    pub opened_at: Instant,
    /// This connection's ownership ticket (see [`owns_connection`]).
    pub conn_id: u64,
    /// Board name from discovery — decides which board-specific registers the
    /// Protocol 1 thread is allowed to write.
    pub board: String,
    pub rate_hz: f64,
    /// Initial front-end LNA gain (dB) for boards that have one.
    pub lna_gain_db: f64,
    /// How the seven open-collector outputs are driven: a preset's convention
    /// or the operator's own per-band table (see `HpsdrConfig::oc_plan`).
    pub oc: HpsdrOcPlan,
    /// Conjugate I/Q in both directions (see `HpsdrConfig::invert_spectrum`).
    pub invert_spectrum: bool,
    /// Switch on the Hermes-Lite's onboard PA (see `HpsdrConfig::pa_enable`).
    /// Ignored on boards that are not a Hermes-Lite.
    pub pa_enable: bool,
    /// Where an HL2IOBoard on the accessory bus takes its receive signal from
    /// (see `HpsdrConfig::io_rx_input`). Ignored when no such board answers.
    pub io_rx_input: HpsdrIoRxInput,
    /// The radio's own PTT line, published for [`HpsdrRx::radio_ptt`].
    pub radio_ptt: Arc<AtomicBool>,
    /// The board's temperature in hundredths of a degree Celsius, published for
    /// [`HpsdrRx::pa_temp_c`]. [`TEMP_UNKNOWN`] until the board reports one —
    /// which most of them never do.
    pub temp_centi_c: Arc<AtomicI32>,
    pub tx: Consumer<f32>,
    pub ctrl: Receiver<Ctrl>,
}

/// What [`ThreadCtx::temp_centi_c`] holds before a board has reported a
/// temperature, and forever on the boards that have no sensor.
///
/// A sentinel rather than an `Option` because the value is shared with the
/// network thread through an atomic, and a temperature that is genuinely absent
/// has to be told apart from one that is genuinely 0 °C — a Hermes-Lite in a
/// cold shack in February reads exactly that.
pub const TEMP_UNKNOWN: i32 = i32::MIN;

/// What every stream of one connection shares. Dropping the last handle stops
/// the stream and shuts the network thread down.
struct DevInner {
    ctrl: Sender<Ctrl>,
    /// Cleared when the network thread exits (socket error, shutdown), so
    /// owners can tell a live connection from a dead one.
    alive: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
    /// Board name reported by discovery (or "HPSDR" if it did not answer).
    board: String,
    /// OpenHPSDR protocol in use (1 or 2).
    protocol: u8,
    /// Actual RX sample rate in Hz.
    sample_rate_hz: f64,
    /// Actual TX I/Q rate in Hz.
    tx_rate_hz: f64,
    /// Front-end LNA gain in dB currently commanded (Hermes-Lite 2 only).
    /// Interior-mutable: it is set through shared stream handles.
    lna_gain_db: Mutex<f64>,
    /// Epoch for every stream's liveness clock (see [`HpsdrRx::silent_for`]).
    opened_at: Instant,
    /// Set while keyed. A half-duplex board can legitimately stop sending I/Q
    /// for the length of an over (an FT8 burst is 12.6 s), which must not be
    /// read as a dead link.
    transmitting: Arc<AtomicBool>,
    /// The state of the radio's own PTT line — a foot switch, a mic button, or
    /// whatever else is wired to the board's PTT input — as the network thread
    /// last saw it reported. A *level*, so a poll can never miss an edge by
    /// arriving late.
    radio_ptt: Arc<AtomicBool>,
    /// The board's own temperature, hundredths of a degree — see
    /// [`TEMP_UNKNOWN`].
    temp_centi_c: Arc<AtomicI32>,
    /// The TX ring's feed end, claimable exactly once — by DDC 0's stream.
    tx_endpoint: Mutex<Option<Producer<f32>>>,
    /// Which DDCs have a live [`HpsdrRx`], so one cannot be vended twice: two
    /// engines draining one ring would each get half the samples.
    attached: Mutex<std::collections::HashSet<u8>>,
}

impl Drop for DevInner {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.lock().expect("join lock").take() {
            let _ = j.join();
        }
    }
}

/// A live connection to an HPSDR radio, shared by every DDC stream on it.
/// Cheap to clone; the connection stops when the last clone (and every
/// [`HpsdrRx`]) is gone.
#[derive(Clone)]
pub struct HpsdrBoard {
    inner: Arc<DevInner>,
}

impl HpsdrBoard {
    /// Open a connection to `ip`, auto-detecting the protocol from a discovery
    /// probe (both P1 and P2 requests are sent) and starting the board at
    /// `sample_rate_hz`. No streams yet — each DDC's starts when
    /// [`HpsdrBoard::rx`] vends it. A manual IP that does not answer the
    /// probe is still tried as Protocol 2. `lna_gain_db` is the initial
    /// front-end gain for boards that have a settable one (see
    /// [`board_has_lna_gain`]); it is ignored on boards that do not.
    pub fn open(
        ip: Ipv4Addr,
        sample_rate_hz: f64,
        lna_gain_db: f64,
        oc: HpsdrOcPlan,
        invert_spectrum: bool,
        pa_enable: bool,
        io_rx_input: HpsdrIoRxInput,
    ) -> Result<HpsdrBoard, HpsdrError> {
        tracing::info!("HPSDR: opening {ip}, requested RX rate {sample_rate_hz:.0} Hz");
        let (board, protocol) = match discovery::probe(ip, Duration::from_millis(800)) {
            Some(dev) => {
                tracing::info!(
                    "HPSDR: {ip} answered probe: board \"{}\", Protocol {}, MAC {}, {}",
                    dev.board,
                    dev.protocol,
                    dev.mac,
                    if dev.in_use { "IN USE" } else { "idle" }
                );
                remember_probe(ip, &dev.board, dev.protocol);
                (dev.board, dev.protocol)
            }
            // A silent probe does not mean "Protocol 2". The usual cause is a
            // reopen while the previous handle is still streaming: the board
            // answers discovery to whichever socket last spoke to it, and a
            // Protocol 1 thread talks to port 1024 several hundred times a
            // second, so the reply lands on the *old* socket and this probe
            // times out. Guessing Protocol 2 there starts the wrong framing
            // against a Protocol 1 board and the connection never produces I/Q.
            None => match recall_probe(ip) {
                Some((board, protocol)) => {
                    tracing::warn!(
                        "HPSDR: {ip} did not answer the discovery probe; reusing what it \
                         reported last time — board \"{board}\", Protocol {protocol}. This is \
                         normal when reconnecting while the previous connection is still \
                         streaming."
                    );
                    (board, protocol)
                }
                None => {
                    tracing::warn!(
                        "HPSDR: {ip} did not answer the discovery probe and has never answered \
                         one; assuming Protocol 2. If this board is a Hermes-Lite 2 or other \
                         Protocol 1 device, RX will not start — check the IP and that no other \
                         program holds the radio."
                    );
                    ("HPSDR".to_string(), 2)
                }
            },
        };

        let rate = clamp_rate(sample_rate_hz, protocol);
        if (rate - sample_rate_hz).abs() > 1.0 {
            tracing::info!(
                "HPSDR: requested {sample_rate_hz:.0} Hz rounded to nearest Protocol {protocol} \
                 rate {rate:.0} Hz"
            );
        }
        // Both protocols take TX I/Q at 48 kHz: Protocol 2 through the DUC, and
        // Protocol 1 through the EP2 frames, whose sample rate is fixed at
        // 48 kHz regardless of the RX rate.
        let tx_rate = TX_RATE_HZ as f64;
        let lna_gain_db = lna_gain_db.clamp(LNA_GAIN_MIN_DB, LNA_GAIN_MAX_DB);
        if board_has_lna_gain(&board) {
            tracing::info!("HPSDR: initial {LNA_GAIN_ELEMENT} gain {lna_gain_db:+.0} dB");
        }
        // State the sideband convention either way — it is the setting that
        // decides whether anything demodulates at all, so a log that goes quiet
        // about it is no help when a board turns out to want the other one.
        tracing::info!(
            "HPSDR: spectrum {} (I/Q {}conjugated, receive and transmit alike)",
            if invert_spectrum { "INVERTED" } else { "normal" },
            if invert_spectrum { "" } else { "not " },
        );
        if oc.drives_anything() {
            tracing::info!(
                "HPSDR: driving the open-collector outputs from {} — check nothing else \
                 (amplifier PTT, antenna relays) is wired to those pins",
                oc.describe()
            );
        }
        // The setting that decides whether a keyed Hermes-Lite makes any power
        // at all, so it is stated at every open rather than left to be inferred.
        if board_is_hermes_lite(&board) {
            if pa_enable {
                tracing::info!("HPSDR: onboard PA ENABLED — transmit goes out of the antenna jack");
            } else {
                tracing::info!(
                    "HPSDR: onboard PA DISABLED — transmit appears at the low-power RF1 output \
                     only and the T/R relay stays in receive, so the antenna jack makes no power. \
                     Turn the PA on in Settings → Device unless an external amplifier is driven \
                     from RF1."
                );
            }
        }

        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(2)))?;
        tracing::debug!(
            "HPSDR: bound local UDP socket {}",
            socket.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into())
        );

        // TX ring ~0.5 s at the TX rate; each DDC's RX ring is created when
        // its stream attaches.
        let tx_cap = ((tx_rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 15);
        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(tx_cap);
        tracing::debug!("HPSDR: TX ring {tx_cap} floats (~0.5 s @ {tx_rate:.0} Hz)");

        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let opened_at = Instant::now();
        // Take ownership of the radio before the thread starts. Whatever
        // connection was driving it is superseded from here, so when the engine
        // drops it a moment from now it will leave the stream alone.
        let conn_id = claim_connection(IpAddr::V4(ip));
        let radio_ptt = Arc::new(AtomicBool::new(false));
        let temp_centi_c = Arc::new(AtomicI32::new(TEMP_UNKNOWN));
        let ctx = ThreadCtx {
            socket,
            radio: IpAddr::V4(ip),
            opened_at,
            conn_id,
            board: board.clone(),
            rate_hz: rate,
            lna_gain_db,
            oc,
            invert_spectrum,
            pa_enable,
            io_rx_input,
            radio_ptt: Arc::clone(&radio_ptt),
            temp_centi_c: Arc::clone(&temp_centi_c),
            tx: tx_cons,
            ctrl: ctrl_rx,
        };
        tracing::info!(
            "HPSDR: starting Protocol {protocol} network thread to {ip} \
             (board \"{board}\", RX {rate:.0} Hz, TX {tx_rate:.0} Hz)"
        );
        let alive = Arc::new(AtomicBool::new(true));
        let thread_alive = Arc::clone(&alive);
        let join = std::thread::Builder::new()
            .name("sdroxide-hpsdr".into())
            .spawn(move || {
                match protocol {
                    1 => protocol1::run(ctx),
                    _ => protocol2::run(ctx),
                }
                thread_alive.store(false, Ordering::Relaxed);
            })
            .map_err(|e| HpsdrError::Msg(format!("spawn network thread: {e}")))?;

        Ok(HpsdrBoard {
            inner: Arc::new(DevInner {
                ctrl: ctrl_tx,
                alive,
                join: Mutex::new(Some(join)),
                board,
                protocol,
                sample_rate_hz: rate,
                tx_rate_hz: tx_rate,
                lna_gain_db: Mutex::new(lna_gain_db),
                opened_at,
                transmitting: Arc::new(AtomicBool::new(false)),
                radio_ptt,
                temp_centi_c,
                tx_endpoint: Mutex::new(Some(tx_prod)),
                attached: Mutex::new(std::collections::HashSet::new()),
            }),
        })
    }

    /// Whether the network thread is still running. It stops on a socket error
    /// or shutdown, at which point this connection is dead and only a fresh
    /// [`HpsdrBoard::open`] revives it.
    pub fn is_alive(&self) -> bool {
        self.inner.alive.load(Ordering::Relaxed)
    }

    /// Board name reported by discovery (or "HPSDR" if it did not answer).
    pub fn board(&self) -> &str {
        &self.inner.board
    }

    /// The board's own temperature in degrees Celsius, where it reports one.
    ///
    /// A Hermes-Lite 2 does, on the analogue input its status frames carry;
    /// nothing else in the HPSDR family that this driver has met reports a
    /// temperature at all, so `None` is the ordinary answer. See
    /// `protocol1::hl2_temperature_c` for what the sensor actually measures.
    pub fn pa_temp_c(&self) -> Option<f32> {
        match self.inner.temp_centi_c.load(Ordering::Relaxed) {
            TEMP_UNKNOWN => None,
            centi => Some(centi as f32 / 100.0),
        }
    }

    /// OpenHPSDR protocol in use (1 or 2).
    pub fn protocol(&self) -> u8 {
        self.inner.protocol
    }

    /// Actual RX sample rate in Hz.
    pub fn sample_rate_hz(&self) -> f64 {
        self.inner.sample_rate_hz
    }

    /// How many DDCs this connection can serve: the Protocol 2 framing's
    /// eight, or Protocol 1's one (its frame layout carries a single receiver
    /// here).
    pub fn ddc_count(&self) -> u8 {
        match self.inner.protocol {
            2 => protocol2::MAX_DDCS,
            _ => 1,
        }
    }

    /// Whether this board has a front-end gain this crate can command.
    pub fn has_lna_gain(&self) -> bool {
        board_has_lna_gain(&self.inner.board)
    }

    /// Attach DDC `ddc` and start its stream. Refused beyond
    /// [`HpsdrBoard::ddc_count`] — asking a Protocol 1 board for a second
    /// receiver — and for a DDC that already has a live stream: two engines
    /// draining one ring would each get half the samples.
    pub fn rx(&self, ddc: u8) -> Result<HpsdrRx, HpsdrError> {
        if ddc >= self.ddc_count() {
            return Err(HpsdrError::Msg(format!(
                "DDC {} does not exist: {} (Protocol {}) has {} receiver(s) here",
                ddc + 1,
                self.inner.board,
                self.inner.protocol,
                self.ddc_count()
            )));
        }
        {
            let mut attached = self.inner.attached.lock().expect("attached lock");
            if !attached.insert(ddc) {
                return Err(HpsdrError::Msg(format!(
                    "DDC {} is already running as another radio",
                    ddc + 1
                )));
            }
        }
        // RX ring ~0.5 s at the RX rate.
        let rx_cap =
            ((self.inner.sample_rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (ring_prod, ring_cons) = RingBuffer::<f32>::new(rx_cap);
        // The liveness clock starts at "data seen now": a stream attached to a
        // long-running connection must age from its attach, not from the
        // connection's epoch, or it would count as silent before its first
        // datagram had a chance to arrive.
        let last_rx_ms: RxClock =
            Arc::new(AtomicU64::new(self.inner.opened_at.elapsed().as_millis() as u64));
        // DDC 0's stream owns the transmitter; there is exactly one TX feed.
        let tx =
            if ddc == 0 { self.inner.tx_endpoint.lock().expect("tx lock").take() } else { None };
        let rx_paused = Arc::new(AtomicBool::new(false));
        let _ = self.inner.ctrl.send(Ctrl::Attach {
            ddc,
            ring: ring_prod,
            last_rx_ms: last_rx_ms.clone(),
            rx_paused: Arc::clone(&rx_paused),
        });
        Ok(HpsdrRx {
            dev: Arc::clone(&self.inner),
            ddc,
            ring: ring_cons,
            tx,
            last_rx_ms,
            rx_paused,
        })
    }
}

/// One DDC's stream on a shared [`HpsdrDevice`]: its IQ ring, its NCO, and —
/// on DDC 0 — the transmitter. Dropping it stops this stream; the connection
/// lives on for whoever else holds it.
pub struct HpsdrRx {
    dev: Arc<DevInner>,
    ddc: u8,
    ring: Consumer<f32>,
    /// TX I/Q into the network thread — DDC 0 only.
    tx: Option<Producer<f32>>,
    /// This stream's liveness clock (ms since the connection opened at the
    /// last datagram, stamped by the network thread).
    last_rx_ms: RxClock,
    /// See [`Ctrl::Attach::rx_paused`].
    rx_paused: Arc<AtomicBool>,
}

impl std::fmt::Debug for HpsdrRx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HpsdrRx")
            .field("ddc", &self.ddc)
            .field("board", &self.dev.board)
            .finish_non_exhaustive()
    }
}

impl Drop for HpsdrRx {
    fn drop(&mut self) {
        self.dev.attached.lock().expect("attached lock").remove(&self.ddc);
        // Hand the TX feed back so a rebuilt DDC-0 stream (Settings → Apply)
        // can claim it again on the same connection.
        if let Some(tx) = self.tx.take() {
            *self.dev.tx_endpoint.lock().expect("tx lock") = Some(tx);
        }
        let _ = self.dev.ctrl.send(Ctrl::Detach { ddc: self.ddc });
    }
}

impl HpsdrRx {
    /// Which DDC this stream is (0-based, as the wire counts them).
    pub fn ddc(&self) -> u8 {
        self.ddc
    }

    /// See [`HpsdrBoard::is_alive`].
    pub fn is_alive(&self) -> bool {
        self.dev.alive.load(Ordering::Relaxed)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.dev.sample_rate_hz
    }

    pub fn board(&self) -> &str {
        &self.dev.board
    }

    pub fn protocol(&self) -> u8 {
        self.dev.protocol
    }

    /// Retune this DDC's NCO.
    pub fn set_rx_freq(&self, hz: f64) {
        tracing::debug!("HPSDR: set DDC{} freq {hz:.0} Hz", self.ddc);
        let _ = self.dev.ctrl.send(Ctrl::RxFreq { ddc: self.ddc, hz });
    }

    /// Tell the accessory board's band decoder where the dial is, which is not
    /// where the radio is tuned when a transverter is in front of it. Only
    /// DDC 0 — the accessory board is the *board's*, not a stream's.
    pub fn set_band_dial(&self, hz: Option<f64>) {
        if self.ddc == 0 {
            let _ = self.dev.ctrl.send(Ctrl::BandDial(hz));
        }
    }

    /// Whether this board has a front-end gain this crate can command.
    pub fn has_lna_gain(&self) -> bool {
        board_has_lna_gain(&self.dev.board)
    }

    /// The front-end LNA gain currently commanded, in dB.
    pub fn lna_gain_db(&self) -> f64 {
        *self.dev.lna_gain_db.lock().expect("lna lock")
    }

    /// Set the front-end LNA gain in dB. No-op on boards without one. The gain
    /// is the one analogue front end, shared by every DDC — any stream may set
    /// it, and all of them hear the result.
    pub fn set_lna_gain_db(&mut self, db: f64) {
        if !self.has_lna_gain() {
            return;
        }
        let db = db.clamp(LNA_GAIN_MIN_DB, LNA_GAIN_MAX_DB);
        *self.dev.lna_gain_db.lock().expect("lna lock") = db;
        tracing::debug!("HPSDR: set {LNA_GAIN_ELEMENT} gain {db:+.0} dB");
        let _ = self.dev.ctrl.send(Ctrl::RxGain(db));
    }

    /// Tell the board where this radio would transmit, while receiving.
    ///
    /// An amplifier, transverter or loop antenna on the accessory bus switches
    /// bands from this and has to be settled before any RF appears, so it
    /// cannot wait for [`Self::tx_begin`]. Belongs to the stream that owns the
    /// transmitter; there is only one, and only one transmit frequency.
    pub fn set_tx_freq(&self, hz: f64) {
        if self.tx.is_some() {
            let _ = self.dev.ctrl.send(Ctrl::TxFreq(hz));
        }
    }

    /// Begin transmitting at `tx_freq_hz`; returns the TX I/Q rate to feed
    /// [`Self::tx_write`]. The transmitter is DDC 0's — a no-op on any other
    /// stream, which the engine never asks anyway (their capabilities carry no
    /// TX).
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        if self.tx.is_some() {
            tracing::info!(
                "HPSDR: TX begin at {tx_freq_hz:.0} Hz ({:.0} Hz I/Q)",
                self.dev.tx_rate_hz
            );
            self.dev.transmitting.store(true, Ordering::Relaxed);
            let _ = self.dev.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        }
        self.dev.tx_rate_hz
    }

    /// The state of the radio's own PTT line — a foot switch, a mic button, or
    /// whatever is wired to the board's PTT input (a Hermes-Lite 2 reports the
    /// ring of its CN4 jack here). `true` is keyed.
    ///
    /// Answered as a level rather than an event, so a caller polling at its own
    /// pace cannot miss a press by asking at the wrong moment. Only the stream
    /// that owns the transmitter reports it: the board has one PTT input and
    /// one transmitter, and a second DDC's radio keying on it would put a
    /// receiver-only tab on the air.
    ///
    /// Protocol 1 only for now — nothing decodes the Protocol 2 status packets
    /// for it yet, so a P2 board always answers `false`.
    pub fn radio_ptt(&self) -> bool {
        self.tx.is_some() && self.dev.radio_ptt.load(Ordering::Relaxed)
    }

    /// The board's own temperature, where it reports one — see
    /// [`HpsdrBoard::pa_temp_c`].
    ///
    /// Answered on every stream, not only the one that owns the transmitter:
    /// the board has one temperature and every tab looking at it is looking at
    /// the same hardware getting hot.
    pub fn pa_temp_c(&self) -> Option<f32> {
        match self.dev.temp_centi_c.load(Ordering::Relaxed) {
            crate::net::TEMP_UNKNOWN => None,
            centi => Some(centi as f32 / 100.0),
        }
    }

    /// Stop transmitting.
    pub fn tx_end(&self) {
        if self.tx.is_some() {
            tracing::info!("HPSDR: TX end");
            self.dev.transmitting.store(false, Ordering::Relaxed);
            let _ = self.dev.ctrl.send(Ctrl::TxOff);
        }
    }

    /// Push interleaved I,Q TX samples (at the device's TX rate). Blocks
    /// briefly when the ring is full (pacing the caller); drops if the thread
    /// stalls, and drops silently on a stream that has no transmitter.
    ///
    /// Writes go in whole I/Q pairs, so the ring always holds an even number of
    /// floats. Giving up mid-pair would put every later sample one slot out of
    /// step — the network thread would read each Q as an I, transmitting the
    /// wrong sideband for the rest of the over.
    pub fn tx_write(&mut self, iq: &[f32]) {
        let Some(tx) = self.tx.as_mut() else { return };
        for pair in iq.chunks_exact(2) {
            let mut tries = 0u32;
            let mut chunk = loop {
                match tx.write_chunk(2) {
                    Ok(c) => break c,
                    Err(_) => {
                        if tries > 2000 {
                            return; // network thread stalled — drop rather than hang
                        }
                        tries += 1;
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
            };
            // The pair may straddle the ring's wrap point, so it arrives as two
            // slices; together they are always exactly the two slots.
            let (head, tail) = chunk.as_mut_slices();
            for (slot, &v) in head.iter_mut().chain(tail.iter_mut()).zip(pair) {
                *slot = v;
            }
            chunk.commit_all();
        }
    }

    /// How long this stream has gone without I/Q, measured from the last
    /// datagram the network thread decoded for this DDC or — if none arrived
    /// yet — from when the stream attached. A stream that never starts is the
    /// failure that matters most here (wrong protocol guessed, or the board
    /// still held by a previous connection), so it has to age just like one
    /// that stops. Always zero while transmitting.
    pub fn silent_for(&self) -> Duration {
        if self.dev.transmitting.load(Ordering::Relaxed) {
            return Duration::ZERO;
        }
        let since_open = self.dev.opened_at.elapsed();
        let last = Duration::from_millis(self.last_rx_ms.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }

    /// Drain interleaved I,Q floats from the RX ring into `out`. Always returns
    /// an even count (never splits an I/Q pair, so the stream stays aligned).
    /// Returns 0 when no data is available yet.
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

    /// Drop whatever the network thread queued in the RX ring. The radio
    /// keeps streaming I/Q for the whole over, so `rx_read` would otherwise
    /// replay a stale backlog after `tx_end`.
    pub fn discard_pending_rx(&mut self) {
        while self.ring.pop().is_ok() {}
    }

    /// Tell the network thread that the engine has stopped reading this stream
    /// for an over, and then that it has started again — see
    /// `IqSource::set_rx_paused`.
    ///
    /// Distinct from the board's own MOX, which the protocol threads already
    /// watch: that says *this* radio is transmitting, and covers the ordinary
    /// case where the DDC being keyed is the one being read. This covers the
    /// other one — an HPSDR receiver lent to a different rig as a panadapter,
    /// where nothing on this connection is keyed at all and the ring still goes
    /// unread for the length of somebody else's over.
    pub fn set_rx_paused(&self, paused: bool) {
        self.rx_paused.store(paused, Ordering::Relaxed);
    }
}

/// Round a requested rate to the nearest rate valid for the protocol.
fn clamp_rate(hz: f64, protocol: u8) -> f64 {
    sdroxide_types::HpsdrConfig::rates_for(protocol)
        .iter()
        .copied()
        .min_by(|a, b| (a - hz).abs().partial_cmp(&(b - hz).abs()).unwrap())
        .unwrap_or(1_536_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lna_gain_wire_codes() {
        // The Hermes-Lite wire value is dB + 12, clamped to the 6-bit field.
        assert_eq!(lna_gain_code(-12.0), 0);
        assert_eq!(lna_gain_code(0.0), 12);
        assert_eq!(lna_gain_code(48.0), 60);
        assert_eq!(lna_gain_code(-100.0), 0);
        assert_eq!(lna_gain_code(100.0), 60);
        // Never overflows into the "field is valid" bit (0x40) above it.
        for db in [-12.0, -0.5, 20.0, 47.4, 48.0] {
            assert!(lna_gain_code(db) < 0x40, "{db} dB");
        }
    }

    #[test]
    fn only_hermes_lite_gets_the_gain_register() {
        assert!(board_has_lna_gain("Hermes-Lite 2"));
        assert!(!board_has_lna_gain("Hermes"));
        assert!(!board_has_lna_gain("Hermes2"));
        assert!(!board_has_lna_gain("Orion2"));
        assert!(!board_has_lna_gain("Saturn"));
    }

    #[test]
    fn sequence_gaps_are_counted_and_wrap_cleanly() {
        let mut s = SeqTracker::new();
        assert_eq!(s.observe(100), 0); // first datagram: nothing to compare
        assert_eq!(s.observe(101), 0);
        assert_eq!(s.observe(105), 3); // 102, 103, 104 never arrived
        assert_eq!(s.observe(106), 0);
        // Wrapping the counter is not a 4-billion-datagram gap.
        let mut s = SeqTracker::new();
        assert_eq!(s.observe(u32::MAX), 0);
        assert_eq!(s.observe(0), 0);
        // A datagram arriving late (or the radio restarting its count)
        // resynchronizes silently rather than reporting a nonsense gap.
        let mut s = SeqTracker::new();
        assert_eq!(s.observe(500), 0);
        assert_eq!(s.observe(400), 0);
        assert_eq!(s.observe(401), 0);
    }

    #[test]
    fn ring_overflow_drops_whole_datagrams() {
        // A ring with room for exactly 4 floats (rtrb rounds up to the request).
        let (mut prod, mut cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(1, 384_000.0);
        assert!(push_iq(&mut prod, &[1.0, 2.0], &mut stats, false));
        assert!(push_iq(&mut prod, &[3.0, 4.0], &mut stats, false));
        // The third datagram cannot fit and is dropped whole, not truncated.
        assert!(!push_iq(&mut prod, &[5.0, 6.0], &mut stats, false));
        let got: Vec<f32> = std::iter::from_fn(|| cons.pop().ok()).collect();
        assert_eq!(got, vec![1.0, 2.0, 3.0, 4.0]);
        // Alignment survived: every I landed on an even index.
        assert_eq!(got.len() % 2, 0);
    }

    #[test]
    fn partial_datagrams_never_split_a_pair() {
        // Room for 4 floats, then a 6-float datagram: it must be dropped rather
        // than have 4 of its floats pushed, which would leave the ring one slot
        // out of step and swap I with Q for good.
        let (mut prod, mut cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(1, 384_000.0);
        assert!(!push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &mut stats, false));
        assert!(cons.pop().is_err(), "nothing was pushed");
    }

    #[test]
    fn a_full_ring_while_keyed_is_not_counted_as_an_overrun() {
        // The engine stops reading its receiver for the length of an over, so
        // the ring fills and stays full until it unkeys. That is the normal
        // cost of transmitting, not a host that cannot keep up: it must not
        // reach the fault counters, which is what made a healthy station log a
        // warning per two seconds of transmit and a running total that only
        // measured how long the operator had been on the air.
        let (mut prod, mut cons) = RingBuffer::<f32>::new(4);
        let mut stats = RxStats::new(1, 384_000.0);
        assert!(push_iq(&mut prod, &[1.0, 2.0, 3.0, 4.0], &mut stats, true));
        assert!(!push_iq(&mut prod, &[5.0, 6.0], &mut stats, true));
        assert_eq!(stats.total_dropped, 0, "a keyed receiver reports no overruns");
        assert_eq!(stats.total_keyed, 1, "the discarded pair is accounted for as keyed");

        // Unkeyed, the very same full ring is a genuine overrun again.
        assert!(!push_iq(&mut prod, &[7.0, 8.0], &mut stats, false));
        assert_eq!(stats.total_dropped, 1);
        assert_eq!(stats.total_keyed, 1);

        // And a drained ring takes the next datagram, which is the signal that
        // the reader is back (see the callers' `tx_backlog`).
        while cons.pop().is_ok() {}
        assert!(push_iq(&mut prod, &[9.0, 10.0], &mut stats, false));
    }

    #[test]
    fn tx_ring_stays_pair_aligned_when_it_wraps() {
        // Write and drain repeatedly so the pairs straddle the ring's wrap
        // point, which is where a two-slice chunk write could go wrong.
        let (mut prod, mut cons) = RingBuffer::<f32>::new(6);
        let mut next = 0.0f32;
        for _ in 0..10 {
            for _ in 0..2 {
                let mut chunk = prod.write_chunk(2).expect("room for one pair");
                let (head, tail) = chunk.as_mut_slices();
                for (slot, v) in head.iter_mut().chain(tail.iter_mut()).zip([next, next + 1.0]) {
                    *slot = v;
                }
                chunk.commit_all();
                next += 2.0;
            }
            // Drain the same way the network thread does: whole pairs only.
            while let Ok(pair) = cons.read_chunk(2) {
                let (a, b) = pair.as_slices();
                let got: Vec<f32> = a.iter().chain(b).copied().collect();
                assert_eq!(got.len(), 2);
                // I is always even, Q always odd: the pairing never slipped.
                assert_eq!(got[0] as i32 % 2, 0, "I at {got:?}");
                assert_eq!(got[1] as i32 % 2, 1, "Q at {got:?}");
                pair.commit_all();
            }
        }
    }

    #[test]
    fn only_the_newest_connection_may_stop_a_radio() {
        // Reproduces the Apply/reconnect ordering: the replacement connection
        // opens while the old one is still live, and only then is the old one
        // dropped. If the departing connection were allowed to send the stop
        // command it would switch the radio off underneath its successor —
        // which is what left the board dead until sdroxide was restarted.
        let radio: IpAddr = "192.0.2.53".parse().unwrap();
        let first = claim_connection(radio);
        assert!(owns_connection(radio, first), "the only connection owns the radio");

        let second = claim_connection(radio);
        assert_ne!(first, second, "each connection gets its own ticket");
        assert!(!owns_connection(radio, first), "the superseded connection must stay quiet");
        assert!(owns_connection(radio, second), "the replacement owns the radio");

        // A second radio is tracked independently, so reconnecting one does not
        // silence the other.
        let other: IpAddr = "192.0.2.54".parse().unwrap();
        let other_id = claim_connection(other);
        assert!(owns_connection(other, other_id));
        assert!(owns_connection(radio, second), "unrelated radio left alone");
        // An unknown ticket never owns anything.
        assert!(!owns_connection(radio, u64::MAX));
        assert!(!owns_connection("192.0.2.99".parse().unwrap(), second));
    }

    #[test]
    fn a_lost_probe_falls_back_to_what_the_board_said_before() {
        // Reconnecting while the previous stream is still running loses the
        // discovery reply, and guessing "Protocol 2" there drives a Hermes-Lite
        // with the wrong framing — the connection comes up and never produces
        // I/Q, which is what made Apply/reconnect need two presses.
        let ip: Ipv4Addr = "192.0.2.53".parse().unwrap();
        assert_eq!(recall_probe(ip), None, "nothing known about a board never probed");
        remember_probe(ip, "Hermes-Lite 2", 1);
        assert_eq!(recall_probe(ip), Some(("Hermes-Lite 2".to_string(), 1)));
        // A later probe that says something different wins.
        remember_probe(ip, "Saturn", 2);
        assert_eq!(recall_probe(ip), Some(("Saturn".to_string(), 2)));
        // Other radios are unaffected.
        assert_eq!(recall_probe("192.0.2.54".parse().unwrap()), None);
    }

    #[test]
    fn tx_rate_is_48k_for_both_protocols() {
        // Protocol 1's EP2 stream is 48 kHz regardless of the DDC rate, so the
        // modulator must never be told to produce at the RX rate.
        assert_eq!(TX_RATE_HZ, 48_000);
    }
}
