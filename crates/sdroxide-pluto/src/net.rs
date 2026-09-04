//! The live connection to a Pluto: [`PlutoHandle`], the state its three
//! threads share, and the control thread that owns the AD9361.
//!
//! # Why three connections
//!
//! IIOD is strictly request/response on one socket, and `READBUF` blocks until
//! the device has filled a buffer — 63 ms at the lowest rate offered here. A
//! retune sharing that socket would queue behind it, so the dial would lag the
//! knob by a buffer. `iiod` is a thread-per-connection server, so instead this
//! opens three: control, receive and transmit, each owned by one blocking
//! thread. (`iiod` may still hold a per-device lock, so a retune issued
//! mid-buffer can wait that long inside the server; that is a bounded stall in
//! the right place, not a queue that grows.)
//!
//! # Half duplex by default, full duplex by choice
//!
//! The AD9361 is a full-duplex part — a synthesiser per direction in FDD — so
//! what decides this is the link, not the silicon. A Pluto is normally reached
//! over a USB 2.0 Ethernet gadget, which will not carry a megasample-per-second
//! stream in both directions at once, so by default receive is torn down for
//! the length of an over and the whole link is available to transmit: the same
//! trade the HPSDR backend makes.
//!
//! [`PlutoConfig::full_duplex`] takes that arbitration out, for a board with
//! real Ethernet behind it (a LibreSDR, a Pluto on a gigabit adapter). The two
//! buffers then stay open together and each thread reads its own socket, which
//! is what the three-connection layout above was already built for — and the
//! two buffers are two different IIO devices (`cf-ad9361-lpc` and
//! `cf-ad9361-dds-core-lpc`), so a per-device lock in the server does not
//! serialise them either. It is the operator's decision because only they can
//! see the link: the symptom of getting it wrong is not a refusal but a
//! transmit buffer that runs dry, which goes on the air as a chopped envelope.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};

use sdroxide_types::PlutoConfig;

use crate::context::Context;
use crate::error::{Error, Result};
use crate::iiod::Connection;
use crate::phy::Phy;
pub use crate::phy::PlutoLimits;
use crate::stream;
use crate::trace::Trace;

/// How long the TCP handshake may take before the address counts as wrong.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// The two enable-state-machine states a TDD board is driven between. Only
/// written on a board this driver put in TDD — see [`Shared::tdd`].
const ENSM_RX: &str = "rx";
const ENSM_TX: &str = "tx";

/// How often a stream thread emits a throughput line (`RUST_LOG=…=debug`).
pub(crate) const STATS_INTERVAL: Duration = Duration::from_secs(2);

/// How much airtime one transmit buffer should cover.
///
/// `WRITEBUF` is synchronous — command, status line, payload, status line — so
/// each buffer costs two round trips over the link on top of the payload
/// itself, and **the DAC has nothing queued while that is in flight**. The
/// buffer therefore has to outlast the round trip by a wide margin, or the
/// transmitted envelope is chopped at the buffer rate.
///
/// This used to be a flat 4096 samples, which is 1.64 ms at the default
/// 2.5 Msps. Measured on a Pluto reached over its USB Ethernet gadget the round
/// trip is ~2.2 ms — *longer than the buffer* — so the transmitter ran dry for
/// ~0.55 ms out of every 2.19 ms: a 456 Hz chop with about a quarter of the
/// modulation missing, unreadable in NFM (the discriminator sees an impulse at
/// every restart) and merely bad in SSB. Lowering the sample rate only narrowed
/// the gap, because the round trip is fixed and it was the buffer that shrank
/// with it — the tell that the buffer length, not the link, was the bottleneck.
///
/// 25 ms puts an order of magnitude between the two while still keying up
/// faster than an operator can hear.
const TX_BUFFER_MS: f64 = 25.0;

/// Floor and ceiling on [`tx_buffer_samples`], in complex samples. The floor
/// keeps a slow rate from producing a buffer so short that the per-buffer round
/// trip dominates again; the ceiling bounds the device-side allocation and the
/// key-up latency at rates a Pluto cannot stream over USB anyway.
const TX_BUFFER_BOUNDS: (usize, usize) = (4096, 1 << 17);

/// Transmit buffer length in complex samples at `rate_hz` — see
/// [`TX_BUFFER_MS`]. Rounded to a multiple of 1024 so the byte count stays
/// comfortably aligned for the device's DMA.
pub(crate) fn tx_buffer_samples(rate_hz: f64) -> usize {
    let want = (rate_hz * TX_BUFFER_MS / 1000.0) as usize;
    let want = want.clamp(TX_BUFFER_BOUNDS.0, TX_BUFFER_BOUNDS.1);
    (want / 1024 * 1024).max(TX_BUFFER_BOUNDS.0)
}

/// Control messages from the stream handles to the control thread.
pub(crate) enum Ctrl {
    /// Retune the receive LO. The AD9361's two receive chains share the one
    /// synthesiser, so this moves *every* attached stream's centre; `origin`
    /// names the chain that asked, and the others are told through their
    /// LO-watch channels — the asker already knows.
    RxFreq {
        hz: f64,
        origin: u8,
    },
    RxGain {
        chain: u8,
        db: f64,
    },
    AgcMode {
        chain: u8,
        mode: String,
    },
    RxPort {
        chain: u8,
        port: String,
    },
    TxPort(String),
    TxGain(f64),
    /// Reference trim in parts per million, applied in software to every LO we
    /// ask for. The device's own `xo_correction` is a persistent debug
    /// attribute, so writing it would outlive the session and surprise the next
    /// program to open the radio.
    Ppm(f64),
    TxOn(f64),
    TxOff,
    Shutdown,
}

/// State the three threads and the handle share.
pub(crate) struct Shared {
    pub phy: Phy,
    /// Where the device is, so the receive thread can redial its own socket
    /// without going back through [`PlutoRig::open`] — see
    /// [`crate::stream::redial_rx`].
    pub addr: SocketAddr,
    /// The *live* receive socket, kept here for [`RigInner::release`] to break
    /// a blocked read on.
    ///
    /// Not in `RigInner::shutdowns` with the other two, because this is the one
    /// connection that can be replaced under us: a handle taken at open goes
    /// stale the first time the receive thread redials, and shutting down a
    /// socket that is already closed leaves the *new* read blocked until its
    /// own deadline — a shutdown that no longer shuts anything down.
    pub rx_shutdown: Mutex<Option<std::net::TcpStream>>,
    /// Receive buffer should be open. Cleared for the length of an over.
    pub rx_enabled: AtomicBool,
    /// Receive buffer *is* open — the acknowledgement the control thread waits
    /// for before letting transmit have the link.
    pub rx_active: AtomicBool,
    pub tx_enabled: AtomicBool,
    /// Transmit buffer *is* open, the mirror of [`Self::rx_active`]: receive
    /// must not reclaim the link until the transmit buffer has actually been
    /// closed, or the two overlap on a link that has room for one.
    pub tx_active: AtomicBool,
    /// Cleared when any thread gives up, which is what `needs_reopen()` reads.
    pub alive: AtomicBool,
    pub opened_at: Instant,
    /// Milliseconds since [`Self::opened_at`] when the receive thread last
    /// decoded samples, or 0 if it never has. Written by the stream thread
    /// rather than the reader, so a long over is not mistaken for a dead link.
    pub last_rx_ms: AtomicU64,
    /// The second chain's clock, stamped only while its ring is attached.
    pub last_rx1_ms: AtomicU64,
    pub transmitting: AtomicBool,
    /// Receive and transmit may hold the link at the same time — see this
    /// module's header. Fixed for the life of the connection: it describes the
    /// network the radio is on, which does not change under it.
    pub full_duplex: bool,
    /// The part was put in TDD, so `ensm_mode` is this driver's to drive: the
    /// receiver is dead until it says `rx`, the transmitter until it says
    /// `tx`, and the GPO pins keying an external amplifier follow whichever it
    /// last said. False on a board left in FDD, where writing it would be
    /// wrong rather than merely redundant.
    pub tdd: bool,
    pub buffer_samples: usize,
    pub rate_hz: f64,
    /// The transmit path's own rate. The AD9361 clocks both directions
    /// together so this is normally [`Self::rate_hz`], but it is read back
    /// rather than assumed, and the transmit buffer is sized against it.
    pub tx_rate_hz: f64,
    /// Transmit buffer length in complex samples — see [`tx_buffer_samples`].
    pub tx_buffer_samples: usize,
    /// How many I/Q pairs the receive buffer should be open with (1 or 2).
    /// The receive thread reopens the buffer when this moves.
    pub rx_pairs: AtomicUsize,
    /// The second chain's ring feed, installed while its stream is attached.
    pub ring1: Mutex<Option<Producer<f32>>>,
    /// Per-chain LO-move subscribers: `(chain, notify)`. The control thread
    /// tells every chain but the one that asked when the shared LO moves.
    pub lo_watch: Mutex<Vec<(u8, Sender<f64>)>>,
    pub trace: Trace,
    /// Set while the engine is transmitting and therefore not reading this
    /// receiver — see `IqSource::set_rx_paused`. Read by the receive thread on
    /// every buffer so a ring that fills during an over is accounted for as the
    /// cost of transmitting rather than as an overrun. Only ever true when this
    /// Pluto is somebody else's panadapter: keying its own transmitter closes
    /// the receive buffer outright (`rx_enabled`), leaving nothing to discard.
    pub rx_paused: AtomicBool,
}

