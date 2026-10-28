//! The N2ADR **HL2IOBoard**: a Raspberry Pi Pico on the Hermes-Lite 2's
//! accessory connector that switches bands for an external amplifier, a
//! transverter or a loop antenna.
//!
//! The board cannot see either frequency — the Hermes-Lite's own registers never
//! reach it — so the host has to tell it:
//!
//! > The only required SDR modification is sending the transmit frequency. SDR
//! > software must send the transmit frequency to provide band information to
//! > power amps, transverters and loop antennas.
//!
//! Its documentation asks a host for four things, and this does all four:
//!
//! - **Reset** (`REG_CONTROL`) once at startup, since the board's registers are
//!   static and outlive the program that last set them.
//! - **The transmit frequency**, which is what an amplifier switches bands on.
//! - **The receive frequency**, as a one-byte code, which the board uses to
//!   choose a receive antenna and preselector.
//! - **`REG_RF_INPUTS`**, whether the board's own SMA jacks replace the radio's
//!   receive input — the operator's answer to how they wired it.
//!
//! Everything else the board does (which output drives which relay, the fan, the
//! tuner) belongs in the operator's own Pico firmware, deliberately: the author
//! asks that SDR programs *not* grow logic for it.
//!
//! # How the bytes get there
//!
//! Protocol 1 has no direct path to an accessory board, so the Hermes-Lite
//! tunnels I2C through its C&C registers. Register `0x3D` is I2C bus 2 — the one
//! brought out to the accessory connector — and one C&C block carries exactly
//! one one-byte I2C transaction:
//!
//! | byte | meaning |
//! |------|---------|
//! | C0   | `0x3D << 1`, plus the MOX bit and the RQST bit (see below) |
//! | C1   | `0x06` to write, `0x07` to read |
//! | C2   | the 7-bit I2C address of the target |
//! | C3   | the register inside that target |
//! | C4   | the byte to write (unused for a read) |
//!
//! Setting RQST (C0 bit 7) makes the gateware answer, which is the only way to
//! know a command arrived: EP2 is UDP, and an I2C write that fell on the floor
//! would leave the amplifier on the wrong band with nothing to say so. The
//! answer comes back on EP6 with its own ACK bit set, echoing the address —
//! or address `0x3F`, which is the gateware reporting that the transaction
//! failed (nothing acknowledged that I2C address).
//!
//! The Hermes-Lite 2 protocol document asks that only one such request be
//! outstanding at a time, so this runs strictly one transaction at a time.
//!
//! Byte layout and register numbers follow the HL2IOBoard documentation and
//! Quisk's `hermes/quisk_hardware.py`, both by the board's author.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sdroxide_types::HpsdrIoRxInput;

/// C&C address `0x3D` — I2C bus 2 — already shifted into C0's address field.
/// The MOX bit (0) and RQST bit (7) are OR'd in per datagram.
const CC_I2C2: u8 = 0x3D << 1;
/// C0 bit 7: "acknowledge this one". The Hermes-Lite calls it RQST.
const CC_RQST: u8 = 0x80;
/// The address the gateware answers with when the transaction failed.
const RADDR_ERROR: u8 = 0x3F;
/// C1 cookies for the I2C tunnel.
const I2C_WRITE: u8 = 0x06;
const I2C_READ: u8 = 0x07;

/// The board's Pico listens here. Distinct from a filter board's `0x20`, so the
/// two accessories coexist on the one bus.
const IOB_ADDR: u8 = 0x1D;
/// A read-only hardware-version register, answered by the board itself rather
/// than by the Pico's firmware — so it identifies the board even if the Pico is
/// unprogrammed or wedged.
const IOB_VERSION_ADDR: u8 = 0x41;
/// That register reads back `0xF` in the top nibble and the hardware version in
/// the bottom (`0xF1` for the current board). Matching only the top nibble
/// means a later revision is still recognized, and its version gets logged.
const IOB_VERSION_MASK: u8 = 0xF0;

/// The transmit frequency is a five-byte big-endian integer in Hz spread over
/// registers 0..=4. Writing **BYTE0 last** is what latches it: the board acts on
/// the whole frequency when the least significant byte arrives, so the other
/// four can be sent in any order and are ignored until then.
const REG_TX_FREQ_BYTE4: u8 = 0;
const REG_TX_FREQ_BYTE0: u8 = 4;
/// Writing 1 here returns every register to zero. The board's registers are
/// static and survive a host going away, so its documentation asks that SDR
/// software clear them at startup rather than inherit whatever the last program
/// left behind.
const REG_CONTROL: u8 = 5;
/// AH-4 tuner status and command register in the Pico firmware.
const REG_ANTENNA_TUNER: u8 = 7;
/// Where the board takes its receive signal from: the radio's own input (0, and
/// what the reset leaves), the board's own J9 SMA (1), or J9 with the J10
/// PureSignal jack switched in on transmit (2).
const REG_RF_INPUTS: u8 = 11;
/// Receiver 1's frequency, as a one-byte *code* rather than a frequency —
/// registers 13..=24 hold one for each of the twelve receivers the protocol can
/// carry. The board uses them to pick a receive antenna and preselector; all
/// zero (which is what the reset leaves) means "receive on the transmit band".
///
/// Protocol 1 carries exactly one receiver, so only this one is ever written.
const REG_FCODE_RX1: u8 = 13;

/// How long to wait for an ACK before sending the request again.
const ACK_TIMEOUT: Duration = Duration::from_millis(150);
/// Minimum spacing between transmit-frequency updates. The board's
/// documentation asks for at most one every half second (Quisk's figure), since
/// an operator spinning the dial would otherwise flood the I2C bus.
const FREQ_INTERVAL: Duration = Duration::from_millis(500);
/// Probes before concluding there is no IO board on this radio.
const PROBE_TRIES: u8 = 3;
/// Gap between those probes.
const PROBE_INTERVAL: Duration = Duration::from_millis(500);
/// How often to read the AH-4 tuner status register.
const TUNER_POLL_INTERVAL: Duration = Duration::from_millis(100);
// The Pico register-7 KEY handshake determines whether an AH-4 is present.
/// Maximum age of a successful status read when authorising a tuning start.
const TUNER_STATUS_MAX_AGE: Duration = Duration::from_millis(200);
/// Maximum time allowed for AH-4 discovery and bypass-status verification.
const TUNER_INIT_TIMEOUT: Duration = Duration::from_secs(2);
/// Maximum time allowed for an AH-4 tuning cycle.
const TUNER_START_TIMEOUT: Duration = Duration::from_secs(4);
/// Maximum time allowed for AH-4 bypass recovery.
const TUNER_RECOVERY_TIMEOUT: Duration = Duration::from_secs(4);

/// One I2C transaction: read or write, one byte either way.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Op {
    cookie: u8,
    addr: u8,
    reg: u8,
    data: u8,
}

impl Op {
    fn write(reg: u8, data: u8) -> Op {
        Op { cookie: I2C_WRITE, addr: IOB_ADDR, reg, data }
    }
    fn read(reg: u8) -> Op {
        Op { cookie: I2C_READ, addr: IOB_ADDR, reg, data: 0 }
    }

    /// The five C&C bytes that carry this transaction, with `mox` riding along
    /// as it does on every other outgoing register.
    ///
    /// C2 is the bare 7-bit I2C address. Its top bit selects stop-at-end versus
    /// continue, and the Hermes-Lite 2 protocol document says the bit "is not
    /// currently used but should be 'stop' for future compatibility" — so it
    /// changes nothing on any gateware that exists, and this follows Quisk,
    /// which leaves it clear and is what the board is known to work with. Set
    /// it here if a later gateware starts honouring it.
    fn cc(&self, mox: u8) -> [u8; 5] {
        [CC_I2C2 | CC_RQST | mox, self.cookie, self.addr, self.reg, self.data]
    }
}

/// Whether this radio has an IO board on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Presence {
    /// Still asking, with this many probes left.
    Probing(u8),
    Present,
    /// Nothing answered; stop talking to the bus.
    Absent,
}

/// Stages of the AH-4 startup bypass test.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TunerInit {
    /// Startup frequency updates are not yet complete.
    WaitingForFrequencies,
    /// Discovery command 0x01 is queued after startup frequency writes.
    DiscoveryRequested,
    /// Waiting for the terminal KEY-handshake result: 0xFC (present) or 0xFB (absent).
    Discovering,
    /// Discovery outcome is latched; request bypass without enabling RF.
    DiscoveryBypassRequested,
    /// Waiting for bypass status; this must not alter the discovery outcome.
    VerifyingBypass,
    /// Initialisation has finished; monitor tuner status.
    Monitoring,
    /// Bypass recovery has been queued and awaits write acknowledgement.
    RecoveryBypassRequested,
    /// Waiting for the recovery bypass status to become idle.
    RecoveringBypass,
}

/// A transaction on the wire, waiting to be acknowledged.
struct InFlight {
    op: Op,
    sent_at: Instant,
}

