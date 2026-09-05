//! The two sample-carrying threads: one owns the receive buffer, one the
//! transmit buffer, and each has its own IIOD connection.
//!
//! Neither thread touches a front-end register — that is the control thread's
//! job, on a third connection. All either does is move bytes and convert them,
//! which is what keeps a `READBUF` that blocks for a whole buffer period from
//! delaying a retune.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use rtrb::{Consumer, Producer};

use crate::error::Error;
use crate::iiod::Connection;
use crate::net::{CONNECT_TIMEOUT, STATS_INTERVAL, Shared};

/// I and Q of the transmit buffer's one pair. `Phy::probe` guarantees it sits
/// at scan indices 0 and 1; a 2R2T device's second transmit pair stays
/// disabled and off the wire. The receive side is no longer a constant — it
/// opens with one or two pairs as streams attach (see `Shared::rx_pairs`).
const TX_IQ_CHANNELS: usize = 2;

/// `-EAGAIN`: the server's own device timeout expired with nothing to hand
/// over. Normal while a device is idle, and not a reason to tear anything down.
const EAGAIN: i64 = -11;
/// `-ETIMEDOUT`, the other spelling of the same thing.
const ETIMEDOUT: i64 = -110;
/// `-EBUSY`: somebody already holds this buffer. Ordinarily that is another
/// program and there is nothing to do about it — but for the few hundred
/// milliseconds after [`redial_rx`] drops a socket, it is *us*, on the
/// connection `iiod` has not finished reaping yet. See [`BUSY_PATIENCE`].
const EBUSY: i64 = -16;

/// How long a receive buffer that answers `-EBUSY` is retried before the
/// refusal is taken at face value.
///
/// The reconnect in [`redial_rx`] closes one socket and opens another straight
/// away, and `iiod` frees a client's device buffer from the connection thread
/// that noticed the close — so for a moment the daemon believes the buffer is
/// still held by a client that is already gone. Waiting it out is the
/// difference between a reconnect that works and one that reports the radio as
/// busy with itself.
const BUSY_PATIENCE: Duration = Duration::from_millis(1_500);

/// How many times the receive socket may be replaced without a single sample
/// arriving in between before the connection is given up on.
///
/// A redial is cheap and worth trying, but a link that stalls again the moment
/// it is re-established is not one this layer can fix by trying harder. Giving
/// up hands the radio to the engine's reopen, which backs off, re-reads the
/// device and puts the reason on screen — all things this thread cannot do.
const MAX_BLIND_REDIALS: u32 = 5;

/// How long to wait before replacing the receive socket, so a link that fails
/// instantly on every attempt cannot become a tight loop of TCP handshakes.
const REDIAL_BACKOFF: Duration = Duration::from_millis(100);

/// The shortest a receive buffer may go without producing a sample before it is
/// closed and reopened rather than read again — see [`silence_before_restart`].
const RESTART_FLOOR: Duration = Duration::from_millis(1_000);

/// How long a receive buffer that is answering "nothing yet" is given before it
/// is closed and reopened.
///
/// `-EAGAIN` on its own is ordinary and absorbed by reading again: it is what
/// the server says when its own device timeout expired with nothing to hand
/// over. What is *not* ordinary is that answer arriving over and over from a
/// buffer that was streaming a moment ago, which is the shape a stalled DMA
/// takes from this side of the link — and no number of further `READBUF`s on
/// the same buffer will restart it. Reopening might; it costs a few
/// milliseconds and it happens on the connection we already have, where the
/// alternative is the engine's watchdog tearing the whole radio down and
/// dialling it again.
///
/// Scaled by the buffer's own airtime rather than fixed, because a buffer only
/// produces once per that period: an operator who asks for a megasample buffer
/// at the lowest rate on offer is legitimately two seconds between deliveries,
/// and a fixed floor would have us reopening a perfectly healthy stream forever.
fn silence_before_restart(buffer_samples: usize, rate_hz: f64) -> Duration {
    let span = Duration::from_secs_f64(buffer_samples as f64 / rate_hz.max(1.0));
    (span * 3).max(RESTART_FLOOR)
}