impl Shared {
    pub(crate) fn stamp_rx(&self) {
        self.last_rx_ms.store(self.opened_at.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    pub(crate) fn stamp_rx1(&self) {
        self.last_rx1_ms.store(self.opened_at.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    /// Report a thread's fatal error once and take the connection down, so the
    /// engine reopens instead of staring at a stream that will never resume.
    pub(crate) fn die(&self, what: &str, e: &Error) {
        if self.alive.swap(false, Ordering::Relaxed) {
            tracing::warn!("PlutoSDR: {what} stopped: {e}");
            self.trace.note(format!("!! {what} stopped: {e}"));
        }
    }

    /// Break the current receive socket at both ends and forget it.
    ///
    /// Both halves matter. The shutdown is what a server blocked writing into a
    /// wedged socket notices, and it is what starts `iiod` reaping the device
    /// buffer this client holds — the buffer a reconnect would otherwise meet
    /// as `-EBUSY`. Forgetting it is what stops [`RigInner::release`] later
    /// shutting down a socket nobody is on any more while the *live* one, by
    /// then a different socket entirely, goes untouched.
    pub(crate) fn drop_rx_socket(&self) {
        if let Some(sock) = self.rx_shutdown.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = sock.shutdown(std::net::Shutdown::Both);
        }
    }

    /// Adopt `conn`'s socket as the one [`RigInner::release`] should break.
    ///
    /// Refused once the connection is being torn down. The mutex is what makes
    /// that check sound: `release` clears `alive` *before* it takes this lock,
    /// so whichever of the two gets there first, the other sees its work — a
    /// handle installed in time is shut down, and one that missed the window is
    /// rejected here rather than left blocking a read nobody can reach.
    pub(crate) fn adopt_rx_socket(&self, conn: &Connection) -> Result<()> {
        let mut slot = self.rx_shutdown.lock().unwrap_or_else(|e| e.into_inner());
        if !self.alive.load(Ordering::Relaxed) {
            return Err(Error::Msg("this connection is closing".into()));
        }
        *slot = conn.shutdown_handle();
        Ok(())
    }
}

/// What every stream of one connection shares: the three threads, their
/// sockets, and the endpoints a stream claims when it attaches. The teardown
/// is [`RigInner::release`], run by the last handle out.
struct RigInner {
    ctrl: Sender<Ctrl>,
    shared: Arc<Shared>,
    joins: Mutex<Vec<JoinHandle<()>>>,
    /// One per connection, for waking its thread out of a blocked read on the
    /// way out. See [`RigInner::release`].
    shutdowns: Vec<std::net::TcpStream>,
    released: AtomicBool,
    /// Chain 0's ring endpoint, and the transmit feed — claimable exactly
    /// once each, by chain 0's stream, and handed back when it drops so a
    /// rebuilt stream (Settings → Apply) can claim them again.
    rx0: Mutex<Option<Consumer<f32>>>,
    tx0: Mutex<Option<Producer<f32>>>,
    /// Which chains have a live [`PlutoRx`], so one cannot be vended twice:
    /// two engines draining one ring would each get half the samples.
    attached: Mutex<std::collections::HashSet<u8>>,

    sample_rate_hz: f64,
    tx_rate_hz: f64,
    rf_bandwidth_hz: f64,
    limits: PlutoLimits,
    model: String,
    firmware: String,
    serial: String,
    addr: SocketAddr,
    /// The gains `open` configured, seeding each stream's cache.
    init_rx_gain_db: f64,
    init_tx_gain_db: f64,
    /// The ports read back at open (chain 0's).
    rx_port0: String,
    tx_port0: String,
    /// A sentence for `IqSource::open_status`, or `None` when it came up clean.
    open_status: Option<String>,
}

impl RigInner {
    /// Stop the threads and close all three sockets, ahead of the engine
    /// building this front end's replacement. Idempotent.
    ///
    /// # Why the sockets are shut down and not just flagged
    ///
    /// This can run on the engine thread, and it joins three threads that
    /// spend their lives blocked in reads. A thread only notices `alive`
    /// between reads, so on a link that stalls, joining one could take
    /// seconds of frozen audio at exactly the moment the operator is trying
    /// to recover. Shutting the socket down makes the blocked read return
    /// immediately. On a healthy link this changes nothing.
    fn release(&self) {
        if self.released.swap(true, Ordering::Relaxed) {
            return;
        }
        self.shared.alive.store(false, Ordering::Relaxed);
        self.shared.rx_enabled.store(false, Ordering::Relaxed);
        self.shared.tx_enabled.store(false, Ordering::Relaxed);
        let _ = self.ctrl.send(Ctrl::Shutdown);
        for sock in &self.shutdowns {
            let _ = sock.shutdown(std::net::Shutdown::Both);
        }
        // After `alive` was cleared, so a receive thread racing to install a
        // replacement socket is refused rather than leaving one behind us.
        self.shared.drop_rx_socket();
        for j in self.joins.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
            let _ = j.join();
        }
        tracing::debug!("PlutoSDR: released {}", self.addr);
    }
}

impl Drop for RigInner {
    fn drop(&mut self) {
        self.release();
    }
}

/// A live connection to a Pluto, shared by every chain stream on it. Cheap to
/// clone; the sockets close when the last clone (and every [`PlutoRx`]) is
/// gone.
#[derive(Clone)]
pub struct PlutoRig {
    inner: Arc<RigInner>,
}

impl PlutoRig {
    /// Open `address` (`host[:port]`) and configure the front end from `cfg`,
    /// with the receive LO at `center_hz`. No streams yet — each chain's
    /// starts when [`PlutoRig::rx`] vends it.
    pub fn open(address: &str, cfg: &PlutoConfig, center_hz: f64) -> Result<PlutoRig> {
        let trace = Trace::new();
        crate::trace::remember(&trace);
        let result = PlutoRig::open_traced(address, cfg, center_hz, &trace);
        if let Err(e) = &result {
            // The trace outlives the failed attempt, and a session report that
            // stops mid-sequence without saying why has to be diagnosed by
            // matching line numbers against the source. Been there.
            trace.note(format!("!! open failed: {e}"));
        }
        result
    }

    fn open_traced(
        address: &str,
        cfg: &PlutoConfig,
        center_hz: f64,
        trace: &Trace,
    ) -> Result<PlutoRig> {
        let addr = resolve(address)?;
        trace.note(format!("opening {addr} (from {address:?})"));
        tracing::info!(
            "PlutoSDR: opening {addr}, requested {:.3} Msps at {:.6} MHz",
            cfg.sample_rate_hz / 1e6,
            center_hz / 1e6
        );

        let mut control = Connection::connect(addr, CONNECT_TIMEOUT, trace.clone())?;
        let version = control.version()?;
        let xml = control.print_xml()?;
        let ctx = Context::parse(&xml)?;
        trace.set_context(ctx.summary());
        tracing::info!(
            "PlutoSDR: {addr} is \"{}\" firmware {} (iiod {version})",
            ctx.hw_model(),
            ctx.fw_version()
        );
        let phy = Phy::probe(&mut control, &ctx, &addr.to_string())?;

        let mut warnings: Vec<String> = Vec::new();

        // Before anything else is configured, and not for tidiness: committing
        // the duplex and the GPO pins re-runs the AD9361's setup from its
        // device-tree copy, which puts the rate, the filter, the gains and both
        // oscillators back to the board's defaults. Everything below has to
        // land on the far side of that.
        let tr = phy.setup_tr_switching(&mut control, cfg.duplex, cfg.ptt_gpo)?;
        warnings.extend(tr.warnings);
        // TDD is one direction at a time in the silicon, so the link is no
        // longer what decides this — the part is.
        let full_duplex = if tr.tdd && cfg.full_duplex {
            let msg = "PlutoSDR: full duplex is off — TDD enables one direction at a time, \
                       and that is what the GPO transmit-receive pins key from. Set the PTT \
                       pins to Off and the duplex to FDD to listen through an over"
                .to_string();
            tracing::warn!("{msg}");
            warnings.push(msg);
            false
        } else {
            cfg.full_duplex
        };

        // Order matters. The rate sets the clock tree, the bandwidth is chosen
        // against the rate, and the receive gain only takes effect once the AGC
        // is in manual — so each step depends on the one before it.
        let rate = phy.set_sample_rate(&mut control, cfg.sample_rate_hz)?;
        let want_bw = if cfg.rf_bandwidth_hz > 0.0 {
            cfg.rf_bandwidth_hz
        } else {
            // Wide enough that the engine's quarter-span LO offset still clears
            // the analog filter, which is what keeps the offset from being
            // quietly abandoned. See `sdroxide_radio::lo_offset_for`.
            rate * 0.9
        };
        let bandwidth = phy.set_bandwidth(&mut control, want_bw)?;
        phy.set_agc_mode(&mut control, 0, cfg.agc.iio_name())?;
        phy.set_rx_gain(&mut control, 0, cfg.agc.iio_name(), cfg.rx_gain_db)?;
        // The second chain, on a firmware that has control registers for one.
        //
        // Not housekeeping: the AD9361 keeps a gain-control mode *per chain*,
        // and chain 1's boots wherever the driver's device tree left it —
        // an attack mode on the boards this was reported from. The control
        // thread tracks a mode per chain and seeds both from this config, so
        // unless the hardware is actually put there, the second receiver's
        // first gain write goes out believing it is in manual and the driver
        // answers `-EOPNOTSUPP`: the slider moves and nothing happens
        // (issue #311).
        //
        // Refusals are not fatal here the way chain 0's are. A board that
        // streams two chains but will not take these has a working first
        // receiver, and saying so beats declining to open at all.
        if phy.rx2_control {
            for step in [
                phy.set_agc_mode(&mut control, 1, cfg.agc.iio_name()),
                phy.set_rx_gain(&mut control, 1, cfg.agc.iio_name(), cfg.rx_gain_db),
            ] {
                if let Err(e) = step {
                    tracing::warn!("PlutoSDR: configuring the second receive chain: {e}");
                }
            }
        }
        if !cfg.rx_port.trim().is_empty() {
            phy.set_rx_port(&mut control, 0, cfg.rx_port.trim())?;
        }
        if !cfg.tx_port.trim().is_empty() {
            phy.set_tx_port(&mut control, cfg.tx_port.trim())?;
        }
        // Silence the transmitter first, then set the operator's level.
        //
        // The order is the point. The AD9361 keeps its attenuator setting
        // across a host disconnect, so whatever the last program to touch this
        // Pluto left behind is live from the moment we connect; writing the
        // minimum first means the register is never something nobody in this
        // session chose. The SoapySDR backend does the same thing on open, for
        // the same reason.
        phy.silence_transmitter(&mut control)?;
        phy.set_tx_gain(&mut control, cfg.tx_gain_db)?;
        // The on-chip tone generators, silenced here rather than only at the
        // first key-up. In FDD the transmit chain is live from this moment, so
        // "the DDS is zeroed before anything can go out" has to mean *open*,
        // not *key-up*; and taking it off the key-up path is part of what makes
        // an over start promptly (issue #135). Best-effort: a firmware that
        // publishes no tone channels has already said so in `silence_dds`, and
        // a rejected write there must not cost the operator their receiver.
        if let Err(e) = phy.silence_dds(&mut control) {
            tracing::warn!("PlutoSDR: could not silence the transmit tone generators: {e}");
        }
        phy.set_rx_lo(&mut control, PlutoConfig::apply_ppm(center_hz, cfg.ppm))?;
        // And park the transmit synthesiser in the band the session opened on.
        // Nothing radiates — in TDD the state machine is in receive and in FDD
        // the attenuator is at its minimum with the tones zeroed — and it means
        // the first key-up moves the synthesiser by kilohertz rather than
        // across the whole tuning range from wherever the part came up.
        let park = PlutoConfig::apply_ppm(center_hz, cfg.ppm);
        let tx_lo = match phy.set_tx_lo(&mut control, park) {
            // The pair is (asked for, achieved): the part quantises to its RF
            // PLL step, so the second is not always the first, and `key_up`
            // needs both to tell "nobody has moved it" from "it never went
            // where we asked".
            Ok(()) => phy.tx_lo(&mut control).ok().map(|at| (park, at)),
            Err(e) => {
                tracing::debug!("PlutoSDR: transmit oscillator not parked at open: {e}");
                None
            }
        };
        let tx_rate = phy.tx_sample_rate(&mut control).unwrap_or(rate);
        let rx_port = phy.rx_port(&mut control).unwrap_or_default();
        let tx_port = phy.tx_port(&mut control).unwrap_or_default();

        if let Some(n) = phy.limits.assumption_notice() {
            warnings.push(n);
        }
        if full_duplex {
            // Both a note for the log — full duplex is the setting a chopped
            // transmission gets blamed on, so the session has to say whether it
            // was on — and the one check that can be made from here. The link
            // itself cannot be measured before anything has streamed; the
            // throughput warnings in `stream::Stats` cover that once it has.
            match phy.ensm_mode(&mut control) {
                Ok(mode) if mode.eq_ignore_ascii_case("fdd") => {
                    tracing::info!("PlutoSDR: full duplex — receive stays up through an over");
                }
                Ok(mode) => {
                    let msg = format!(
                        "PlutoSDR: full duplex is on, but this board's enable state machine is \
                         in {mode}, not FDD — it can only receive or transmit at one time, so \
                         receive will still stop for the length of an over"
                    );
                    tracing::warn!("{msg}");
                    warnings.push(msg);
                }
                // Not fatal, and not worth a warning on screen: a firmware
                // that does not publish `ensm_mode` is one this check cannot
                // be made on, not one that is known to be wrong.
                Err(e) => tracing::debug!("PlutoSDR: could not read ensm_mode: {e}"),
            }
        }
        // A dial left outside this board's range — a restored session from a
        // different radio is the usual way — is clamped by `set_rx_lo`, and
        // silently receiving 20 MHz from where the operator is looking is
        // indistinguishable from a broken receiver. Say it instead.
        let (lo_min, lo_max) = phy.limits.rx_lo_hz;
        if center_hz < lo_min || center_hz > lo_max {
            let msg = format!(
                "PlutoSDR: {:.6} MHz is outside this board's {:.3}–{:.3} MHz tuning range, so \
                 it is receiving at {:.6} MHz instead — retune inside the range",
                center_hz / 1e6,
                lo_min / 1e6,
                lo_max / 1e6,
                center_hz.clamp(lo_min, lo_max) / 1e6,
            );
            tracing::warn!("{msg}");
            warnings.push(msg);
        }
        if (rate - cfg.sample_rate_hz).abs() > 1.0 {
            let msg = format!(
                "PlutoSDR: {:.3} Msps requested, {:.3} Msps is what the hardware produced",
                cfg.sample_rate_hz / 1e6,
                rate / 1e6
            );
            tracing::info!("{msg}");
            warnings.push(msg);
        }
        tracing::info!(
            "PlutoSDR: {:.3} Msps, analog filter {:.3} MHz, AGC {}, RX gain {:.1} dB, \
             ports RX {rx_port} / TX {tx_port}",
            rate / 1e6,
            bandwidth / 1e6,
            cfg.agc.iio_name(),
            cfg.rx_gain_db,
        );

        // In TDD nothing receives until the state machine is told to. Last,
        // because every write above it moves the part through its own
        // transitions — and because the transmitter has been silenced by now,
        // so the first state this radio is deliberately put into is receive.
        if tr.tdd {
            phy.set_ensm_mode(&mut control, ENSM_RX).map_err(|e| {
                Error::Msg(format!(
                    "this board is in TDD — which is what the GPO transmit-receive pins key \
                     from — but it would not take ensm_mode = {ENSM_RX} ({e}), so its \
                     receiver would never start. Set the PTT pins to Off in the \
                     PlutoSDR settings to run it in FDD"
                ))
            })?;
        }

        if phy.rx_pairs_available() > 1 {
            tracing::info!(
                "PlutoSDR: this firmware streams {} receive chains — a second radio on the \
                 same address can take the other one (they share the one LO)",
                phy.rx_pairs_available()
            );
        }
        let buffer_samples = cfg.buffer_samples.clamp(1024, 1 << 20);
        let shared = Arc::new(Shared {
            phy: phy.clone(),
            addr,
            rx_shutdown: Mutex::new(None),
            rx_enabled: AtomicBool::new(true),
            rx_paused: AtomicBool::new(false),
            rx_active: AtomicBool::new(false),
            tx_enabled: AtomicBool::new(false),
            tx_active: AtomicBool::new(false),
            alive: AtomicBool::new(true),
            opened_at: Instant::now(),
            last_rx_ms: AtomicU64::new(0),
            last_rx1_ms: AtomicU64::new(0),
            transmitting: AtomicBool::new(false),
            full_duplex,
            tdd: tr.tdd,
            buffer_samples,
            rate_hz: rate,
            tx_rate_hz: tx_rate,
            tx_buffer_samples: tx_buffer_samples(tx_rate),
            rx_pairs: AtomicUsize::new(1),
            ring1: Mutex::new(None),
            lo_watch: Mutex::new(Vec::new()),
            trace: trace.clone(),
        });

        // RX ring ~0.5 s at the RX rate; TX ring ~0.5 s at the TX rate.
        let rx_cap = ((rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
        let (rx_prod, rx_cons) = RingBuffer::<f32>::new(rx_cap);
        let tx_cap = ((tx_rate * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 15);
        let (tx_prod, tx_cons) = RingBuffer::<f32>::new(tx_cap);
        tracing::debug!(
            "PlutoSDR: RX ring {rx_cap} floats, TX ring {tx_cap} floats, \
             {buffer_samples}-sample RX device buffers, {}-sample TX device buffers ({:.1} ms)",
            shared.tx_buffer_samples,
            shared.tx_buffer_samples as f64 / tx_rate * 1e3,
        );

        // The data connections are opened after the control one has proved the
        // device is a Pluto, so a wrong address costs one connection, not three.
        let rx_conn = Connection::connect(addr, CONNECT_TIMEOUT, trace.clone()).map_err(|e| {
            Error::Msg(format!(
                "the receive connection to {addr} was refused ({e}) — `iiod` accepted the \
                 first connection, so this is a per-connection limit rather than a wrong \
                 address"
            ))
        })?;
        let tx_conn = Connection::connect(addr, CONNECT_TIMEOUT, trace.clone()).map_err(|e| {
            Error::Msg(format!("the transmit connection to {addr} was refused ({e})"))
        })?;

        // Taken before the connections are handed to their threads: after that
        // the only way to reach a socket is through the thread that is blocked
        // on it, which is precisely the situation these exist to break. The
        // receive socket is not among them — it is the one that gets replaced
        // under us, so it lives in `Shared::rx_shutdown` where the receive
        // thread can keep it current.
        let shutdowns: Vec<std::net::TcpStream> =
            [&control, &tx_conn].iter().filter_map(|c| c.shutdown_handle()).collect();
        shared.adopt_rx_socket(&rx_conn)?;

        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let rx_shared = Arc::clone(&shared);
        let tx_shared = Arc::clone(&shared);
        let ctl_shared = Arc::clone(&shared);
        let ctl_cfg = cfg.clone();
        let joins = vec![
            spawn("sdroxide-pluto-rx", move || stream::rx_thread(rx_conn, rx_shared, rx_prod))?,
            spawn("sdroxide-pluto-tx", move || stream::tx_thread(tx_conn, tx_shared, tx_cons))?,
            spawn("sdroxide-pluto-ctl", move || {
                control_thread(control, ctl_shared, ctrl_rx, ctl_cfg, center_hz, tx_lo)
            })?,
        ];

        Ok(PlutoRig {
            inner: Arc::new(RigInner {
                ctrl: ctrl_tx,
                shared,
                joins: Mutex::new(joins),
                shutdowns,
                released: AtomicBool::new(false),
                rx0: Mutex::new(Some(rx_cons)),
                tx0: Mutex::new(Some(tx_prod)),
                attached: Mutex::new(std::collections::HashSet::new()),
                sample_rate_hz: rate,
                tx_rate_hz: tx_rate,
                rf_bandwidth_hz: bandwidth,
                limits: phy.limits.clone(),
                model: phy.model.clone(),
                firmware: phy.firmware.clone(),
                serial: phy.serial.clone(),
                addr,
                init_rx_gain_db: cfg.rx_gain_db,
                init_tx_gain_db: cfg
                    .tx_gain_db
                    .clamp(phy.limits.tx_gain_db.0, phy.limits.tx_gain_db.1),
                rx_port0: rx_port,
                tx_port0: tx_port,
                open_status: (!warnings.is_empty()).then(|| warnings.join("; ")),
            }),
        })
    }

    /// One line naming the radio, for logs and the UI.
    pub fn label(&self) -> String {
        let model =
            if self.inner.model.is_empty() { "PlutoSDR" } else { self.inner.model.as_str() };
        format!("{model} @ {} ({:.3} Msps)", self.inner.addr.ip(), self.sample_rate_hz() / 1e6)
    }

    /// A warning captured while opening, or `None` when it came up clean.
    pub fn open_status(&self) -> Option<String> {
        self.inner.open_status.clone()
    }

    pub fn trace(&self) -> &Trace {
        &self.inner.shared.trace
    }

    pub fn is_alive(&self) -> bool {
        self.inner.shared.alive.load(Ordering::Relaxed)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.inner.sample_rate_hz
    }

    pub fn tx_rate_hz(&self) -> f64 {
        self.inner.tx_rate_hz
    }

    pub fn rf_bandwidth_hz(&self) -> f64 {
        self.inner.rf_bandwidth_hz
    }

    pub fn limits(&self) -> &PlutoLimits {
        &self.inner.limits
    }

    pub fn model(&self) -> &str {
        &self.inner.model
    }

    pub fn firmware(&self) -> &str {
        &self.inner.firmware
    }

    pub fn serial(&self) -> &str {
        &self.inner.serial
    }

    pub fn addr(&self) -> SocketAddr {
        self.inner.addr
    }

    /// How many receive chains this firmware streams (1, or 2 on a 2R2T build
    /// such as a Pluto+).
    pub fn rx_chains(&self) -> u8 {
        self.inner.shared.phy.rx_pairs_available() as u8
    }

    /// Whether receive runs through an over on this connection — the
    /// operator's answer about their link, not a property of the board.
    pub fn full_duplex(&self) -> bool {
        self.inner.shared.full_duplex
    }

    /// Stop the threads and close the sockets. What dropping the last handle
    /// does anyway; public for the single-stream wrapper's teardown semantic.
    pub fn release(&self) {
        self.inner.release();
    }

    /// Attach receive chain `chain` and start its stream. Refused for a chain
    /// this firmware does not stream — a stock Pluto has one — and for one
    /// that already has a live stream: two engines draining one ring would
    /// each get half the samples.
    pub fn rx(&self, chain: u8) -> Result<PlutoRx> {
        let inner = &self.inner;
        if !self.is_alive() {
            return Err(Error::Msg("this connection is closed".into()));
        }
        let chains = self.rx_chains();
        if chain >= chains {
            return Err(Error::Unsupported(format!(
                "receive chain {} does not exist: this {} firmware streams {} chain(s){}",
                chain + 1,
                if inner.model.is_empty() { "Pluto" } else { inner.model.as_str() },
                chains,
                if chains == 1 {
                    " — a 2R2T build (a Pluto+, or firmware with both chains enabled) is \
                     needed for a second"
                } else {
                    ""
                }
            )));
        }
        {
            let mut attached = inner.attached.lock().unwrap_or_else(|e| e.into_inner());
            if !attached.insert(chain) {
                return Err(Error::Msg(format!(
                    "receive chain {} is already running as another radio",
                    chain + 1
                )));
            }
        }
        let (ring, tx) = if chain == 0 {
            let ring = inner.rx0.lock().unwrap_or_else(|e| e.into_inner()).take();
            let tx = inner.tx0.lock().unwrap_or_else(|e| e.into_inner()).take();
            match ring {
                Some(mut r) => {
                    // The receive thread kept filling this ring while nobody
                    // held it (chain 0's producer lives in the thread); drain
                    // the backlog so a re-attached stream starts live rather
                    // than replaying half a second of the past.
                    while r.pop().is_ok() {}
                    (r, tx)
                }
                None => {
                    inner.attached.lock().unwrap_or_else(|e| e.into_inner()).remove(&chain);
                    return Err(Error::Msg("chain 0's stream endpoint is gone".into()));
                }
            }
        } else {
            let cap =
                ((inner.sample_rate_hz * 2.0 * 0.5) as usize).next_power_of_two().max(1 << 16);
            let (prod, cons) = RingBuffer::<f32>::new(cap);
            // The liveness clock starts at "data seen now": a stream attached
            // to a long-running connection must age from its attach, not from
            // the connection's epoch.
            inner.shared.stamp_rx1();
            *inner.shared.ring1.lock().unwrap_or_else(|e| e.into_inner()) = Some(prod);
            // The receive thread reopens its buffer for both pairs.
            inner.shared.rx_pairs.store(2, Ordering::Relaxed);
            (cons, None)
        };
        let (lo_tx, lo_rx) = crossbeam_channel::unbounded();
        inner.shared.lo_watch.lock().unwrap_or_else(|e| e.into_inner()).push((chain, lo_tx));
        Ok(PlutoRx {
            rig: Arc::clone(inner),
            chain,
            ring: Some(ring),
            lo_moves: lo_rx,
            tx,
            rx_gain_db: inner.init_rx_gain_db,
            tx_gain_db: inner.init_tx_gain_db,
            rx_port: if chain == 0 { inner.rx_port0.clone() } else { String::new() },
            tx_port: inner.tx_port0.clone(),
        })
    }
}

/// One receive chain's stream on a shared [`PlutoRig`]: its IQ ring, its gain
/// and port, and — on chain 0 — the transmitter. The LO is the rig's, shared
/// by every chain: tuning here moves the siblings too, and their moves arrive
/// through [`PlutoRx::poll_lo_moves`]. Dropping the stream detaches it; the
/// connection lives on for whoever else holds it.
pub struct PlutoRx {
    rig: Arc<RigInner>,
    chain: u8,
    ring: Option<Consumer<f32>>,
    /// LO moves a *sibling* stream commanded, in engine-domain hertz.
    lo_moves: Receiver<f64>,
    /// TX feed — chain 0 only.
    tx: Option<Producer<f32>>,
    rx_gain_db: f64,
    tx_gain_db: f64,
    rx_port: String,
    tx_port: String,
}

impl std::fmt::Debug for PlutoRx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlutoRx")
            .field("chain", &self.chain)
            .field("addr", &self.rig.addr)
            .finish_non_exhaustive()
    }
}

impl Drop for PlutoRx {
    fn drop(&mut self) {
        self.rig.attached.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.chain);
        self.rig
            .shared
            .lo_watch
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|(c, _)| *c != self.chain);
        if self.chain == 0 {
            // Hand the endpoints back so a rebuilt chain-0 stream (Settings →
            // Apply) can claim them again on the same connection.
            if let Some(r) = self.ring.take() {
                *self.rig.rx0.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
            }
            if let Some(t) = self.tx.take() {
                *self.rig.tx0.lock().unwrap_or_else(|e| e.into_inner()) = Some(t);
            }
        } else {
            // The receive thread narrows its buffer back to one pair.
            self.rig.shared.rx_pairs.store(1, Ordering::Relaxed);
            *self.rig.shared.ring1.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

impl PlutoRx {
    /// Which receive chain this stream is (0-based).
    pub fn chain(&self) -> u8 {
        self.chain
    }

    pub fn is_alive(&self) -> bool {
        self.rig.shared.alive.load(Ordering::Relaxed)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.rig.sample_rate_hz
    }

    pub fn rf_bandwidth_hz(&self) -> f64 {
        self.rig.rf_bandwidth_hz
    }

    /// Whether this stream keeps running through an over — see
    /// [`PlutoRig::full_duplex`].
    pub fn full_duplex(&self) -> bool {
        self.rig.shared.full_duplex
    }

    pub fn limits(&self) -> &PlutoLimits {
        &self.rig.limits
    }

    /// Retune the receive local oscillator — the one LO every chain shares.
    pub fn set_rx_freq(&self, hz: f64) {
        let _ = self.rig.ctrl.send(Ctrl::RxFreq { hz, origin: self.chain });
    }

    /// LO moves a sibling stream commanded since the last poll, newest last.
    /// The engine adopts these as centre changes; it must not answer them.
    pub fn poll_lo_moves(&self) -> Vec<f64> {
        self.lo_moves.try_iter().collect()
    }

    pub fn rx_gain_db(&self) -> f64 {
        self.rx_gain_db
    }

    pub fn set_rx_gain_db(&mut self, db: f64) {
        let db = db.clamp(self.rig.limits.rx_gain_db.0, self.rig.limits.rx_gain_db.1);
        self.rx_gain_db = db;
        let _ = self.rig.ctrl.send(Ctrl::RxGain { chain: self.chain, db });
    }

    pub fn tx_gain_db(&self) -> f64 {
        self.tx_gain_db
    }

    /// Transmit gain in dB — negative, because the AD9361 expresses it as
    /// attenuation. Chain 0 owns the transmitter; a no-op elsewhere.
    pub fn set_tx_gain_db(&mut self, db: f64) {
        if self.tx.is_none() {
            return;
        }
        let db = db.clamp(self.rig.limits.tx_gain_db.0, self.rig.limits.tx_gain_db.1);
        self.tx_gain_db = db;
        let _ = self.rig.ctrl.send(Ctrl::TxGain(db));
    }

    /// Switch this chain's receive AGC mode (`manual`, `slow_attack`,
    /// `fast_attack`, `hybrid`).
    pub fn set_agc_mode(&self, mode: &str) {
        let _ = self.rig.ctrl.send(Ctrl::AgcMode { chain: self.chain, mode: mode.to_string() });
    }

    /// Reference trim in parts per million. One crystal serves the whole
    /// device, so this belongs to chain 0's radio; a no-op elsewhere.
    pub fn set_ppm(&self, ppm: f64) {
        if self.chain == 0 {
            let _ = self.rig.ctrl.send(Ctrl::Ppm(ppm));
        }
    }

    pub fn rx_port(&self) -> &str {
        &self.rx_port
    }

    pub fn tx_port(&self) -> &str {
        &self.tx_port
    }

    /// Select the receive port, if it is not the one already selected.
    ///
    /// The no-op guard is not an optimisation. `rf_port_select` is refused
    /// (`-EINVAL`) while the receive buffer is running, and the engine re-asserts
    /// the antenna on every retune — so a radio with exactly one wired port
    /// logged a rejected write each time the operator touched the dial, for a
    /// change that was never a change.
    pub fn set_rx_port(&mut self, port: &str) {
        if self.rx_port == port {
            return;
        }
        self.rx_port = port.to_string();
        let _ = self.rig.ctrl.send(Ctrl::RxPort { chain: self.chain, port: port.to_string() });
    }

    /// Select the transmit port, if it is not the one already selected. Same
    /// reasoning as [`Self::set_rx_port`]; chain 0 owns the transmitter.
    pub fn set_tx_port(&mut self, port: &str) {
        if self.tx.is_none() || self.tx_port == port {
            return;
        }
        self.tx_port = port.to_string();
        let _ = self.rig.ctrl.send(Ctrl::TxPort(port.to_string()));
    }

    /// Begin transmitting at `tx_freq_hz`; returns the TX I/Q rate to feed
    /// [`Self::tx_write`]. The transmitter is chain 0's — a no-op on any
    /// other stream, which the engine never asks anyway (their capabilities
    /// carry no TX).
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        if self.tx.is_some() {
            tracing::info!(
                "PlutoSDR: TX begin at {tx_freq_hz:.0} Hz ({:.3} Msps I/Q, {:.2} dB)",
                self.rig.tx_rate_hz / 1e6,
                self.tx_gain_db
            );
            self.rig.shared.transmitting.store(true, Ordering::Relaxed);
            let _ = self.rig.ctrl.send(Ctrl::TxOn(tx_freq_hz));
        }
        self.rig.tx_rate_hz
    }

    pub fn tx_end(&self) {
        if self.tx.is_some() {
            tracing::info!("PlutoSDR: TX end");
            self.rig.shared.transmitting.store(false, Ordering::Relaxed);
            let _ = self.rig.ctrl.send(Ctrl::TxOff);
        }
    }

    /// Push interleaved I,Q transmit samples. Blocks briefly when the ring is
    /// full (pacing the caller); drops rather than hanging if the transmit
    /// thread has stalled, and drops silently on a stream with no transmitter.
    ///
    /// Writes go in whole I/Q pairs. Giving up mid-pair would put every later
    /// sample one slot out of step, so each Q would be encoded as an I — the
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
                            return; // ~200 ms: the thread has stalled
                        }
                        tries += 1;
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
            };
            let (head, tail) = chunk.as_mut_slices();
            for (slot, &v) in head.iter_mut().chain(tail.iter_mut()).zip(pair) {
                *slot = v;
            }
            chunk.commit_all();
        }
    }

    /// How many transmit floats are still queued, so PTT can be held until the
    /// tail has actually gone out (an FT8 burst needs every symbol).
    pub fn tx_pending(&self) -> usize {
        self.tx.as_ref().map_or(0, |tx| tx.buffer().capacity().saturating_sub(tx.slots()))
    }

    /// Drain interleaved I,Q floats from the RX ring into `out`. Always returns
    /// an even count, so the stream stays aligned. 0 means nothing yet.
    pub fn rx_read(&mut self, out: &mut [f32]) -> usize {
        let Some(ring) = self.ring.as_mut() else { return 0 };
        let take = ring.slots().min(out.len()) & !1;
        let mut n = 0;
        while n < take {
            match ring.pop() {
                Ok(v) => {
                    out[n] = v;
                    n += 1;
                }
                Err(_) => break,
            }
        }
        n
    }

    /// Drop whatever the receive thread queued. Receive is torn down for the
    /// length of an over, but a partial buffer can still be sitting in the ring
    /// when it resumes, and replaying it would put a burst of stale audio in
    /// front of the first live sample.
    pub fn discard_pending_rx(&mut self) {
        if let Some(ring) = self.ring.as_mut() {
            while ring.pop().is_ok() {}
        }
    }

    /// Tell the receive thread that the engine has stopped reading for an over,
    /// and then that it has started again — see `IqSource::set_rx_paused`. Only
    /// reached when this Pluto is somebody else's panadapter: keying its own
    /// transmitter closes the receive buffer, so there is nothing to account
    /// for.
    pub fn set_rx_paused(&self, paused: bool) {
        self.rig.shared.rx_paused.store(paused, Ordering::Relaxed);
    }

    /// How long this stream has gone without samples, measured from the last
    /// buffer decoded for its chain or — if none arrived yet — from when it
    /// attached. A stream that never starts is the failure that matters most
    /// here, so it ages just like one that stops. Always zero while
    /// transmitting, when receive is deliberately switched off.
    pub fn silent_for(&self) -> Duration {
        let shared = &self.rig.shared;
        if shared.transmitting.load(Ordering::Relaxed) {
            return Duration::ZERO;
        }
        let clock = if self.chain == 0 { &shared.last_rx_ms } else { &shared.last_rx1_ms };
        let since_open = shared.opened_at.elapsed();
        let last = Duration::from_millis(clock.load(Ordering::Relaxed));
        since_open.saturating_sub(last)
    }
}