/// Drives one HL2IOBoard: finds it, resets it, and keeps it told of the
/// transmit frequency.
pub(crate) struct IoBoard {
    presence: Presence,
    /// Most recently read AH-4 tuner status; None until the first successful read.
    tuner_status: Option<u8>,
    /// Time of the most recent successful AH-4 tuner status read.
    tuner_status_at: Option<Instant>,
    /// Whether an AH-4 start request is outstanding.
    tuner_start_pending: bool,
    /// Whether a non-idle status has been observed for this request.
    tuner_start_seen_active: bool,
    /// When the start request was accepted.
    tuner_start_started_at: Option<Instant>,
    /// When the next tuner status read is due.
    tuner_poll_at: Option<Instant>,
    /// When initial bypass verification began.
    tuner_verify_started_at: Option<Instant>,
    /// When AH-4 bypass recovery began.
    tuner_recovery_started_at: Option<Instant>,
    /// A timed-out recovery may still produce a late status result.
    tuner_recovery_timed_out: bool,
    /// Whether an AH-4 tuner has been detected as ready.
    tuner_present: bool,
    /// A latched fault prevents new tuning requests until recovery is verified.
    tuner_fault: bool,
    /// Blocks I2C traffic when an acknowledgement leaves transaction state uncertain.
    i2c_quarantined: bool,
    /// Current stage of the initial AH-4 bypass test.
    tuner_init: TunerInit,
    /// Number of initial frequency writes still awaiting acknowledgement.
    startup_writes_pending: u8,
    /// Whether the initial frequency batch has been queued.
    startup_frequencies_queued: bool,
    /// Which input the operator has wired the board for, stated once after the
    /// reset. Only the connection knows this; the board itself comes up at 0.
    rx_input: HpsdrIoRxInput,
    inflight: Option<InFlight>,
    queue: VecDeque<Op>,
    /// The transmit frequency the board has been told, so an unchanged dial is
    /// silent.
    sent_hz: Option<u64>,
    /// The receive frequency *code* it has been told. Held as the code rather
    /// than the frequency because that is what the board acts on, and a code
    /// spans a whole band (see [`hertz_to_code`]).
    sent_rx_code: Option<u8>,
    /// When the last frequency went out, for the half-second rate limit. Shared
    /// by both directions: it exists to keep the I2C bus quiet, and the bus does
    /// not care which of them is talking.
    sent_at: Option<Instant>,
    /// When the next probe may go out.
    probe_at: Option<Instant>,
}

impl IoBoard {
    /// Return the last successfully read tuner status and its original timestamp.
    pub(crate) fn tuner_status_snapshot(&self) -> Option<(u8, Instant)> {
        self.tuner_status.zip(self.tuner_status_at)
    }

    pub(crate) fn tuner_present(&self) -> bool {
        self.tuner_present
    }

    /// Discovery outcome, independent of the freshness of the status byte.
    /// None means discovery has not reached a result yet; callers must not
    /// treat it as positive AH-4 detection.
    pub(crate) fn tuner_presence(&self) -> Option<bool> {
        match self.tuner_init {
            TunerInit::WaitingForFrequencies
            | TunerInit::DiscoveryRequested
            | TunerInit::Discovering => None,
            _ => Some(self.tuner_present),
        }
    }

    /// Queue an AH-4 start command only for a verified, idle tuner.
    /// This method never enables transmitter RF.
    pub(crate) fn tuner_start(&mut self) -> bool {
        // Only start from a verified, idle state with no other I2C work
        // outstanding, and with a sufficiently recent status reading.
        if self.i2c_quarantined
            || self.presence != Presence::Present
            || !self.tuner_present()
            || self.tuner_fault
            || self.tuner_init != TunerInit::Monitoring
            || self.tuner_status != Some(0x00)
            || self.tuner_status_at.is_none_or(|t|
                Instant::now().saturating_duration_since(t) > TUNER_STATUS_MAX_AGE)
            || self.tuner_start_pending
            || self.inflight.is_some()
            || !self.queue.is_empty()
        {
            return false;
        }

        self.queue
            .push_back(Op::write(REG_ANTENNA_TUNER, 0x01));

        self.tuner_start_pending = true;
        self.tuner_start_seen_active = false;

        tracing::info!("HL2IOBoard: AH-4 start command queued");
        true
    }

    /// Request normal AH-4 bypass only from a fresh, verified idle state.
    /// This does not clear or override a latched tuner fault.
    pub(crate) fn tuner_bypass(&mut self) -> bool {
        if self.i2c_quarantined
            || self.presence != Presence::Present
            || !self.tuner_present()
            || self.tuner_fault
            || self.tuner_init != TunerInit::Monitoring
            || self.tuner_status != Some(0x00)
            || self.tuner_status_at.is_none_or(|t|
                Instant::now().saturating_duration_since(t) > TUNER_STATUS_MAX_AGE)
            || self.tuner_start_pending
            || self.inflight.is_some()
            || !self.queue.is_empty()
        {
            return false;
        }

        self.queue
            .push_back(Op::write(REG_ANTENNA_TUNER, 0x02));

        tracing::info!("HL2IOBoard: normal AH-4 bypass requested");
        true
    }

    /// Request AH-4 bypass recovery without enabling transmitter RF.
    pub(crate) fn tuner_recover(&mut self) -> bool {
        if self.i2c_quarantined
            || self.presence != Presence::Present
            || !self.tuner_fault
            || self.tuner_start_pending
            || self.inflight.is_some()
            || !self.queue.is_empty()
            || self.tuner_init != TunerInit::Monitoring
        {
            return false;
        }

        self.queue
            .push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
        self.tuner_recovery_timed_out = false;
        self.tuner_init = TunerInit::RecoveryBypassRequested;

        tracing::warn!("HL2IOBoard: AH-4 bypass recovery requested");
        true
    }

    pub(crate) fn new(rx_input: HpsdrIoRxInput) -> IoBoard {
        IoBoard {
            presence: Presence::Probing(PROBE_TRIES),
            tuner_status: None,
            tuner_status_at: None,
            tuner_start_pending: false,
            tuner_start_seen_active: false,
            tuner_start_started_at: None,
            tuner_poll_at: None,
            tuner_verify_started_at: None,
            tuner_recovery_started_at: None,
            tuner_recovery_timed_out: false,
            tuner_present: false,
            tuner_fault: false,
            i2c_quarantined: false,
            tuner_init: TunerInit::WaitingForFrequencies,
            startup_writes_pending: 0,
            startup_frequencies_queued: false,
            rx_input,
            inflight: None,
            queue: VecDeque::new(),
            sent_hz: None,
            sent_rx_code: None,
            sent_at: None,
            probe_at: None,
        }
    }

    /// Move the receive input while running.
    ///
    /// A board already found is told straight away; one still being probed
    /// picks it up with the rest of its start-up writes, after the reset that
    /// would otherwise undo it.
    pub(crate) fn set_rx_input(&mut self, input: HpsdrIoRxInput) {
        if self.rx_input == input {
            return;
        }
        self.rx_input = input;
        if self.presence == Presence::Present {
            self.queue.push_back(Op::write(REG_RF_INPUTS, input.code()));
            tracing::info!(
                "HL2IOBoard: receive input -> {} (mode {})",
                input.label(),
                input.code()
            );
        }
    }

    /// The C&C block to put in this datagram, or `None` to leave the slot to the
    /// ordinary register rotation — which is the answer almost every time, since
    /// this talks only when the frequency moves.
    ///
    /// `tx_freq_hz` is the frequency the radio would transmit on right now;
    /// `rx_freq_hz` is where its receiver is tuned. Both are on the *air*: with
    /// a transverter in front they are the transverter's band, not the I.F. the
    /// radio itself is on, and a 3 cm dial does not fit in 32 bits.
    pub(crate) fn next_request(
        &mut self,
        now: Instant,
        tx_freq_hz: u64,
        rx_freq_hz: u64,
        mox: u8,
    ) -> Option<[u8; 5]> {
        // After an uncertain ACK, no further I2C request is safe.
        if self.i2c_quarantined {
            return None;
        }

        if self.tuner_start_pending
            && self
                .tuner_start_started_at
                .is_some_and(|started| now >= started + TUNER_START_TIMEOUT)
        {
            tracing::error!(
                "HL2IOBoard: AH-4 tuning timed out; tuner state is uncertain"
            );
            self.tuner_start_pending = false;
            self.tuner_start_seen_active = false;
            self.tuner_start_started_at = None;
            self.tuner_present = false;
            self.tuner_fault = true;
        }

        if self.tuner_init == TunerInit::RecoveringBypass
            && self
                .tuner_recovery_started_at
                .is_some_and(|started| now >= started + TUNER_RECOVERY_TIMEOUT)
        {
            tracing::error!(
                "HL2IOBoard: AH-4 bypass recovery timed out; fault remains latched"
            );
            self.tuner_init = TunerInit::Monitoring;
            self.tuner_recovery_started_at = None;
            self.tuner_recovery_timed_out = true;
            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
            self.tuner_fault = true;
        }

        // Enforce the discovery deadline even while an I2C request is outstanding.
        // Keep the in-flight request tracked; bypass can only be sent when safe.
        if self.tuner_init == TunerInit::Discovering
            && self
                .tuner_verify_started_at
                .is_some_and(|started| now >= started + TUNER_INIT_TIMEOUT)
        {
            self.tuner_present = false;
            self.tuner_init = TunerInit::DiscoveryBypassRequested;
            self.tuner_verify_started_at = None;
            self.queue.clear();
            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
            tracing::warn!(
                "HL2IOBoard: AH-4 discovery timed out without a definitive KEY result; \
                 treating tuner as absent and requesting bypass"
            );
        }

        // An unanswered request leaves the I2C transaction outcome uncertain.
        // Do not retry or send another request until the bus is recovered.
        if let Some(f) = self.inflight.as_ref() {
            if now < f.sent_at + ACK_TIMEOUT {
                return None;
            }
        }

        if let Some(f) = self.inflight.take() {
            let op = f.op;
            self.give_up(op, now, "no acknowledgement within timeout");
            self.tuner_status = None;
            self.tuner_status_at = None;
            if self.tuner_present() || op.reg == REG_ANTENNA_TUNER {
                self.tuner_fault = true;
            }
            self.i2c_quarantined = true;
            tracing::error!(
                "HL2IOBoard: I2C bus quarantined after an unanswered request"
            );
            return None;
        }

        self.refill(now, tx_freq_hz, rx_freq_hz);
        let op = self.queue.pop_front()?;

        if op.cookie == I2C_WRITE
            && op.reg == REG_ANTENNA_TUNER
            && op.data == 0x01
            && self.tuner_start_pending
        {
            self.tuner_start_started_at = Some(now);
            tracing::info!("HL2IOBoard: AH-4 start command sent");
        }

        self.inflight = Some(InFlight { op, sent_at: now });
        Some(op.cc(mox))
    }