/// How long to leave a buffer alone after it has answered "nothing yet" before
/// asking it again.
///
/// `-EAGAIN` is meant to cost a whole server-side device timeout to collect —
/// the server sits on the `READBUF` until [`SERVER_TIMEOUT_MS`] runs out, so
/// asking again the moment it answers is one command every three seconds and
/// pacing it would be pointless. That assumption is not a guarantee: a
/// firmware that does not honour `TIMEOUT`, or a buffer whose DMA has stopped,
/// answers immediately instead, and then the retry loop is bounded by nothing
/// but the round-trip time and sits on a core at 100% for as long as the
/// condition lasts.
///
/// Waiting a fraction of the buffer's own airtime costs nothing even when the
/// device is healthy, because a buffer cannot produce more often than that
/// anyway; the ceiling keeps the added latency small on the long buffers where
/// a quarter of the airtime would be a visible delay.
///
/// [`SERVER_TIMEOUT_MS`]: crate::iiod
fn pace_after_empty(buffer_samples: usize, rate_hz: f64) -> Duration {
    let span = Duration::from_secs_f64(buffer_samples as f64 / rate_hz.max(1.0));
    (span / 4).clamp(Duration::from_millis(1), Duration::from_millis(20))
}

/// Periodic throughput accounting, emitted once per [`STATS_INTERVAL`] so a
/// tester can see whether samples are flowing — and at what rate — without a
/// per-buffer log flood. A wrong sample layout or a wrong rate shows up
/// immediately as an implausible ksps figure.
struct Stats {
    what: &'static str,
    nominal_hz: f64,
    /// Whether both directions are sharing the link, so the shortfall warning
    /// can name the setting that put them there.
    full_duplex: bool,
    /// How long the buffer has actually been open, summed across every window.
    ///
    /// Wall time since the thread started is the wrong denominator: the
    /// transmit buffer is closed between overs and the receive buffer is closed
    /// *during* them, so counting the idle stretches reported clock errors
    /// hundreds of thousands of ppm out — a figure alarming enough in a debug
    /// log to send a tester after the wrong fault entirely.
    active: Duration,
    since: Instant,
    win_buffers: u64,
    win_samples: u64,
    win_dropped: u64,
    /// Discarded while the engine was not reading this receiver because the
    /// station was transmitting. Counted apart from `win_dropped` because it
    /// is not a fault: see [`Stats::on_dropped_keyed`].
    win_keyed: u64,
    total_samples: u64,
    total_dropped: u64,
    total_keyed: u64,
}

impl Stats {
    fn new(what: &'static str, nominal_hz: f64, full_duplex: bool) -> Stats {
        Stats {
            what,
            nominal_hz,
            full_duplex,
            active: Duration::ZERO,
            since: Instant::now(),
            win_buffers: 0,
            win_samples: 0,
            win_dropped: 0,
            win_keyed: 0,
            total_samples: 0,
            total_dropped: 0,
            total_keyed: 0,
        }
    }

    /// The buffer just opened: start a fresh window, so the first figure after
    /// an idle stretch measures streaming and not the wait that preceded it.
    fn opened(&mut self) {
        self.since = Instant::now();
        self.win_buffers = 0;
        self.win_samples = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
    }

    /// The buffer just closed: bank the part-window that will never be
    /// reported, so its samples and its time stay paired in the clock estimate.
    fn closed(&mut self) {
        self.active += self.since.elapsed();
    }

    fn on_buffer(&mut self, pairs: usize) {
        self.win_buffers += 1;
        self.win_samples += pairs as u64;
        self.total_samples += pairs as u64;
    }

    fn on_dropped(&mut self, pairs: usize) {
        self.win_dropped += pairs as u64;
        self.total_dropped += pairs as u64;
    }

    /// Record `pairs` complex samples discarded because the ring was full while
    /// the station was transmitting.
    ///
    /// Not this radio's own over: a half-duplex Pluto closes its receive buffer
    /// for the length of one, so nothing arrives to discard. This is the case
    /// where it is somebody else's panadapter — the engine stops reading the
    /// keyed radio's source and empties the ring on unkey, while this receiver,
    /// which is not the transmitter, carries on streaming throughout. The ring
    /// fills within its own depth of key-down and everything after that is
    /// discarded: expected, at exactly the sample rate, for as long as the
    /// operator transmits. Counting it as an overrun turns an ordinary over
    /// into a warning that blames the DSP thread and advises a lower sample
    /// rate. See `IqSource::set_rx_paused`.
    fn on_dropped_keyed(&mut self, pairs: usize) {
        self.win_keyed += pairs as u64;
        self.total_keyed += pairs as u64;
    }