/// The single-stream view of a Pluto: chain 0 of a [`PlutoRig`] of its own.
/// What every caller used before chains were split from the connection, kept
/// for them — the loopback tests and the probe example drive exactly one
/// chain. `release()` keeps its old meaning here: the whole connection goes
/// down, not just the stream.
pub struct PlutoHandle {
    rig: PlutoRig,
    rx0: PlutoRx,

    /// Actual RX sample rate in Hz, after the hardware rounded the request.
    pub sample_rate_hz: f64,
    /// Actual TX sample rate. The AD9361 clocks both paths together, so this is
    /// normally the same number — read back rather than assumed.
    pub tx_rate_hz: f64,
    /// Analog filter bandwidth actually set, in Hz. The engine's LO-offset
    /// policy is decided against this.
    pub rf_bandwidth_hz: f64,
    /// What the device says it can do.
    pub limits: PlutoLimits,
    pub model: String,
    pub firmware: String,
    pub serial: String,
    /// Where it was reached, for labels and errors.
    pub addr: SocketAddr,
}

impl PlutoHandle {
    /// Open `address` (`host[:port]`), configure the front end from `cfg`, and
    /// start receiving at `center_hz`.
    pub fn open(address: &str, cfg: &PlutoConfig, center_hz: f64) -> Result<PlutoHandle> {
        let rig = PlutoRig::open(address, cfg, center_hz)?;
        let rx0 = rig.rx(0)?;
        Ok(PlutoHandle {
            sample_rate_hz: rig.sample_rate_hz(),
            tx_rate_hz: rig.tx_rate_hz(),
            rf_bandwidth_hz: rig.rf_bandwidth_hz(),
            limits: rig.limits().clone(),
            model: rig.model().to_string(),
            firmware: rig.firmware().to_string(),
            serial: rig.serial().to_string(),
            addr: rig.addr(),
            rig,
            rx0,
        })
    }