    /// Queue whatever there is to say, if anything.
    fn refill(&mut self, now: Instant, tx_freq_hz: u64, rx_freq_hz: u64) {
        if !self.queue.is_empty() {
            return;
        }
        match self.presence {
            Presence::Probing(_) => {
                if self.probe_at.is_none_or(|t| now >= t) {
                    self.probe_at = Some(now + PROBE_INTERVAL);
                    self.queue.push_back(Op {
                        cookie: I2C_READ,
                        addr: IOB_VERSION_ADDR,
                        reg: 0,
                        data: 0,
                    });
                }
            }
            Presence::Present => {
                // Bypass verification is diagnostic only and cannot alter the
                // presence result latched by the preceding KEY handshake.
                if self.tuner_init == TunerInit::VerifyingBypass
                    && self
                        .tuner_verify_started_at
                        .is_some_and(|started| now >= started + TUNER_INIT_TIMEOUT)
                {
                    self.tuner_init = TunerInit::Monitoring;
                    self.tuner_verify_started_at = None;
                    self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                    tracing::warn!(
                        "HL2IOBoard: post-discovery bypass verification timed out; \
                         retaining discovery result and continuing status monitoring"
                    );
                }

                let rx_code = hertz_to_code(rx_freq_hz);
                let tx_stale = self.sent_hz != Some(tx_freq_hz);
                let rx_stale = self.sent_rx_code != Some(rx_code);

                // Preserve frequency-update priority and rate limiting.
                if self.sent_at.is_some_and(|t| now < t + FREQ_INTERVAL) {
                    if matches!(
                        self.tuner_init,
                        TunerInit::Discovering
                            | TunerInit::VerifyingBypass
                            | TunerInit::Monitoring
                    ) && self.tuner_poll_at.is_none_or(|t| now >= t)
                    {
                        self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                        self.queue
                            .push_back(Op::read(REG_ANTENNA_TUNER));
                    }
                    return;
                }

                self.sent_at = Some(now);

                // Transmit frequency first, as before.
                if tx_stale {
                    self.sent_hz = Some(tx_freq_hz);
                    self.queue.extend(freq_writes(tx_freq_hz));
                    tracing::debug!(
                        "HL2IOBoard: transmit frequency -> {tx_freq_hz} Hz"
                    );
                }

                if rx_stale {
                    self.sent_rx_code = Some(rx_code);
                    self.queue
                        .push_back(Op::write(REG_FCODE_RX1, rx_code));
                    tracing::debug!(
                        "HL2IOBoard: receive frequency -> {rx_freq_hz} Hz \
                         (code {rx_code})"
                    );
                }

                // Count the first frequency batch once, not every subsequent change.
                if !self.startup_frequencies_queued
                    && self.tuner_init == TunerInit::WaitingForFrequencies
                {
                    self.startup_writes_pending = self.queue.len() as u8;
                    self.startup_frequencies_queued = true;
                }

                // Do not let a tuner poll jump ahead of queued frequency writes.
                if !self.queue.is_empty() {
                    return;
                }

                // Poll the tuner during bypass verification and normal monitoring.
                if matches!(
                    self.tuner_init,
                    TunerInit::Discovering | TunerInit::VerifyingBypass | TunerInit::Monitoring
                ) && self.tuner_poll_at.is_none_or(|t| now >= t)
                {
                    self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                    self.queue
                        .push_back(Op::read(REG_ANTENNA_TUNER));
                }
            }
            Presence::Absent => {}
        }
    }