    /// What this receiver threw away while the station was transmitting,
    /// phrased so it cannot be read as a fault. Empty when the operator did not
    /// key up, so it costs nothing on a receive-only session.
    fn keyed_note(&self) -> String {
        if self.win_keyed == 0 {
            return String::new();
        }
        format!(
            "; {} sample(s) discarded while keyed (expected — this receiver is not read \
             during an over); {} discarded while keyed in total",
            self.win_keyed, self.total_keyed,
        )
    }

    /// The device's sample clock measured against the host's, in ppm, once
    /// enough time has passed for the figure to mean anything. The same
    /// oscillator drives the AD9361's synthesiser, so this is also the tuning
    /// error: a Pluto reading tens of ppm here is tens of ppm off frequency,
    /// which is what the `ppm` setting on the Radio tab is for.
    fn clock_error(&self) -> String {
        let dt = self.active.as_secs_f64();
        if dt < 20.0 || self.total_samples == 0 || self.nominal_hz <= 0.0 {
            return "clock: measuring".to_string();
        }
        let measured = self.total_samples as f64 / dt;
        let ppm = (measured / self.nominal_hz - 1.0) * 1e6;
        format!("clock: {measured:.0} sps, {ppm:+.1} ppm vs nominal")
    }

    fn tick(&mut self) {
        let dt = self.since.elapsed();
        if dt < STATS_INTERVAL {
            return;
        }
        let ksps = self.win_samples as f64 / dt.as_secs_f64() / 1000.0;
        // A direction that cannot carry the full rate is the fault that used to
        // need an oscilloscope to find: on transmit the device empties its
        // buffer faster than the next one arrives, so the carrier drops out
        // between buffers and the envelope goes out chopped. Nothing else in
        // the program can see it, so it is said here.
        // 5 %, not a hair's breadth: a window ends on a buffer boundary while
        // its clock does not, and the first buffer of an over is waited for
        // from an empty ring, so a percent or two of shortfall is bookkeeping.
        if self.win_dropped == 0 && self.nominal_hz > 0.0 && ksps * 1e3 < self.nominal_hz * 0.95 {
            tracing::warn!(
                "PlutoSDR {}: {ksps:.1} of {:.1} ksps over {:.2}s — the link is not carrying the \
                 full sample rate. On transmit this leaves the modulation with gaps in it; \
                 lower the sample rate, or reach the radio over Ethernet rather than the USB \
                 gadget{}",
                self.what,
                self.nominal_hz / 1e3,
                dt.as_secs_f64(),
                if self.full_duplex {
                    ". Full duplex is on, so an over is asking this link for both directions \
                     at once — turning it off halves what is being asked for"
                } else {
                    ""
                },
            );
        }
        if self.win_dropped > 0 {
            tracing::warn!(
                "PlutoSDR {}: {} buffers, {} samples ({ksps:.1} ksps) over {:.2}s; \
                 {} sample(s) DROPPED (ring full — the DSP thread is not keeping up; \
                 try a lower sample rate); {} dropped in total{}",
                self.what,
                self.win_buffers,
                self.win_samples,
                dt.as_secs_f64(),
                self.win_dropped,
                self.total_dropped,
                self.keyed_note(),
            );
        } else {
            tracing::debug!(
                "PlutoSDR {}: {} buffers, {} samples ({ksps:.1} ksps) over {:.2}s; {}{}",
                self.what,
                self.win_buffers,
                self.win_samples,
                dt.as_secs_f64(),
                self.clock_error(),
                self.keyed_note(),
            );
        }
        self.active += dt;
        self.since = Instant::now();
        self.win_buffers = 0;
        self.win_samples = 0;
        self.win_dropped = 0;
        self.win_keyed = 0;
    }
}