    pub fn label(&self) -> String {
        self.rig.label()
    }
    pub fn open_status(&self) -> Option<String> {
        self.rig.open_status()
    }
    pub fn trace(&self) -> &Trace {
        self.rig.trace()
    }
    pub fn is_alive(&self) -> bool {
        self.rig.is_alive()
    }
    pub fn set_rx_freq(&self, hz: f64) {
        self.rx0.set_rx_freq(hz);
    }
    pub fn rx_gain_db(&self) -> f64 {
        self.rx0.rx_gain_db()
    }
    pub fn set_rx_gain_db(&mut self, db: f64) {
        self.rx0.set_rx_gain_db(db);
    }
    pub fn tx_gain_db(&self) -> f64 {
        self.rx0.tx_gain_db()
    }
    pub fn set_tx_gain_db(&mut self, db: f64) {
        self.rx0.set_tx_gain_db(db);
    }
    pub fn set_agc_mode(&self, mode: &str) {
        self.rx0.set_agc_mode(mode);
    }
    pub fn set_ppm(&self, ppm: f64) {
        self.rx0.set_ppm(ppm);
    }
    pub fn rx_port(&self) -> &str {
        self.rx0.rx_port()
    }
    pub fn tx_port(&self) -> &str {
        self.rx0.tx_port()
    }
    pub fn set_rx_port(&mut self, port: &str) {
        self.rx0.set_rx_port(port);
    }
    pub fn set_tx_port(&mut self, port: &str) {
        self.rx0.set_tx_port(port);
    }
    pub fn tx_begin(&self, tx_freq_hz: f64) -> f64 {
        self.rx0.tx_begin(tx_freq_hz)
    }
    pub fn tx_end(&self) {
        self.rx0.tx_end();
    }
    pub fn tx_write(&mut self, iq: &[f32]) {
        self.rx0.tx_write(iq);
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
    pub fn set_rx_paused(&self, paused: bool) {
        self.rx0.set_rx_paused(paused);
    }
    pub fn silent_for(&self) -> Duration {
        self.rx0.silent_for()
    }

    /// Stop the threads and close all three sockets, ahead of the engine
    /// building this front end's replacement. Idempotent, and leaves the
    /// handle callable: `rx_read` returns nothing and `is_alive` returns
    /// false, which is what the reopen path expects.
    pub fn release(&mut self) {
        self.rig.release();
    }
}

fn spawn<F: FnOnce() + Send + 'static>(name: &str, body: F) -> Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(name.into())
        .spawn(body)
        .map_err(|e| Error::Msg(format!("cannot spawn {name}: {e}")))
}