    /// An acknowledgement came back on EP6. `raddr` is the address the gateware
    /// echoed and `data` is C1..C4 — for a read, the four bytes it fetched.
    pub(crate) fn on_ack(&mut self, raddr: u8, data: [u8; 4], now: Instant) {
        // ACKs cannot be safely associated with requests after an uncertain timeout.
        if self.i2c_quarantined {
            return;
        }

        // Only ACKs for the I2C bus this drives are ours. Nothing else in this
        // crate sets RQST, but taking someone else's answer would strand the
        // real request forever.
        let ours = raddr == CC_I2C2 >> 1 || raddr == RADDR_ERROR;
        let Some(f) = (if ours { self.inflight.take() } else { None }) else {
            return;
        };
        if raddr == RADDR_ERROR {
            // Nothing acknowledged that I2C address, or the bus was busy.
            if f.op.cookie == I2C_READ && f.op.reg == REG_ANTENNA_TUNER {
                self.tuner_status = None;
                self.tuner_status_at = None;
            }
            self.give_up(f.op, now, "the radio reported an I2C error");
            return;
        }
        if let Presence::Probing(_) = self.presence
            && f.op.cookie == I2C_READ
        {
            // The first byte the board returned is its version register.
            if data[0] & IOB_VERSION_MASK == IOB_VERSION_MASK {
                self.presence = Presence::Present;
                tracing::info!(
                    "HL2IOBoard: found on the accessory bus (hardware version {}). Sending it \
                     the transmit frequency so it can switch bands for an amplifier, transverter \
                     or loop antenna.",
                    data[0] & !IOB_VERSION_MASK
                );
                // Its registers are static and may hold whatever the last
                // program left; its documentation asks for this at startup.
                self.queue.push_back(Op::write(REG_CONTROL, 1));
                // The reset just wrote 0 there, so only a non-default choice
                // needs saying.
                if self.rx_input.code() != 0 {
                    self.queue.push_back(Op::write(REG_RF_INPUTS, self.rx_input.code()));
                    tracing::info!(
                        "HL2IOBoard: receive input -> {} (mode {})",
                        self.rx_input.label(),
                        self.rx_input.code()
                    );
                }
                // The reset just zeroed every register, so say both frequencies
                // straight away rather than waiting for the operator to move
                // the dial.
                self.sent_hz = None;
                self.sent_rx_code = None;
                self.sent_at = None;
            } else {
                self.probe_failed(now, &format!("unexpected version byte 0x{:02X}", data[0]));
            }
        }

        // Count acknowledged initial frequency writes.
        if self.tuner_init == TunerInit::WaitingForFrequencies
            && self.startup_writes_pending > 0
            && f.op.cookie == I2C_WRITE
            && ((0..=REG_TX_FREQ_BYTE0).contains(&f.op.reg)
                || f.op.reg == REG_FCODE_RX1)
        {
            self.startup_writes_pending -= 1;

            if self.startup_writes_pending == 0 {
                self.tuner_init = TunerInit::DiscoveryRequested;
                tracing::debug!(
                    "HL2IOBoard: initial frequency writes complete; starting AH-4 KEY discovery"
                );
                self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x01));
            }
        }

        // Advance the tuner startup sequence only after its writes are acknowledged.
        if f.op.cookie == I2C_WRITE && f.op.reg == REG_ANTENNA_TUNER {
            match (self.tuner_init, f.op.data) {
                (TunerInit::DiscoveryRequested, 0x01) => {
                    self.tuner_init = TunerInit::Discovering;
                    self.tuner_verify_started_at = Some(now);
                    self.tuner_poll_at = Some(now);
                    self.queue.push_back(Op::read(REG_ANTENNA_TUNER));
                    tracing::debug!(
                        "HL2IOBoard: discovery command acknowledged; waiting for KEY handshake"
                    );
                }
                (TunerInit::DiscoveryBypassRequested, 0x02) => {
                    self.tuner_init = TunerInit::VerifyingBypass;
                    self.tuner_verify_started_at = Some(now);
                    self.tuner_poll_at = Some(now);
                    self.queue.push_back(Op::read(REG_ANTENNA_TUNER));
                    tracing::debug!(
                        "HL2IOBoard: discovery outcome latched; bypass requested and status verification started"
                    );
                }
                (TunerInit::Monitoring, 0x02) => {
                    self.tuner_poll_at = Some(now);
                    self.queue
                        .push_back(Op::read(REG_ANTENNA_TUNER));
                    tracing::info!(
                        "HL2IOBoard: normal bypass acknowledged; checking tuner status"
                    );
                }
                (TunerInit::RecoveryBypassRequested, 0x02) => {
                    self.tuner_init = TunerInit::RecoveringBypass;
                    self.tuner_recovery_started_at = Some(now);
                    self.tuner_poll_at = Some(now);
                    self.queue
                        .push_back(Op::read(REG_ANTENNA_TUNER));
                    tracing::info!(
                        "HL2IOBoard: recovery bypass acknowledged; checking tuner status"
                    );
                }
                _ => {}
            }
        }

        // Save reads of the AH-4 tuner status register and interpret the
        // result of the initial bypass test.
        if f.op.cookie == I2C_READ {
            tracing::info!(
                "HL2IOBoard: I2C read register {} ACK bytes C1..C4 = {:02X} {:02X} {:02X} {:02X}",
                f.op.reg,
                data[0],
                data[1],
                data[2],
                data[3]
            );
        }

        if f.op.cookie == I2C_READ && f.op.reg == REG_ANTENNA_TUNER {
            tracing::info!(
                "HL2IOBoard: tuner read ACK bytes C1..C4 = {:02X} {:02X} {:02X} {:02X}",
                data[0],
                data[1],
                data[2],
                data[3]
            );
            let status = data[3];
            let previous_status = self.tuner_status;
            self.tuner_status = Some(status);
            self.tuner_status_at = Some(now);

            if matches!(self.tuner_init, TunerInit::VerifyingBypass)
                || previous_status != Some(status)
            {
                tracing::info!(
                    "HL2IOBoard: tuner status register read: 0x{:02X}",
                    status
                );
            }

            match self.tuner_init {
                TunerInit::Discovering => {
                    match status {
                        0xFC => {
                            // Only the terminal KEY-handshake result confirms presence.
                            // Startup discovery must never request RF.
                            self.tuner_present = true;
                            self.tuner_init = TunerInit::DiscoveryBypassRequested;
                            self.tuner_verify_started_at = None;
                            self.queue.clear();
                            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
                            tracing::info!(
                                "HL2IOBoard: AH-4 discovery completed (0xFC); requesting bypass without RF"
                            );
                        }
                        0xFB => {
                            self.tuner_present = false;
                            self.tuner_init = TunerInit::DiscoveryBypassRequested;
                            self.tuner_verify_started_at = None;
                            self.queue.clear();
                            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
                            tracing::info!(
                                "HL2IOBoard: AH-4 discovery completed (0xFB); tuner absent; requesting bypass"
                            );
                        }
                        _ => {
                            // 0x04, 0xEE and 0x07 are intermediate, not terminal.
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                        }
                    }
                }
                TunerInit::VerifyingBypass => {
                    match status {
                        0x00 => {
                            self.tuner_init = TunerInit::Monitoring;
                            self.tuner_verify_started_at = None;
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                            tracing::info!(
                                "Post-discovery bypass status is idle; discovery result retained"
                            );
                        }
                        0xFC => {
                            self.tuner_init = TunerInit::Monitoring;
                            self.tuner_verify_started_at = None;
                            self.tuner_fault = self.tuner_present;
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                            tracing::warn!(
                                "Post-discovery bypass status reports failure; discovery result retained"
                            );
                        }
                        _ => {
                            // The write ACK is not proof of bypass. Wait for a
                            // definitive status, without inferring presence.
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                        }
                    }
                }
                TunerInit::Monitoring => {
                    // A timed-out bypass may still report its final status.
                    // Only a confirmed idle status clears the latched fault.
                    if self.tuner_recovery_timed_out {
                        match status {
                            0x00 => {
                                self.tuner_recovery_timed_out = false;
                                self.tuner_fault = false;
                                tracing::info!(
                                    "HL2IOBoard: late AH-4 bypass recovery succeeded; fault cleared"
                                );
                            }
                            0xF0..=0xFF => {
                                self.tuner_recovery_timed_out = false;
                                self.tuner_fault = true;
                                tracing::warn!(
                                    "HL2IOBoard: late AH-4 recovery returned terminal status 0x{:02X}; fault remains latched",
                                    status
                                );
                            }
                            _ => {}
                        }
                    }

                    // Presence is latched by the startup KEY handshake only.
                    // Later status reads cannot turn an absent tuner into present.
                    if self.tuner_start_pending {
                        match status {
                            0x00 if self.tuner_start_seen_active => {
                                self.tuner_start_pending = false;
                                self.tuner_start_seen_active = false;
                                self.tuner_start_started_at = None;
                                tracing::info!(
                                    "HL2IOBoard: AH-4 tuning cycle completed"
                                );
                            }
                            0x00 => {
                                // The tuner may not have started yet.
                            }
                            0xEE => {
                                self.tuner_start_seen_active = true;
                                tracing::info!(
                                    "HL2IOBoard: AH-4 requests tuning RF"
                                );
                            }
                            0xFC | 0xFD => {
                                self.tuner_start_pending = false;
                                self.tuner_start_seen_active = false;
                                self.tuner_start_started_at = None;
                                self.tuner_fault = true;
                                tracing::warn!(
                                    "HL2IOBoard: AH-4 tuning failed or reset; status 0x{:02X}",
                                    status
                                );
                            }
                            0xF0..=0xFF => {
                                self.tuner_start_pending = false;
                                self.tuner_start_seen_active = false;
                                self.tuner_start_started_at = None;
                                self.tuner_fault = true;
                                tracing::warn!(
                                    "HL2IOBoard: AH-4 returned an unrecognised terminal status 0x{:02X}",
                                    status
                                );
                            }
                            _ => {
                                // Intermediate tuner status.
                                self.tuner_start_seen_active = true;
                            }
                        }
                    }

                    self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                }
                TunerInit::RecoveringBypass => {
                    match status {
                        0x00 => {
                            self.tuner_present = true;
                            self.tuner_fault = false;
                            self.tuner_recovery_timed_out = false;
                            self.tuner_init = TunerInit::Monitoring;
                            self.tuner_recovery_started_at = None;
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                            tracing::info!(
                                "HL2IOBoard: AH-4 bypass recovery succeeded; fault cleared"
                            );
                        }
                        0xF0..=0xFF => {
                            self.tuner_init = TunerInit::Monitoring;
                            self.tuner_recovery_started_at = None;
                            self.tuner_recovery_timed_out = false;
                            self.tuner_fault = true;
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                            tracing::warn!(
                                "HL2IOBoard: AH-4 bypass recovery failed; status 0x{:02X}; fault remains latched",
                                status
                            );
                        }
                        _ => {
                            // Keep waiting for a definitive recovery result.
                            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
                        }
                    }
                }
                _ => {}
            }

        }
    }

    /// A transaction that will not complete.
    fn give_up(&mut self, op: Op, now: Instant, why: &str) {
        if let Presence::Probing(_) = self.presence {
            return self.probe_failed(now, why);
        }
        // Abandon the rest of the sequence with it. A frequency the board only
        // half received is not a frequency — and the byte that went missing may
        // have been the one that latches — so forget that it was ever sent and
        // say the whole five bytes again from the top.
        self.queue.clear();
        if op.cookie == I2C_WRITE
            && op.reg == REG_ANTENNA_TUNER
            && op.data == 0x01
        {
            self.tuner_start_pending = false;
            self.tuner_start_seen_active = false;
            self.tuner_start_started_at = None;
            self.tuner_fault = true;
            tracing::warn!("HL2IOBoard: AH-4 start request failed at the I2C layer");
        }
        if self.tuner_init == TunerInit::DiscoveryBypassRequested
            && !(op.cookie == I2C_WRITE
                && op.reg == REG_ANTENNA_TUNER
                && op.data == 0x02)
        {
            // An explicit I2C error cleared the queued bypass after discovery
            // timed out. Restore the bypass request; bus quarantine still
            // prevents sending it if the transaction outcome is uncertain.
            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
            tracing::warn!(
                "HL2IOBoard: I2C error during timed-out discovery; restoring the bypass request"
            );
        } else if self.tuner_init == TunerInit::DiscoveryRequested
            && op.cookie == I2C_WRITE
            && op.reg == REG_ANTENNA_TUNER
            && op.data == 0x01
        {
            self.tuner_present = false;
            self.tuner_init = TunerInit::DiscoveryBypassRequested;
            self.tuner_verify_started_at = None;
            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
            tracing::warn!("HL2IOBoard: discovery command failed at I2C layer; tuner absent; requesting bypass");
        } else if self.tuner_init == TunerInit::Discovering
            && op.cookie == I2C_READ
            && op.reg == REG_ANTENNA_TUNER
        {
            self.tuner_present = false;
            self.tuner_init = TunerInit::DiscoveryBypassRequested;
            self.tuner_verify_started_at = None;
            self.queue.push_back(Op::write(REG_ANTENNA_TUNER, 0x02));
            tracing::warn!("HL2IOBoard: discovery status read failed; tuner absent; requesting bypass");
        } else if self.tuner_init == TunerInit::DiscoveryBypassRequested
            && op.cookie == I2C_WRITE
            && op.reg == REG_ANTENNA_TUNER
            && op.data == 0x02
        {
            // A failed bypass write must not leave startup stuck in this state.
            // Preserve the discovery result, but invalidate status and block
            // tuning if a tuner was found and its bypass state is uncertain.
            self.tuner_status = None;
            self.tuner_status_at = None;
            self.tuner_init = TunerInit::Monitoring;
            self.tuner_verify_started_at = None;
            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
            self.tuner_fault = self.tuner_present;
            tracing::warn!(
                "HL2IOBoard: post-discovery bypass write failed at the I2C layer; \
                 discovery result retained and tuning remains blocked as needed"
            );
        }

        if self.tuner_init == TunerInit::RecoveryBypassRequested
            && op.cookie == I2C_WRITE
            && op.reg == REG_ANTENNA_TUNER
            && op.data == 0x02
        {
            self.tuner_init = TunerInit::Monitoring;
            self.tuner_recovery_started_at = None;
            self.tuner_fault = true;
            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
            tracing::warn!(
                "HL2IOBoard: AH-4 recovery bypass write failed; fault remains latched"
            );
        } else if self.tuner_init == TunerInit::RecoveringBypass
            && op.cookie == I2C_READ
            && op.reg == REG_ANTENNA_TUNER
        {
            self.tuner_init = TunerInit::Monitoring;
            self.tuner_recovery_started_at = None;
            self.tuner_fault = true;
            self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
            tracing::warn!(
                "HL2IOBoard: AH-4 recovery status read failed; fault remains latched"
            );
        }
        self.sent_hz = None;
        self.sent_rx_code = None;
        self.sent_at = None;
        let operation = match op.cookie {
            I2C_READ => "read",
            I2C_WRITE => "write",
            _ => "unknown operation",
        };

        tracing::warn!(
            "HL2IOBoard: I2C {operation} of register {} failed ({why}); sending the transmit frequency again",
            op.reg
        );
    }

    fn probe_failed(&mut self, now: Instant, why: &str) {
        let Presence::Probing(left) = self.presence else { return };
        self.queue.clear();
        let left = left.saturating_sub(1);
        self.probe_at = Some(now + PROBE_INTERVAL);
        if left == 0 {
            self.presence = Presence::Absent;
            tracing::debug!(
                "HL2IOBoard: none found on the accessory bus ({why}); not driving it. This is \
                 normal on a radio without one."
            );
        } else {
            self.presence = Presence::Probing(left);
        }
    }
}