/// Push one buffer's interleaved I/Q into the ring, keeping I and Q paired: if
/// the ring cannot take the whole buffer, the whole buffer is dropped. Pushing
/// what fits would leave the ring one float out of step, which swaps I with Q
/// for the rest of the session — a mirrored, unusable spectrum that reads as a
/// protocol bug rather than the overrun it is.
///
/// `paused` says whether the engine has stopped reading for an over, which
/// decides how a full ring is accounted for — a fault, or the normal cost of
/// transmitting. It is deliberately not a reason to skip the push: the samples
/// are still offered, and it is the reader's business whether it wants them.
fn push_iq(ring: &mut Producer<f32>, iq: &[f32], stats: &mut Stats, paused: bool) {
    let Ok(mut chunk) = ring.write_chunk(iq.len()) else {
        if paused {
            stats.on_dropped_keyed(iq.len() / 2);
        } else {
            stats.on_dropped(iq.len() / 2);
        }
        return;
    };
    let (head, tail) = chunk.as_mut_slices();
    head.copy_from_slice(&iq[..head.len()]);
    tail.copy_from_slice(&iq[head.len()..]);
    chunk.commit_all();
}

/// Replace the receive socket after it has failed, without taking the rest of
/// the connection down with it.
///
/// A stalled receive socket is not a stalled radio, and the difference is
/// measurable: the fault this exists for wedges one TCP connection mid-payload
/// for seconds at a time while `iiod` on the same board answers a *fresh*
/// connection in single-digit milliseconds. The old response was
/// [`Shared::die`], which clears `alive` for the whole rig — control and
/// transmit sockets included — and leaves the engine to redial the device from
/// scratch: read the context XML again, set the whole front end again, and
/// swap the source. That is about a second of dead audio on top of however
/// long the read waited before giving up.
///
/// Dialling one socket and reopening one buffer is ~50 ms, and it keeps the
/// control connection — with it the dial, the gain and a transmission in
/// progress — untouched.
///
/// Returns `None` when the connection should be given up on, having already
/// reported why through [`Shared::die`].
fn redial_rx(shared: &Shared, cause: &Error) -> Option<Connection> {
    tracing::warn!("PlutoSDR: the receive socket failed ({cause}) — replacing it");
    shared.trace.note(format!("~~ receive socket failed ({cause}); redialling"));
    // First, and before anything is dialled. `iiod` will not hand the same
    // buffer to two connections, and until it has noticed that this one is
    // finished the replacement's `OPEN` meets our own ghost as `-EBUSY` —
    // which is why `open_rx_buffer` waits that answer out rather than
    // believing it. Shutting the socket down at both ends is what a server
    // blocked writing into it actually notices; the descriptor itself goes
    // when the caller installs the replacement a few milliseconds later.
    shared.drop_rx_socket();
    std::thread::sleep(REDIAL_BACKOFF);
    if !shared.alive.load(Ordering::Relaxed) {
        return None;
    }
    let conn = match Connection::connect(shared.addr, CONNECT_TIMEOUT, shared.trace.clone()) {
        Ok(conn) => conn,
        Err(e) => {
            // The board is not merely stalled, it is unreachable — which is the
            // engine's problem to solve, and it names the original fault rather
            // than the redial so the report says what actually broke.
            shared.die("the receive stream", cause);
            shared.trace.note(format!("!! the receive socket could not be replaced: {e}"));
            return None;
        }
    };
    if let Err(e) = shared.adopt_rx_socket(&conn) {
        tracing::debug!("PlutoSDR: receive socket replaced during shutdown ({e})");
        return None;
    }
    tracing::info!("PlutoSDR: receive socket replaced; reopening the buffer");
    Some(conn)
}

/// Open the receive buffer, riding out the `-EBUSY` that follows our own
/// reconnect — see [`BUSY_PATIENCE`].
fn open_rx_buffer(conn: &mut Connection, shared: &Shared, pairs: usize) -> Result<(), Error> {
    let deadline = Instant::now() + BUSY_PATIENCE;
    loop {
        match conn.open_buffer(&shared.phy.rx_buffer_id, shared.buffer_samples, pairs * 2) {
            Err(Error::Remote { code: EBUSY, .. }) if Instant::now() < deadline => {
                shared.trace.note("~~ receive buffer still busy; waiting for iiod to release it");
                std::thread::sleep(Duration::from_millis(50));
                if !shared.alive.load(Ordering::Relaxed) {
                    return Err(Error::Msg("this connection is closing".into()));
                }
            }
            other => return other,
        }
    }
}