/// Resolve `host[:port]` to one socket address, preferring IPv4 — a Pluto's USB
/// gadget only has an IPv4 address, and `pluto.local` on a host with IPv6
/// otherwise resolves to something that cannot be reached.
pub(crate) fn resolve(address: &str) -> Result<SocketAddr> {
    let (host, port) = crate::split_addr(address).map_err(Error::Msg)?;
    let mut addrs = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| Error::Unreachable(format!("cannot resolve {host:?}: {e}")))?
        .collect::<Vec<_>>();
    addrs.sort_by_key(|a| !a.is_ipv4());
    addrs
        .into_iter()
        .next()
        .ok_or_else(|| Error::Unreachable(format!("{host:?} resolved to no addresses")))
}

/// Owns the control connection and the AD9361.
///
/// Everything that touches a front-end register happens here, on its own
/// socket, so nothing the operator does — a retune mid-drag, a gain slider —
/// waits behind a buffer read.
fn control_thread(
    mut conn: Connection,
    shared: Arc<Shared>,
    ctrl: Receiver<Ctrl>,
    cfg: PlutoConfig,
    center_hz: f64,
    tx_lo: Option<(f64, f64)>,
) {
    let phy = &shared.phy;
    let mut ppm = cfg.ppm;
    // Per receive chain, because a 2R2T firmware has a register set for each.
    // Chain 1's values start where chain 0's config put things; its own radio
    // asserts what it actually wants the moment it attaches.
    let mut rx_gain_db = [cfg.rx_gain_db; 2];
    // Tracked because the receive gain register is only writable in manual —
    // see `Phy::set_rx_gain`. Seeded from what `open` actually set.
    let mut agc_mode = [cfg.agc.iio_name().to_string(), cfg.agc.iio_name().to_string()];
    // Seeded from where `open` left the oscillator, so a ppm trim made before
    // the operator has touched the dial still moves it. This is the *shared*
    // LO — the AD9361's chains have one — in engine-domain hertz.
    let mut rx_hz = center_hz;
    // What the transmit synthesiser was last asked for and what it achieved,
    // in the same ppm-corrected hertz `key_up` asks for — seeded from where
    // `open` parked it, and `None` if it would not park. See [`key_up`] for why
    // an over that does not have to move it is so much quicker off the mark.
    let mut tx_lo = tx_lo;
    while shared.alive.load(Ordering::Relaxed) {
        let msg = match ctrl.recv_timeout(Duration::from_millis(200)) {
            Ok(m) => m,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        };
        let outcome = match msg {
            Ctrl::RxFreq { hz, origin } => {
                // An echo of where the LO already is moves nothing and tells
                // nobody — the dedup that keeps two engines sharing this LO
                // from chasing each other.
                if (hz - rx_hz).abs() < 0.5 {
                    Ok(())
                } else {
                    match phy.set_rx_lo(&mut conn, PlutoConfig::apply_ppm(hz, ppm)) {
                        Ok(()) => {
                            rx_hz = hz;
                            notify_lo_moved(&shared, hz, origin);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            Ctrl::RxGain { chain, db } => {
                // Remembered whatever the mode is, so a slider moved while an
                // attack mode is running still takes effect on the way back
                // into manual rather than being thrown away.
                rx_gain_db[chain as usize & 1] = db;
                phy.set_rx_gain(&mut conn, chain, &agc_mode[chain as usize & 1], db)
            }
            Ctrl::AgcMode { chain, mode } => {
                phy.set_agc_mode(&mut conn, chain, &mode).and_then(|()| {
                    let c = chain as usize & 1;
                    agc_mode[c] = mode;
                    // The gain register is the AD9361's while an attack mode
                    // runs, so the value the operator last chose has to be
                    // replayed on the way back into manual — otherwise the
                    // radio resumes at whatever level the AGC happened to
                    // leave behind.
                    phy.set_rx_gain(&mut conn, chain, &agc_mode[c], rx_gain_db[c])
                })
            }
            Ctrl::RxPort { chain, port } => phy.set_rx_port(&mut conn, chain, &port),
            Ctrl::TxPort(p) => phy.set_tx_port(&mut conn, &p),
            Ctrl::TxGain(db) => phy.set_tx_gain(&mut conn, db),
            Ctrl::Ppm(v) => {
                ppm = v;
                // Take effect now rather than at the next retune: an operator
                // trimming ppm is watching a carrier while they drag. The
                // engine-domain frequency is unchanged, so the siblings are
                // not told — their dials did not move.
                if rx_hz > 0.0 {
                    phy.set_rx_lo(&mut conn, PlutoConfig::apply_ppm(rx_hz, ppm))
                } else {
                    Ok(())
                }
            }
            Ctrl::TxOn(hz) => key_up(&mut conn, &shared, hz, ppm, &mut tx_lo),
            Ctrl::TxOff => key_down(&mut conn, &shared, rx_hz, ppm),
            Ctrl::Shutdown => break,
        };
        if let Err(e) = outcome {
            // A rejected attribute write is not fatal on its own — a value out
            // of range, a mode this board does not have — but a socket that has
            // gone is. Distinguishing them is what keeps a bad slider from
            // tearing down a working radio.
            match e {
                Error::Remote { .. } | Error::Unsupported(_) => {
                    tracing::warn!("PlutoSDR: {e}");
                    shared.trace.note(format!("!! {e}"));
                }
                _ => {
                    shared.die("the control connection", &e);
                    break;
                }
            }
        }
    }
    shared.rx_enabled.store(false, Ordering::Relaxed);
    shared.tx_enabled.store(false, Ordering::Relaxed);
    shared.alive.store(false, Ordering::Relaxed);
    // Last thing on the way out, and best-effort by design: on a TDD board the
    // transmit GPO is keying somebody's amplifier, and a session that ends
    // during an over would otherwise leave it keyed until the Pluto is
    // rebooted. The socket may already have been shut down to break a blocked
    // read, in which case this simply fails — which is why it is not the only
    // thing standing between an over and a stuck PA, just the cheap one.
    if shared.tdd
        && let Err(e) = shared.phy.set_ensm_mode(&mut conn, ENSM_RX)
    {
        tracing::debug!("PlutoSDR: could not put the state machine back to receive: {e}");
    }
    conn.exit();
    tracing::debug!("PlutoSDR: control thread finished");
}

/// Tell every attached stream but `origin` that the shared LO moved to `hz`
/// (engine-domain hertz). Their engines adopt the new centre; the origin's
/// already knows — it asked.
fn notify_lo_moved(shared: &Shared, hz: f64, origin: u8) {
    let watchers = shared.lo_watch.lock().unwrap_or_else(|e| e.into_inner());
    for (chain, tx) in watchers.iter() {
        if *chain != origin {
            let _ = tx.send(hz);
        }
    }
}

/// Tune the transmit LO if it has moved, take receive down unless the link can
/// carry both, throw the T/R switch, then hand the link to the transmit thread.
///
/// # The order is what the amplifier feels
///
/// In TDD the state-machine write is what asserts the slaved GPO pin — it *is*
/// the PTT line an external PA, LNA or transmit-receive switch follows — so
/// everything ahead of it is delay the operator sees as the relay lagging the
/// key. Issue #135 measured that at two to three seconds against an unkey that
/// was instant, and the two things in front of it are why:
///
/// * **The transmit oscillator.** Writing it makes the AD9361 force its state
///   machine to ALERT, retune the synthesiser and calibrate the transmit
///   quadrature before restoring the state — the better part of a second, and
///   it used to be paid on *every* key-up even when the dial had not moved
///   since the last one. So the frequency actually in force is remembered and
///   an over that does not need it moved skips the write, which is what
///   `LimeHandle::tx_begin` does with its own slow call next door.
/// * **Waiting for the receive buffer to close.** Half a second at worst, and
///   the chip does not need it: the state machine leaving receive is what
///   silences that direction. It is the *link* that cannot carry both, and only
///   the transmit buffer opening below cares about that — so the wait now sits
///   behind the T/R switch instead of in front of it.
///
/// The DDS step is the one that cannot be skipped: the transmit path is fed by
/// on-chip tone generators unless they are explicitly zeroed, and a Pluto that
/// skips it puts out a steady carrier pair at full power instead of the
/// modulation. `open` zeroes them too, so this is the cheap re-assertion and
/// not the only one.
fn key_up(
    conn: &mut Connection,
    shared: &Shared,
    hz: f64,
    ppm: f64,
    tx_lo: &mut Option<(f64, f64)>,
) -> Result<()> {
    let phy = &shared.phy;
    phy.silence_dds(conn)?;
    let want = PlutoConfig::apply_ppm(hz, ppm);
    // Skipped only when this over is on the frequency the last write asked for
    // *and* the synthesiser is still where that write actually left it. The
    // second half is a read — one round trip, against the better part of a
    // second for the write — and it is what makes this safe to skip at all:
    // nothing here has to be sure that unkeying, a ppm trim or a retune of the
    // other direction left the transmit oscillator alone, because it looks.
    let settled = match *tx_lo {
        Some((asked, at)) if (asked - want).abs() < 0.5 => {
            phy.tx_lo(conn).is_ok_and(|now| (now - at).abs() < 0.5)
        }
        _ => false,
    };
    if !settled {
        // Cleared first: a write that fails part way leaves the synthesiser
        // somewhere nobody knows, and the next over must not trust the old
        // answer.
        *tx_lo = None;
        phy.set_tx_lo(conn, want)?;
        *tx_lo = phy.tx_lo(conn).ok().map(|at| (want, at));
    }
    // Tell the receive thread to stop *before* the state machine moves, so it
    // is already unwinding while the relay settles; the wait for it is below.
    if !shared.full_duplex {
        shared.rx_enabled.store(false, Ordering::Relaxed);
    }
    // In TDD the transmit path is off until the state machine is moved, so
    // this comes before the buffer opens rather than after: a DMA buffer
    // filling a disabled transmitter is samples thrown away. It is also what
    // asserts the slaved GPO pin, so an external amplifier is keyed a few
    // milliseconds before there is anything for it to amplify — which is the
    // order a T/R switch wants.
    if shared.tdd {
        phy.set_ensm_mode(conn, ENSM_TX)?;
    }
    if !shared.full_duplex {
        // Now wait for the receive thread to actually let go of its buffer:
        // on a USB 2.0 gadget there is only room for one of them, and the
        // transmit buffer opens the moment the flag below is set. Bounded — if
        // the receive thread has died, transmit should still work.
        let deadline = Instant::now() + Duration::from_millis(500);
        while shared.rx_active.load(Ordering::Relaxed) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    shared.tx_enabled.store(true, Ordering::Relaxed);
    Ok(())
}

/// Give the link back to receive, and put the receive LO back where the dial is
/// — the transmit LO may have moved it if the two share a synthesiser.
///
/// Both halves are skipped in full duplex, and for the same reason: receive
/// never let go. The LO in particular must be left alone — nothing moved it (a
/// part that transmits and receives at once has a synthesiser for each, which
/// is the premise of running this way at all), and writing it mid-stream would
/// put a gap in a receiver that is working.
fn key_down(conn: &mut Connection, shared: &Shared, rx_hz: f64, ppm: f64) -> Result<()> {
    shared.tx_enabled.store(false, Ordering::Relaxed);
    if shared.full_duplex {
        return Ok(());
    }
    // Wait for the transmit buffer to actually close before receive reclaims
    // the link: on a USB 2.0 gadget there is only room for one of them.
    let deadline = Instant::now() + Duration::from_millis(500);
    while shared.tx_active.load(Ordering::Relaxed) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    // After the buffer has drained, for the same reason key-up ran before it:
    // this drops the transmit GPO, and an amplifier switched out from under a
    // signal that is still there is one arcing its relay.
    if shared.tdd {
        shared.phy.set_ensm_mode(conn, ENSM_RX)?;
    }
    shared.rx_enabled.store(true, Ordering::Relaxed);
    if rx_hz > 0.0 {
        shared.phy.set_rx_lo(conn, PlutoConfig::apply_ppm(rx_hz, ppm))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Pluto's gadget interface is IPv4-only. On a host where `pluto.local`
    /// also resolves to a link-local IPv6 address, picking that one produces a
    /// connection that can never succeed.
    #[test]
    fn resolution_prefers_ipv4() {
        let addr = resolve("127.0.0.1:30431").expect("loopback");
        assert!(addr.is_ipv4());
        assert_eq!(addr.port(), 30431);
        // The default port is filled in when the operator gives only a host.
        assert_eq!(resolve("127.0.0.1").expect("bare").port(), crate::DEFAULT_PORT);
    }

    /// The property the transmit buffer exists to have. `WRITEBUF` is
    /// synchronous and the device plays nothing while one is in flight, so a
    /// buffer that covers less airtime than the round trip costs guarantees a
    /// gap in the modulation on every single one — which is what a flat 4096
    /// samples did at 2.5 Msps: 1.64 ms of buffer against a ~2.2 ms round trip.
    #[test]
    fn a_transmit_buffer_outlasts_the_round_trip_that_refills_it() {
        // Generous next to the ~2.2 ms measured over a Pluto's USB gadget.
        let round_trip_ms = 5.0;
        // Up to about 5 Msps, which is already 20 MB/s of transmit payload —
        // past what that gadget carries, so the rates above it are theoretical.
        for rate in [2_083_333.0, 2_500_000.0, 3_072_000.0, 4_000_000.0, 5_000_000.0] {
            let ms = tx_buffer_samples(rate) as f64 / rate * 1e3;
            assert!(
                ms > round_trip_ms * 3.0,
                "{rate} sps gives a {ms:.2} ms transmit buffer, too short to stay ahead"
            );
        }
        // Bounded the other way by the sample cap, which is there for the
        // device-side allocation: 25 ms at the AD9361's top rate would ask a
        // Pluto for megabytes of DMA memory it has no way to fill over USB.
        assert_eq!(tx_buffer_samples(30_720_000.0), TX_BUFFER_BOUNDS.1);
        // Key-up latency is capped by the *time*, not the count, so it does not
        // grow with the rate.
        assert!(tx_buffer_samples(2_500_000.0) as f64 / 2.5e6 < 0.030);
        assert_eq!(tx_buffer_samples(0.0), TX_BUFFER_BOUNDS.0);
    }

    #[test]
    fn a_bad_address_is_reported_not_guessed_at() {
        assert!(resolve("").is_err());
        assert!(resolve("192.168.2.1:not-a-port").is_err());
    }
}