/// The board's one-byte logarithmic frequency code, as Quisk's `hertz2code`
/// computes it: `round(15.47 · ln(hz / 18748.1))`, clamped to `1..=255`, with
/// `0` reserved for "not specified".
///
/// Each step is 6.68 %, so an entire amateur band lands on one or two codes —
/// which is the point. The receive code is what the board switches an antenna
/// and preselector on, and deduplicating on the *code* rather than the frequency
/// means a spun dial puts nothing at all on the I2C bus while a band change
/// always does.
fn hertz_to_code(hz: u64) -> u8 {
    if hz == 0 {
        return 0;
    }
    let code = (15.47 * (hz as f64 / 18748.1).ln()).round();
    // Below ~18.7 kHz the logarithm goes negative; the floor is 1, since 0 has
    // its own meaning.
    code.clamp(1.0, 255.0) as u8
}

/// The five register writes that hand `hz` to the board, **BYTE0 last** — that
/// write is what makes the board act on the frequency, so the other four have to
/// already be in place.
fn freq_writes(hz: u64) -> [Op; 5] {
    // A five-byte big-endian field; the top byte is there for transverted
    // frequencies above 4.29 GHz.
    let be = hz.to_be_bytes();
    [
        Op::write(REG_TX_FREQ_BYTE4, be[3]),
        Op::write(REG_TX_FREQ_BYTE4 + 1, be[4]),
        Op::write(REG_TX_FREQ_BYTE4 + 2, be[5]),
        Op::write(REG_TX_FREQ_BYTE4 + 3, be[6]),
        Op::write(REG_TX_FREQ_BYTE0, be[7]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuner_presence_is_unknown_until_discovery_has_a_result() {
        let mut board = IoBoard::new(sdroxide_types::HpsdrIoRxInput::Radio);
        assert_eq!(board.tuner_presence(), None);

        board.tuner_init = TunerInit::DiscoveryBypassRequested;
        board.tuner_present = false;
        assert_eq!(board.tuner_presence(), Some(false));

        board.tuner_present = true;
        assert_eq!(board.tuner_presence(), Some(true));
    }

    /// A receive frequency for the tests that are not about the receive code.
    /// Held fixed so it contributes exactly one write after the reset.
    const RX_HZ: u64 = 14_074_000;

    /// Answer the request `board` just made, as the gateware would.
    fn ack(board: &mut IoBoard, now: Instant, data: [u8; 4]) {
        board.on_ack(CC_I2C2 >> 1, data, now);
    }

    /// Take the board from cold to found, returning the probe it sent.
    fn find(board: &mut IoBoard, now: Instant) -> [u8; 5] {
        let probe =
            board.next_request(now, 14_074_000, RX_HZ, 0).expect("probes on the first poll");
        ack(board, now, [0xF1, 0, 0, 0]);
        probe
    }

    fn complete_tuner_priming(board: &mut IoBoard, now: Instant) -> Instant {
        let mut now = now;

        let cc = board.next_request(now, RX_HZ, RX_HZ, 0).expect("intermediate status");
        assert_eq!((cc[3], cc[4]), (REG_ANTENNA_TUNER, 0x01));
        ack(board, now, [0; 4]);
        now += TUNER_POLL_INTERVAL;

        for intermediate in [0x04, 0xEE, 0x07] {
            let cc = board.next_request(now, RX_HZ, RX_HZ, 0).expect("intermediate status");
            assert_eq!((cc[1], cc[3]), (I2C_READ, REG_ANTENNA_TUNER));
            ack(board, now, [0, 0, 0, intermediate]);
            assert!(!board.tuner_present, "intermediate status must not confirm presence");
            assert!(!board.tuner_start_pending, "discovery must not request RF");
            now += TUNER_POLL_INTERVAL;
        }

        let cc = board.next_request(now, RX_HZ, RX_HZ, 0).expect("terminal discovery status");
        assert_eq!((cc[1], cc[3]), (I2C_READ, REG_ANTENNA_TUNER));
        ack(board, now, [0, 0, 0, 0xFC]);
        assert!(board.tuner_present);
        assert!(!board.tuner_start_pending, "discovery must not request RF");

        let cc = board.next_request(now, RX_HZ, RX_HZ, 0).expect("bypass request");
        assert_eq!((cc[3], cc[4]), (REG_ANTENNA_TUNER, 0x02));
        ack(board, now, [0; 4]);

        let cc = board.next_request(now, RX_HZ, RX_HZ, 0).expect("bypass status");
        assert_eq!((cc[1], cc[3]), (I2C_READ, REG_ANTENNA_TUNER));
        ack(board, now, [0, 0, 0, 0x00]);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert!(board.tuner_present, "bypass status must preserve discovery result");
        now
    }

    /// A present, idle board with a latched tuner fault and an empty bus.
    fn faulted_idle_board() -> IoBoard {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        board.presence = Presence::Present;
        board.tuner_init = TunerInit::Monitoring;
        board.tuner_present = true;
        board.tuner_status = Some(0x00);
        board.tuner_status_at = Some(Instant::now());
        board.tuner_fault = true;
        board
    }

    #[test]
    fn tuner_start_ack_timeout_quarantines_the_i2c_bus() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_fault = false;

        assert!(board.tuner_start());

        let first = board
            .next_request(now, RX_HZ, RX_HZ, 0)
            .expect("initial tuner start");
        assert_eq!((first[3], first[4]), (REG_ANTENNA_TUNER, 0x01));

        // The uncertain start ACK timeout must latch both fault and quarantine.
        assert!(
            board.next_request(now + ACK_TIMEOUT, RX_HZ, RX_HZ, 0).is_none(),
            "no request may be sent after the uncertain start ACK"
        );
        assert!(board.tuner_fault, "uncertain outcome must latch a fault");
        assert!(board.i2c_quarantined, "uncertain ACK must quarantine the bus");
        assert!(!board.tuner_start_pending);
        assert!(board.inflight.is_none(), "timed-out start must leave in-flight state");

        // Even if work appears in the queue later, quarantine blocks transmission.
        board.queue.push_back(Op::write(REG_CONTROL, 1));
        assert!(
            board.next_request(now + ACK_TIMEOUT + Duration::from_millis(1), RX_HZ, RX_HZ, 0).is_none(),
            "quarantine must block subsequent I2C requests"
        );
        assert_eq!(board.queue.len(), 1, "queued work must remain unsent");
    }

    #[test]
    fn late_ack_is_ignored_while_i2c_bus_is_quarantined() {
        let mut board = faulted_idle_board();
        let now = Instant::now();

        board.i2c_quarantined = true;
        board.queue.push_back(Op::write(REG_CONTROL, 1));

        // A delayed ACK must not alter the quarantined board state.
        ack(&mut board, now, [0; 4]);

        assert!(board.i2c_quarantined, "late ACK must not clear quarantine");
        assert!(board.inflight.is_none());
        assert_eq!(board.queue.len(), 1, "queued work must remain untouched");
        assert!(
            board.next_request(now + Duration::from_millis(1), RX_HZ, RX_HZ, 0).is_none(),
            "no request may be sent after a late ACK"
        );
        assert_eq!(board.queue.len(), 1, "queued work must remain unsent");
        assert!(!board.tuner_start(), "tuner start must remain blocked");
        assert!(!board.tuner_recover(), "tuner recovery must remain blocked");
    }

    #[test]
    fn tuner_start_rejects_stale_idle_status() {
        let mut board = faulted_idle_board();
        board.tuner_fault = false;
        board.tuner_status_at = Some(
            Instant::now() - TUNER_STATUS_MAX_AGE - Duration::from_millis(1)
        );

        assert!(
            !board.tuner_start(),
            "an old idle status must not authorise tuning"
        );

        board.tuner_status_at = Some(Instant::now());
        assert!(
            board.tuner_start(),
            "a fresh idle status may authorise tuning"
        );
    }

    #[test]
    fn tuner_status_read_error_invalidates_previous_status() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_fault = false;
        board.inflight = Some(InFlight {
            op: Op::read(REG_ANTENNA_TUNER),
            sent_at: now,
        });

        board.on_ack(RADDR_ERROR, [0; 4], now);

        assert_eq!(board.tuner_status, None);
        assert_eq!(board.tuner_status_at, None);
        assert!(
            !board.tuner_start(),
            "a failed status read must not authorise tuning"
        );
    }

    #[test]
    fn tuner_start_requires_a_present_ready_idle_board_and_empty_bus() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_fault = false;

        // A board that is not currently present must reject the request.
        board.presence = Presence::Probing(PROBE_TRIES);
        assert!(!board.tuner_start());

        // An outstanding transaction must also block the request.
        board.presence = Presence::Present;
        board.inflight = Some(InFlight {
            op: Op::read(REG_ANTENNA_TUNER),
            sent_at: now,
        });
        assert!(!board.tuner_start());

        // A queued operation must block the request as well.
        board.inflight = None;
        board.queue.push_back(Op::write(REG_CONTROL, 1));
        assert!(!board.tuner_start());

        // Once the bus is idle, the verified tuner may accept a start.
        board.queue.clear();
        assert!(board.tuner_start());
        assert!(board.tuner_start_pending);
        assert_eq!(
            board.queue.front().copied(),
            Some(Op::write(REG_ANTENNA_TUNER, 0x01))
        );
    }

    #[test]
    fn tuner_bypass_requires_fresh_verified_idle_state() {
        let mut board = faulted_idle_board();
        board.tuner_fault = false;

        assert!(board.tuner_bypass());
        assert_eq!(
            board.queue.front().copied(),
            Some(Op::write(REG_ANTENNA_TUNER, 0x02))
        );
        assert!(!board.tuner_start_pending);
    }

    #[test]
    fn tuner_bypass_ack_queues_status_read() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_fault = false;

        assert!(board.tuner_bypass());

        let write = board
            .next_request(now, RX_HZ, RX_HZ, 0)
            .expect("normal bypass write");
        assert_eq!(
            (write[3], write[4]),
            (REG_ANTENNA_TUNER, 0x02)
        );

        ack(&mut board, now, [0; 4]);

        let read = board
            .next_request(now, RX_HZ, RX_HZ, 0)
            .expect("status read after bypass acknowledgement");
        assert_eq!(read[1], I2C_READ);
        assert_eq!(read[2], IOB_ADDR);
        assert_eq!(read[3], REG_ANTENNA_TUNER);
    }

    #[test]
    fn tuner_bypass_rejects_unsafe_states() {
        let mut board = faulted_idle_board();
        board.tuner_fault = false;

        // A stale status must not authorise bypass.
        board.tuner_status_at = Some(
            Instant::now() - TUNER_STATUS_MAX_AGE - Duration::from_millis(1)
        );
        assert!(!board.tuner_bypass());
        board.tuner_status_at = Some(Instant::now());

        // A latched tuner fault must not be overridden by normal bypass.
        board.tuner_fault = true;
        assert!(!board.tuner_bypass());
        board.tuner_fault = false;

        // The board must be present and the tuner must be detected.
        board.presence = Presence::Probing(PROBE_TRIES);
        assert!(!board.tuner_bypass());
        board.presence = Presence::Present;

        board.tuner_present = false;
        assert!(!board.tuner_bypass());
        board.tuner_present = true;

        // An in-flight or queued operation must block bypass.
        board.inflight = Some(InFlight {
            op: Op::read(REG_ANTENNA_TUNER),
            sent_at: Instant::now(),
        });
        assert!(!board.tuner_bypass());
        board.inflight = None;

        board.queue.push_back(Op::write(REG_CONTROL, 1));
        assert!(!board.tuner_bypass());
        board.queue.clear();

        // Quarantine and an active tuner-start request must also block it.
        board.i2c_quarantined = true;
        assert!(!board.tuner_bypass());
        board.i2c_quarantined = false;

        board.tuner_start_pending = true;
        assert!(!board.tuner_bypass());
    }

    #[test]
    fn tuner_recovery_requires_a_latched_fault_and_an_idle_bus() {
        let mut board = faulted_idle_board();
        board.tuner_fault = false;
        assert!(!board.tuner_recover(), "no fault to recover");

        board.tuner_fault = true;
        board.inflight = Some(InFlight {
            op: Op::read(REG_ANTENNA_TUNER),
            sent_at: Instant::now(),
        });
        assert!(!board.tuner_recover(), "transaction in flight");

        board.inflight = None;
        board.queue.push_back(Op::write(REG_CONTROL, 1));
        assert!(!board.tuner_recover(), "queued operation");
    }

    #[test]
    fn tuner_recovery_clears_fault_only_after_idle_status_is_confirmed() {
        let mut board = faulted_idle_board();
        let now = Instant::now();

        assert!(board.tuner_recover());
        assert!(board.tuner_fault, "request alone must not clear the fault");

        let write = board.next_request(now, RX_HZ, RX_HZ, 0)
            .expect("recovery bypass write");
        assert_eq!(write[1], I2C_WRITE);
        assert_eq!((write[3], write[4]), (REG_ANTENNA_TUNER, 0x02));
        ack(&mut board, now, [0; 4]);

        assert_eq!(board.tuner_init, TunerInit::RecoveringBypass);
        assert!(board.tuner_fault);

        let read = board.next_request(now, RX_HZ, RX_HZ, 0)
            .expect("recovery status read");
        assert_eq!(read[1], I2C_READ);
        assert_eq!(read[3], REG_ANTENNA_TUNER);
        ack(&mut board, now, [0, 0, 0, 0x00]);

        assert!(!board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_status, Some(0x00));
    }

    #[test]
    fn tuner_recovery_terminal_failure_keeps_fault_latched() {
        let mut board = faulted_idle_board();
        let now = Instant::now();

        assert!(board.tuner_recover());
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("bypass write");
        ack(&mut board, now, [0; 4]);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("status read");
        ack(&mut board, now, [0, 0, 0, 0xFC]);

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
    }

    #[test]
    fn tuner_recovery_timeout_keeps_fault_latched() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_init = TunerInit::RecoveringBypass;
        board.tuner_recovery_started_at = Some(now);

        // Isolate the recovery deadline from unrelated I2C polling.
        board.tuner_poll_at = None;
        board.inflight = None;
        board.queue.clear();

        // Recovery must still be pending at the old two-second deadline.
        let _ = board.next_request(
            now + Duration::from_secs(2),
            RX_HZ,
            RX_HZ,
            0,
        );

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::RecoveringBypass);
        assert_eq!(board.tuner_recovery_started_at, Some(now));

        // It should time out at the new four-second deadline.
        let _ = board.next_request(
            now + TUNER_RECOVERY_TIMEOUT,
            RX_HZ,
            RX_HZ,
            0,
        );

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_recovery_started_at, None);
        assert!(board.tuner_recovery_timed_out);
    }

    #[test]
    fn late_idle_status_clears_fault_after_recovery_timeout() {
        let mut board = faulted_idle_board();
        let start = Instant::now();
        board.tuner_init = TunerInit::RecoveringBypass;
        board.tuner_recovery_started_at = Some(start);
        board.tuner_poll_at = None;

        let timeout_at = start + TUNER_RECOVERY_TIMEOUT;
        let _ = board.next_request(timeout_at, RX_HZ, RX_HZ, 0);

        assert!(board.tuner_fault);
        assert!(board.tuner_recovery_timed_out);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);

        // Simulate the next status poll after the timeout.
        board.inflight = None;
        board.queue.clear();
        board.queue.push_back(Op::read(REG_ANTENNA_TUNER));
        let read_at = timeout_at + TUNER_POLL_INTERVAL;
        board.next_request(read_at, RX_HZ, RX_HZ, 0)
            .expect("post-timeout status read");
        ack(&mut board, read_at, [0, 0, 0, 0x00]);

        assert!(!board.tuner_fault);
        assert!(!board.tuner_recovery_timed_out);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_status, Some(0x00));
    }

    #[test]
    fn late_recovery_failure_stays_latched_on_later_idle_status() {
        let mut board = faulted_idle_board();
        let start = Instant::now();
        board.tuner_init = TunerInit::RecoveringBypass;
        board.tuner_recovery_started_at = Some(start);
        board.tuner_poll_at = None;

        let timeout_at = start + TUNER_RECOVERY_TIMEOUT;
        let _ = board.next_request(timeout_at, RX_HZ, RX_HZ, 0);

        assert!(board.tuner_fault);
        assert!(board.tuner_recovery_timed_out);

        // A late terminal failure ends recovery but leaves the fault latched.
        let failure_at = timeout_at + TUNER_POLL_INTERVAL;
        board.inflight = None;
        board.queue.clear();
        board.queue.push_back(Op::read(REG_ANTENNA_TUNER));
        board.next_request(failure_at, RX_HZ, RX_HZ, 0)
            .expect("status read after timeout");
        ack(&mut board, failure_at, [0, 0, 0, 0xFC]);

        assert!(board.tuner_fault);
        assert!(!board.tuner_recovery_timed_out);

        // An ordinary later idle status must not clear that fault.
        let idle_at = failure_at + TUNER_POLL_INTERVAL;
        board.inflight = None;
        board.queue.clear();
        board.queue.push_back(Op::read(REG_ANTENNA_TUNER));
        board.next_request(idle_at, RX_HZ, RX_HZ, 0)
            .expect("subsequent status read");
        ack(&mut board, idle_at, [0, 0, 0, 0x00]);

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_status, Some(0x00));
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
    }

    #[test]
    fn tuner_recovery_bypass_write_failure_keeps_fault_latched() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_init = TunerInit::RecoveryBypassRequested;

        board.give_up(
            Op::write(REG_ANTENNA_TUNER, 0x02),
            now,
            "test failure",
        );

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_recovery_started_at, None);
    }

    #[test]
    fn tuner_recovery_status_read_failure_keeps_fault_latched() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_init = TunerInit::RecoveringBypass;
        board.tuner_recovery_started_at = Some(now);

        board.give_up(
            Op::read(REG_ANTENNA_TUNER),
            now,
            "test failure",
        );

        assert!(board.tuner_fault);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_recovery_started_at, None);
    }


    #[test]
    fn discovery_bypass_i2c_error_retains_presence_and_latches_fault() {
        let mut board = faulted_idle_board();
        let now = Instant::now();
        board.tuner_fault = false;
        board.tuner_init = TunerInit::DiscoveryBypassRequested;
        board.tuner_present = true;
        board.tuner_status = Some(0xEE);
        board.tuner_status_at = Some(now);
        board.inflight = Some(InFlight {
            op: Op::write(REG_ANTENNA_TUNER, 0x02),
            sent_at: now,
        });

        // Simulate the radio reporting that the Pico did not ACK the bypass write.
        board.on_ack(RADDR_ERROR, [0; 4], now);

        assert!(board.tuner_present, "bypass failure must not erase discovery");
        assert!(board.tuner_fault, "uncertain bypass state must block tuning");
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert_eq!(board.tuner_status, None, "pre-failure status must be invalidated");
        assert_eq!(board.tuner_status_at, None);
        assert_eq!(board.tuner_verify_started_at, None);
        assert!(board.inflight.is_none());
        assert!(!board.tuner_start(), "faulted tuner must not start");
    }

    #[test]
    fn probe_reads_the_version_register_and_then_resets_the_board() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();
        let probe = find(&mut board, now);
        assert_eq!(probe[0], CC_I2C2 | CC_RQST, "I2C bus 2, answer requested");
        assert_eq!(probe[1], I2C_READ);
        assert_eq!(probe[2], IOB_VERSION_ADDR);
        assert_eq!(board.presence, Presence::Present);

        // The first thing said to a board that has just been found is "clear
        // everything" — its registers survive whatever program ran last.
        let reset = board.next_request(now, 14_074_000, RX_HZ, 0).expect("resets the board");
        assert_eq!(reset[1], I2C_WRITE);
        assert_eq!(reset[2], IOB_ADDR);
        assert_eq!(reset[3], REG_CONTROL);
        assert_eq!(reset[4], 1);
    }

    #[test]
    fn tuner_status_read_is_recorded_exactly() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        find(&mut board, now);

        // Complete the startup reset and frequency writes.
        for _ in 0..6 {
            let request = board
                .next_request(now, RX_HZ, RX_HZ, 0)
                .expect("startup request");
            ack(&mut board, now, [0; 4]);

            if request[3] == REG_FCODE_RX1 {
                break;
            }
        }

        // Simulate a successful read of the tuner status register.
        board.inflight = Some(InFlight {
            op: Op::read(REG_ANTENNA_TUNER),
            sent_at: now,
        });
        ack(&mut board, now, [0, 0, 0, 0xFD]);

        assert_eq!(board.tuner_status, Some(0xFD));
        assert_eq!(board.tuner_status_at, Some(now));
        assert_eq!(board.tuner_status_snapshot(), Some((0xFD, now)));
    }

    #[test]
    fn tuner_initialisation_waits_for_startup_frequency_writes() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();

        find(&mut board, now);

        // Complete the reset and initial frequency writes.
        for _ in 0..7 {
            let request = board
                .next_request(now, RX_HZ, RX_HZ, 0)
                .expect("startup request");
            ack(&mut board, now, [0; 4]);
            now += Duration::from_millis(3);

            if request[3] == REG_FCODE_RX1 {
                break;
            }
        }

        // Discovery starts directly with 0x01 after the initial frequency writes.
        let request = board
            .next_request(now, RX_HZ, RX_HZ, 0)
            .expect("AH-4 discovery request");
        assert_eq!((request[3], request[4]), (REG_ANTENNA_TUNER, 0x01));
        assert_eq!(board.tuner_init, TunerInit::DiscoveryRequested);

        ack(&mut board, now, [0; 4]);
        now += Duration::from_millis(3);

        assert_eq!(board.tuner_init, TunerInit::Discovering);
        assert!(!board.tuner_present, "presence is not assumed before terminal discovery status");
    }

    #[test]
    fn ah4_discovery_fb_latches_absence_even_if_bypass_status_is_idle() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);

        // Reset + five transmit-frequency bytes + receive-frequency code.
        for _ in 0..7 {
            board.next_request(now, RX_HZ, RX_HZ, 0).expect("startup write");
            ack(&mut board, now, [0; 4]);
            now += Duration::from_millis(3);
            now += TUNER_POLL_INTERVAL;
        }

        let discover = board.next_request(now, RX_HZ, RX_HZ, 0).expect("discovery");
        assert_eq!((discover[3], discover[4]), (REG_ANTENNA_TUNER, 0x01));
        ack(&mut board, now, [0; 4]);
        now += TUNER_POLL_INTERVAL;

        for intermediate in [0x04, 0xEE, 0x07] {
            let read = board.next_request(now, RX_HZ, RX_HZ, 0)
                .expect("intermediate status");
            assert_eq!((read[1], read[3]), (I2C_READ, REG_ANTENNA_TUNER));
            ack(&mut board, now, [0, 0, 0, intermediate]);
            assert!(!board.tuner_present);
            assert_eq!(board.tuner_init, TunerInit::Discovering);
            now += TUNER_POLL_INTERVAL;
        }

        let read = board.next_request(now, RX_HZ, RX_HZ, 0).expect("terminal discovery status");
        assert_eq!((read[1], read[3]), (I2C_READ, REG_ANTENNA_TUNER));
        ack(&mut board, now, [0, 0, 0, 0xFB]);
        assert!(!board.tuner_present);
        assert_eq!(board.tuner_init, TunerInit::DiscoveryBypassRequested);
        assert!(!board.tuner_start_pending);

        let bypass = board.next_request(now, RX_HZ, RX_HZ, 0).expect("final bypass");
        assert_eq!((bypass[3], bypass[4]), (REG_ANTENNA_TUNER, 0x02));
        ack(&mut board, now, [0; 4]);

        let status = board.next_request(now, RX_HZ, RX_HZ, 0).expect("bypass verification");
        assert_eq!((status[1], status[3]), (I2C_READ, REG_ANTENNA_TUNER));
        ack(&mut board, now, [0, 0, 0, 0x00]);
        assert_eq!(board.tuner_init, TunerInit::Monitoring);
        assert!(!board.tuner_present, "idle bypass status must not overwrite 0xFB");
        assert!(!board.tuner_start(), "an absent tuner must not start");
    }

    #[test]
    fn ah4_discovery_host_timeout_latches_absence_and_requests_bypass() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();
        board.presence = Presence::Present;
        board.tuner_init = TunerInit::Discovering;
        board.tuner_verify_started_at = Some(now);
        board.tuner_poll_at = None;

        let bypass = board
            .next_request(now + TUNER_INIT_TIMEOUT, RX_HZ, RX_HZ, 0)
            .expect("bypass after host discovery timeout");
        assert_eq!((bypass[3], bypass[4]), (REG_ANTENNA_TUNER, 0x02));
        assert!(!board.tuner_present);
        assert_eq!(board.tuner_init, TunerInit::DiscoveryBypassRequested);
    }

    /// `REG_RF_INPUTS` is the operator's answer to "what is wired to the board's
    /// own SMA jacks", and only they know it. The reset writes 0, so only a
    /// non-default choice costs a transaction.
    #[test]
    fn the_receive_input_mode_is_stated_only_when_it_is_not_the_default() {
        let now = Instant::now();
        for (mode, code) in [(HpsdrIoRxInput::IoBoard, 1), (HpsdrIoRxInput::IoBoardPureSignal, 2)] {
            let mut board = IoBoard::new(mode);
            find(&mut board, now);
            let reset = board.next_request(now, RX_HZ, RX_HZ, 0).expect("the reset");
            ack(&mut board, now, [0; 4]);
            assert_eq!((reset[3], reset[4]), (REG_CONTROL, 1));
            let sel = board.next_request(now, RX_HZ, RX_HZ, 0).expect("the input mode");
            ack(&mut board, now, [0; 4]);
            assert_eq!((sel[3], sel[4]), (REG_RF_INPUTS, code), "{mode:?}");
        }
        // The default is what the reset already wrote, so nothing follows it but
        // the frequencies.
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        find(&mut board, now);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("the reset");
        ack(&mut board, now, [0; 4]);
        let next = board.next_request(now, RX_HZ, RX_HZ, 0).expect("straight to the frequency");
        assert_eq!(next[3], REG_TX_FREQ_BYTE4, "no redundant mode write");
    }

    /// Issue #292: the receive input follows the band, so it has to be movable
    /// while running — written at once to a board already found, and to one
    /// still being probed after the reset that would otherwise undo it.
    #[test]
    fn the_receive_input_can_be_moved_while_running() {
        let now = Instant::now();
        let mut board = IoBoard::new(HpsdrIoRxInput::IoBoard);
        find(&mut board, now);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("the reset");
        ack(&mut board, now, [0; 4]);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("the input mode");
        ack(&mut board, now, [0; 4]);
        // The band changes to one kept on the radio's own jack.
        board.set_rx_input(HpsdrIoRxInput::Radio);
        let mut wrote = None;
        for _ in 0..10 {
            let Some(cc) = board.next_request(now, RX_HZ, RX_HZ, 0) else { break };
            ack(&mut board, now, [0; 4]);
            if cc[3] == REG_RF_INPUTS {
                wrote = Some(cc[4]);
                break;
            }
        }
        assert_eq!(wrote, Some(0), "the radio's own input is mode 0");
        // Setting what it already is costs nothing.
        board.set_rx_input(HpsdrIoRxInput::Radio);
        assert!(board.queue.iter().all(|op| op.reg != REG_RF_INPUTS));

        // Before the board is found, the choice waits for the start-up writes.
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        board.set_rx_input(HpsdrIoRxInput::IoBoard);
        find(&mut board, now);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("the reset");
        ack(&mut board, now, [0; 4]);
        let sel = board.next_request(now, RX_HZ, RX_HZ, 0).expect("the input mode");
        assert_eq!((sel[3], sel[4]), (REG_RF_INPUTS, 1));
    }

    #[test]
    fn a_radio_without_an_io_board_gives_up_and_stays_quiet() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        for _ in 0..PROBE_TRIES {
            assert!(board.next_request(now, 14_074_000, RX_HZ, 0).is_some(), "probe");
            board.on_ack(RADDR_ERROR, [0, 0, 0, 0], now);
            now += PROBE_INTERVAL;
        }
        assert_eq!(board.presence, Presence::Absent);
        // And never touches the bus again, however long it runs or however far
        // the operator tunes.
        for _ in 0..50 {
            now += Duration::from_secs(1);
            assert_eq!(board.next_request(now, 7_074_000, RX_HZ, 0), None);
        }
    }

    #[test]
    fn the_frequency_goes_out_big_endian_with_the_latching_byte_last() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        board.next_request(now, 14_074_000, RX_HZ, 0); // the reset
        ack(&mut board, now, [0; 4]);

        let mut sent = Vec::new();
        for _ in 0..5 {
            let cc = board.next_request(now, 14_074_000, RX_HZ, 0).expect("a frequency byte");
            ack(&mut board, now, [0; 4]);
            sent.push((cc[3], cc[4]));
            now += Duration::from_millis(3);
        }
        // Registers 0..=4, most significant byte first, so BYTE0 — the write the
        // board acts on — is last.
        assert_eq!(
            sent,
            vec![
                (0, 0x00), // BYTE4: a 32-bit Hz value never reaches here
                (1, 0x00), // BYTE3
                (2, 0xD6), // 14_074_000 = 0x00_00D6C090
                (3, 0xC0),
                (4, 0x90), // BYTE0 latches
            ]
        );
        assert_eq!(sent[4].0, REG_TX_FREQ_BYTE0, "the latching write is last");

        // The receive code follows in the same batch, once.
        let rx = board.next_request(now, 14_074_000, RX_HZ, 0).expect("the receive code");
        ack(&mut board, now, [0; 4]);
        assert_eq!((rx[3], rx[4]), (REG_FCODE_RX1, hertz_to_code(RX_HZ)));

        now += Duration::from_millis(3);
        now = complete_tuner_priming(&mut board, now);

        // The dial is still, but periodic tuner-status polling continues.
        now += FREQ_INTERVAL * 2;
        let status = board
            .next_request(now, 14_074_000, RX_HZ, 0)
            .expect("periodic tuner status poll");
        assert_eq!(status[1], I2C_READ);
        assert_eq!(status[3], REG_ANTENNA_TUNER);
        ack(&mut board, now, [0x00, 0, 0, 0]);

        // No further traffic is due at this instant.
        assert_eq!(board.next_request(now, 14_074_000, RX_HZ, 0), None);
    }

    /// The receive code is what the board picks an antenna and preselector on.
    /// It is deduplicated on the *code*, and a code is 6.68 % wide — so tuning
    /// around a band says nothing at all while moving to another says it once.
    #[test]
    fn the_receive_code_speaks_on_a_band_change_and_not_on_a_tune() {
        // Each amateur band is its own code, and adjacent bands never share
        // one, so a band change is always heard.
        let bands = [1_900_000, 3_650_000, 7_100_000, 14_175_000, 21_225_000, 28_500_000];
        for pair in bands.windows(2) {
            assert_ne!(hertz_to_code(pair[0]), hertz_to_code(pair[1]), "{pair:?} share a code");
        }
        // A code spans 6.68 %, but the boundaries sit where they sit rather
        // than on band edges, so a band can straddle two (20 m crosses one at
        // ~14.145 MHz). That costs at most one extra write mid-band and is
        // exactly as correct — the code still names where the receiver is.
        // What matters is that ordinary tuning is silent:
        assert_eq!(hertz_to_code(14_000_000), hertz_to_code(14_100_000));
        assert_eq!(hertz_to_code(7_000_000), hertz_to_code(7_200_000));
        // The rails: zero means "not specified", and nothing under-runs it.
        assert_eq!(hertz_to_code(0), 0);
        assert_eq!(hertz_to_code(1), 1, "below the curve's floor, still a valid code");
        assert_eq!(hertz_to_code(18_748), 1);
        // Quisk's own figures, to the code.
        assert_eq!(hertz_to_code(14_074_000), 102);
        assert_eq!(hertz_to_code(7_074_000), 92);

        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        // Drain the reset, the transmit frequency and the first receive code.
        for _ in 0..7 {
            board.next_request(now, 14_074_000, 14_074_000, 0).expect("startup traffic");
            ack(&mut board, now, [0; 4]);
        }
        now += Duration::from_millis(3);
        now = complete_tuner_priming(&mut board, now);
        // Tuning around 20 m does not change the transmit frequency or receive
        // code, but periodic tuner-status polling continues.
        for hz in [14_000_000, 14_050_000, 14_100_000, 14_074_000] {
            now += FREQ_INTERVAL;
            let status = board
                .next_request(now, 14_074_000, hz, 0)
                .expect("periodic tuner status poll");
            assert_eq!(status[1], I2C_READ, "expected a status read at {hz}");
            assert_eq!(status[3], REG_ANTENNA_TUNER);
            ack(&mut board, now, [0x00, 0, 0, 0]);
            assert_eq!(board.next_request(now, 14_074_000, hz, 0), None, "no extra traffic at {hz}");
        }
        // Moving to 40 m says it, once.
        now += FREQ_INTERVAL;
        let cc = board.next_request(now, 14_074_000, 7_074_000, 0).expect("a band change");
        ack(&mut board, now, [0; 4]);
        assert_eq!((cc[3], cc[4]), (REG_FCODE_RX1, 92));
        now += FREQ_INTERVAL;
        let status = board
            .next_request(now, 14_074_000, 7_100_000, 0)
            .expect("periodic tuner status poll");
        assert_eq!(status[1], I2C_READ);
        assert_eq!(status[3], REG_ANTENNA_TUNER);
        ack(&mut board, now, [0x00, 0, 0, 0]);
        assert_eq!(
            board.next_request(now, 14_074_000, 7_100_000, 0),
            None,
            "no extra traffic while still on 40 m"
        );
    }

    #[test]
    fn frequency_updates_are_rate_limited_but_never_lost() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        board.next_request(now, 7_074_000, RX_HZ, 0); // the reset
        ack(&mut board, now, [0; 4]);

        // Drain the first frequency and the receive code that follows it.
        for _ in 0..6 {
            board.next_request(now, 7_074_000, RX_HZ, 0).expect("first frequency");
            ack(&mut board, now, [0; 4]);
        }

        now += Duration::from_millis(3);
        now = complete_tuner_priming(&mut board, now);
        // Start a fresh rate-limit window so this test isolates
        // frequency-update rate limiting from discovery timing.
        board.sent_at = Some(now);
        // A band change inside the rate-limit window is held, not dropped...
        now += Duration::from_millis(50);
        assert_eq!(board.next_request(now, 14_074_000, RX_HZ, 0), None, "too soon");

        // ...and a second change replaces the first. At 100 ms, the
        // periodic tuner-status poll is due, so service it before continuing.
        now += Duration::from_millis(50);

        let status = board
            .next_request(now, 21_074_000, RX_HZ, 0)
            .expect("periodic tuner status poll");
        assert_eq!(status[1], I2C_READ);
        assert_eq!(status[3], REG_ANTENNA_TUNER);
        ack(&mut board, now, [0x00, 0, 0, 0]);
        assert_eq!(
            board.next_request(now, 21_074_000, RX_HZ, 0),
            None,
            "frequency change is still rate-limited"
        );
        now += FREQ_INTERVAL;
        let cc = board.next_request(now, 21_074_000, RX_HZ, 0).expect("the held change goes out");
        assert_eq!(cc[3], REG_TX_FREQ_BYTE4);
        ack(&mut board, now, [0; 4]);
        let mut bytes = vec![cc[4]];
        for _ in 0..4 {
            let cc = board.next_request(now, 21_074_000, RX_HZ, 0).expect("the rest");
            ack(&mut board, now, [0; 4]);
            bytes.push(cc[4]);
        }
        assert_eq!(u32::from_be_bytes(bytes[1..5].try_into().unwrap()), 21_074_000);
    }

    #[test]
    fn an_unanswered_write_quarantines_the_bus_without_retrying() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        board.next_request(now, 7_074_000, RX_HZ, 0).expect("reset");
        ack(&mut board, now, [0; 4]);

        let first = board.next_request(now, 7_074_000, RX_HZ, 0)
            .expect("first frequency byte");
        now += ACK_TIMEOUT;

        assert_eq!(
            board.next_request(now, 7_074_000, RX_HZ, 0),
            None,
            "timed-out request must not be retransmitted"
        );
        assert!(board.i2c_quarantined);
        assert!(board.inflight.is_none());
        assert_eq!(
            board.next_request(now + ACK_TIMEOUT, 7_074_000, RX_HZ, 0),
            None,
            "quarantine must block further requests"
        );
        assert_eq!(first[3], REG_TX_FREQ_BYTE4);
    }

    #[test]
    fn the_mox_bit_rides_i2c_commands_like_every_other_register() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();
        let probe = board.next_request(now, 14_074_000, RX_HZ, 1).expect("probe");
        assert_eq!(probe[0] & 1, 1, "keyed");
        assert_eq!(probe[0] & !1, CC_I2C2 | CC_RQST);
    }

    #[test]
    fn an_ack_for_someone_else_does_not_strand_our_request() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();
        board.next_request(now, 14_074_000, RX_HZ, 0).expect("probe");
        // An answer to an unrelated register must not be read as ours.
        board.on_ack(0x09, [0xF1, 0, 0, 0], now);
        assert_ne!(board.presence, Presence::Present, "that was not our answer");
        // The real one still lands.
        board.on_ack(CC_I2C2 >> 1, [0xF1, 0, 0, 0], now);
        assert_eq!(board.presence, Presence::Present);
    }
}