/// Owns the receive buffer.
///
/// The buffer is opened with one or two I/Q pairs enabled, following
/// `Shared::rx_pairs` — a second radio attaching to the other chain widens
/// it, and its departure narrows it back. The channel mask is a property of
/// an *open* buffer, so a change means close-and-reopen: a brief gap on the
/// surviving stream, not a glitch in its alignment.
pub(crate) fn rx_thread(mut conn: Connection, shared: Arc<Shared>, mut ring: Producer<f32>) {
    let phy = &shared.phy;
    let max_pairs = phy.rx_pairs_available();
    let mut raw = vec![0u8; shared.buffer_samples * phy.rx_sample_bytes(max_pairs)];
    let mut iq: Vec<f32> = Vec::with_capacity(shared.buffer_samples * 2);
    let mut iq1: Vec<f32> = Vec::with_capacity(shared.buffer_samples * 2);
    let mut stats = Stats::new("RX", shared.rate_hz, shared.full_duplex);
    let mut open_pairs = 0usize;
    // When this buffer last produced anything, and how long it may go without
    // before it is reopened. Reset on every open so a buffer is judged from
    // when it started, not from the last one's death.
    let patience = silence_before_restart(shared.buffer_samples, shared.rate_hz);
    // How long to leave an empty buffer alone before asking it again — see
    // [`pace_after_empty`].
    let empty_pace = pace_after_empty(shared.buffer_samples, shared.rate_hz);
    let mut last_sets = Instant::now();
    // Consecutive socket replacements with no samples in between — see
    // [`MAX_BLIND_REDIALS`].
    let mut blind_redials = 0u32;
    // How long a payload read may stall on this stream's socket. Raised when a
    // link that had been delivering perfectly well stalls anyway, and carried
    // across redials so each replacement inherits what the last one learnt —
    // see `Connection::set_payload_deadline`.
    let mut payload_deadline = conn.payload_deadline();

    while shared.alive.load(Ordering::Relaxed) {
        let want_pairs = shared.rx_pairs.load(Ordering::Relaxed).clamp(1, max_pairs);
        if !shared.rx_enabled.load(Ordering::Relaxed) {
            if open_pairs > 0 {
                let _ = conn.close_buffer(&phy.rx_buffer_id);
                open_pairs = 0;
                stats.closed();
                shared.rx_active.store(false, Ordering::Relaxed);
            }
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        if open_pairs > 0 && open_pairs != want_pairs {
            let _ = conn.close_buffer(&phy.rx_buffer_id);
            open_pairs = 0;
            stats.closed();
            tracing::debug!("PlutoSDR: receive buffer reopening for {want_pairs} pair(s)");
        }
        if open_pairs == 0 {
            if let Err(e) = open_rx_buffer(&mut conn, &shared, want_pairs) {
                shared.die("the receive buffer", &e);
                break;
            }
            open_pairs = want_pairs;
            stats.opened();
            last_sets = Instant::now();
            shared.rx_active.store(true, Ordering::Relaxed);
            tracing::debug!(
                "PlutoSDR: receive buffer open, {} samples × {} bytes ({} pair(s))",
                shared.buffer_samples,
                phy.rx_sample_bytes(open_pairs),
                open_pairs
            );
        }
        // Request exactly one device buffer's worth for the layout that is
        // open — `raw` is sized for the widest, and asking for more bytes
        // than the buffer carries would change the request on the wire.
        let set_bytes = phy.rx_sample_bytes(open_pairs);
        let want_bytes = shared.buffer_samples * set_bytes;
        // An open buffer that has stopped producing is reopened rather than
        // read again — see `silence_before_restart`. Checked here rather than
        // in the `-EAGAIN` arm below so that every shape of silence is caught,
        // including a server answering with an empty buffer. The clock is reset
        // by the reopen, so a device that is genuinely gone costs one restart
        // per window rather than a tight loop of them while the engine's
        // watchdog runs down.
        if last_sets.elapsed() >= patience {
            tracing::warn!(
                "PlutoSDR: no samples for {:.1}s — reopening the receive buffer",
                last_sets.elapsed().as_secs_f64()
            );
            shared.trace.note("~~ receive buffer reopened after a silence");
            let _ = conn.close_buffer(&phy.rx_buffer_id);
            open_pairs = 0;
            stats.closed();
            shared.rx_active.store(false, Ordering::Relaxed);
            last_sets = Instant::now();
            continue;
        }
        let asked_at = Instant::now();
        let n = match conn.read_buf(&phy.rx_buffer_id, open_pairs * 2, &mut raw[..want_bytes]) {
            Ok(n) => n,
            Err(Error::Remote { code, .. }) if code == EAGAIN || code == ETIMEDOUT => {
                // Nothing yet, which is ordinary. Ask again — but not sooner
                // than the buffer could possibly have refilled, or a server
                // that refuses instantly turns this into a spin. See
                // [`pace_after_empty`].
                if let Some(rest) = empty_pace.checked_sub(asked_at.elapsed()) {
                    std::thread::sleep(rest);
                }
                continue;
            }
            // The server answered, so the link works and the refusal is
            // considered: that is the engine's to act on, not ours to retry.
            Err(e @ Error::Remote { .. }) => {
                shared.die("the receive stream", &e);
                break;
            }
            // Anything else — a socket that stopped delivering, framing that no
            // longer makes sense — is this connection being finished, which is
            // not the same as the radio being finished. Replace it in place.
            Err(e) => {
                if blind_redials >= MAX_BLIND_REDIALS {
                    shared.die("the receive stream", &e);
                    break;
                }
                blind_redials += 1;
                // A payload that ran out of time on a socket which had been
                // delivering samples is not a wedge — it is a link with less
                // headroom than it is being asked for, and the bytes were on
                // their way. Replacing the socket throws away a transfer that
                // would have finished, so the replacement gets longer before
                // the same call is made about it.
                if e.is_payload_stall() && stats.total_samples > 0 {
                    payload_deadline *= 2;
                    tracing::info!(
                        "PlutoSDR: the receive link stalls under load — allowing a payload                          {:.1}s to arrive before the socket is replaced",
                        payload_deadline.as_secs_f64()
                    );
                }
                let Some(mut fresh) = redial_rx(&shared, &e) else { break };
                fresh.set_payload_deadline(payload_deadline);
                payload_deadline = fresh.payload_deadline();
                conn = fresh;
                open_pairs = 0;
                stats.closed();
                shared.rx_active.store(false, Ordering::Relaxed);
                continue;
            }
        };
        let sets = n / set_bytes;
        if sets == 0 {
            continue;
        }
        last_sets = Instant::now();
        blind_redials = 0;
        let i_bytes = phy.rx_scan[0].bytes();
        iq.clear();
        iq1.clear();
        for s in 0..sets {
            let mut off = s * set_bytes;
            iq.push(phy.rx_scan[0].decode(&raw[off..off + i_bytes]));
            off += i_bytes;
            let q_bytes = phy.rx_scan[1].bytes();
            iq.push(phy.rx_scan[1].decode(&raw[off..off + q_bytes]));
            off += q_bytes;
            if open_pairs == 2 {
                let i2 = phy.rx_scan[2].bytes();
                iq1.push(phy.rx_scan[2].decode(&raw[off..off + i2]));
                off += i2;
                let q2 = phy.rx_scan[3].bytes();
                iq1.push(phy.rx_scan[3].decode(&raw[off..off + q2]));
            }
        }
        let paused = shared.rx_paused.load(Ordering::Relaxed);
        push_iq(&mut ring, &iq, &mut stats, paused);
        // Stamped by this thread rather than by the reader, so an over — during
        // which nothing drains the ring — is not read as a dead radio.
        shared.stamp_rx();
        if open_pairs == 2 {
            // The second chain's radio may not have attached its ring yet (or
            // may just have dropped it); its samples then fall on the floor,
            // which is what "nobody is listening" should cost.
            let mut slot = shared.ring1.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(r1) = slot.as_mut() {
                push_iq(r1, &iq1, &mut stats, paused);
                shared.stamp_rx1();
            }
        }
        stats.on_buffer(sets);
        stats.tick();
    }

    if open_pairs > 0 {
        let _ = conn.close_buffer(&phy.rx_buffer_id);
    }
    shared.rx_active.store(false, Ordering::Relaxed);
    conn.exit();
    tracing::debug!("PlutoSDR: receive thread finished");
}

/// Owns the transmit buffer.
///
/// `WRITEBUF` does not return until the device has taken the data, so this loop
/// is paced by the hardware itself — there is no clock on this side to drift.
///
/// # What has to hold for the carrier to be continuous
///
/// The device plays nothing while a `WRITEBUF` is in flight, so one buffer must
/// cover *more airtime than the round trip costs* — see
/// [`crate::net::tx_buffer_samples`], which is what sizes it. Given that, the
/// device's queue depth is conserved: the engine produces one buffer's worth of
/// samples in exactly the airtime that buffer covers, so each push replaces
/// what was played since the last one, and the lead established by the first
/// push of an over survives to the end of it.
pub(crate) fn tx_thread(mut conn: Connection, shared: Arc<Shared>, mut ring: Consumer<f32>) {
    let phy = &shared.phy;
    let set_bytes = phy.tx_sample_bytes();
    let i_bytes = phy.tx_scan[0].bytes();
    let pairs = shared.tx_buffer_samples;
    let mut raw = vec![0u8; pairs * set_bytes];
    let mut stats = Stats::new("TX", shared.tx_rate_hz, shared.full_duplex);
    let mut open = false;

    // How long a full buffer takes the engine to produce, which is what the
    // wait below is really waiting for. Scaled rather than fixed: a flat 20 ms
    // was shorter than a buffer at any useful rate once buffers grew past a few
    // milliseconds, so every single one would have been cut short and padded —
    // trading the old gap between buffers for a gap inside each of them.
    let buffer_span = Duration::from_secs_f64(pairs as f64 / shared.tx_rate_hz.max(1.0));
    let fill_wait = buffer_span * 2 + Duration::from_millis(20);

    while shared.alive.load(Ordering::Relaxed) {
        if !shared.tx_enabled.load(Ordering::Relaxed) {
            if open {
                let _ = conn.close_buffer(&phy.tx_buffer_id);
                open = false;
                stats.closed();
                shared.tx_active.store(false, Ordering::Relaxed);
            }
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        if !open {
            if let Err(e) = conn.open_buffer(&phy.tx_buffer_id, pairs, TX_IQ_CHANNELS) {
                shared.die("the transmit buffer", &e);
                break;
            }
            open = true;
            stats.opened();
            shared.tx_active.store(true, Ordering::Relaxed);
            tracing::debug!(
                "PlutoSDR: transmit buffer open, {pairs} samples ({:.1} ms)",
                buffer_span.as_secs_f64() * 1e3
            );
        }

        // Give the engine time to fill a whole buffer before padding with
        // silence. The first buffers of an over are legitimately short — the
        // ring starts empty — and sending them as-is would put a gap in the
        // middle of the modulation instead of only at the very start.
        let want = pairs * 2;
        let deadline = Instant::now() + fill_wait;
        while ring.slots() < want
            && Instant::now() < deadline
            && shared.tx_enabled.load(Ordering::Relaxed)
        {
            std::thread::sleep(Duration::from_micros(200));
        }

        let mut short = 0usize;
        for p in 0..pairs {
            let off = p * set_bytes;
            let (i, q) = match (ring.pop(), ring.pop()) {
                (Ok(i), Ok(q)) => (i, q),
                // Only ever an odd tail if the ring were pushed a half pair,
                // which `tx_write` makes impossible; treat it as silence.
                _ => {
                    short += 1;
                    (0.0, 0.0)
                }
            };
            phy.tx_scan[0].encode(i, &mut raw[off..off + i_bytes]);
            phy.tx_scan[1].encode(q, &mut raw[off + i_bytes..off + set_bytes]);
        }
        if short > 0 {
            tracing::trace!("PlutoSDR TX: padded {short} of {pairs} samples with silence");
        }
        match conn.write_buf(&phy.tx_buffer_id, &raw) {
            Ok(_) => {
                stats.on_buffer(pairs);
                stats.tick();
            }
            Err(Error::Remote { code, .. }) if code == EAGAIN || code == ETIMEDOUT => continue,
            Err(e) => {
                shared.die("the transmit stream", &e);
                break;
            }
        }
    }

    if open {
        let _ = conn.close_buffer(&phy.tx_buffer_id);
    }
    shared.tx_active.store(false, Ordering::Relaxed);
    conn.exit();
    tracing::debug!("PlutoSDR: transmit thread finished");
}
