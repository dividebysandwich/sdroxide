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

use std::sync::mpsc::Sender;
use crate::net::HpsdrAtuCommandResult;

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
/// Where the board takes its receive signal from: the radio's own input (0, and
/// what the reset leaves), the board's own J9 SMA (1), or J9 with the J10
/// PureSignal jack switched in on transmit (2).
const REG_RF_INPUTS: u8 = 11;

/// Generic antenna tuner interface registers on the Pico.
const REG_ANTENNA_TUNER: u8 = 7;

/// Generic tuner discovery command and its response codes.
const TUNER_DISCOVER: u8 = 0x03;
const TUNER_PRESENT: u8 = 0x01;
const TUNER_ABSENT: u8 = 0x02;
/// Receiver 1's frequency, as a one-byte *code* rather than a frequency —
/// registers 13..=24 hold one for each of the twelve receivers the protocol can
/// carry. The board uses them to pick a receive antenna and preselector; all
/// zero (which is what the reset leaves) means "receive on the transmit band".
///
/// Protocol 1 carries exactly one receiver, so only this one is ever written.
const REG_FCODE_RX1: u8 = 13;

/// How long to wait for an ACK before sending the request again.
const ACK_TIMEOUT: Duration = Duration::from_millis(150);
/// Attempts per transaction before it is abandoned. Three tries inside the
/// half-second frequency cadence, so a retry can never collide with the next
/// frequency.
const MAX_TRIES: u8 = 3;
/// Minimum spacing between transmit-frequency updates. The board's
/// documentation asks for at most one every half second (Quisk's figure), since
/// an operator spinning the dial would otherwise flood the I2C bus.
const FREQ_INTERVAL: Duration = Duration::from_millis(500);
/// Probes before concluding there is no IO board on this radio.
const PROBE_TRIES: u8 = 3;
/// Gap between those probes.
const PROBE_INTERVAL: Duration = Duration::from_millis(500);

/// Interval between reads while the Pico completes tuner discovery.
const TUNER_POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Maximum time allowed for generic tuner discovery.
const TUNER_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Interval between runtime tuner status reads.
const TUNER_STATUS_POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Maximum age of a tuner status that may authorise a command.
const TUNER_STATUS_MAX_AGE: Duration = Duration::from_millis(200);

/// Generic tuner commands written to REG_ANTENNA_TUNER.
const TUNER_CMD_TUNE: u8 = 0x01;
const TUNER_CMD_BYPASS: u8 = 0x02;

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

/// Progress of the generic Pico tuner discovery exchange.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TunerDiscovery {
    /// The I/O Board has not yet completed its startup reset.
    NotStarted,
    /// Discovery command 0x03 has been sent; its ACK is awaited.
    AwaitingCommandAck,
    /// Discovery command was acknowledged; the four-byte response is awaited.
    AwaitingResponse,
    /// Compatible firmware was identified.
    Complete { present: bool, firmware_major: u8, firmware_minor: u8 },
    /// The response did not match the generic tuner protocol.
    Unsupported,
}

/// A transaction on the wire, waiting to be acknowledged.
struct InFlight {
    op: Op,
    sent_at: Instant,
    tries: u8,
}

/// Drives one HL2IOBoard: finds it, resets it, and keeps it told of the
/// transmit frequency.
pub(crate) struct IoBoard {
    presence: Presence,
    tuner_discovery: TunerDiscovery,
    tuner_discovery_deadline: Option<Instant>,
    tuner_poll_at: Option<Instant>,

    /// Most recently reported raw tuner status and the time it was received.
    tuner_status: Option<(u8, Instant)>,
    /// When the next runtime status poll is due.
    tuner_status_poll_at: Option<Instant>,
    /// True while a TUNE or BYPASS write is awaiting its I2C ACK.
    tuner_command_pending: bool,
    /// Delivers the result of the command's I2C write to its requesting stream.
    tuner_command_reply: Option<Sender<HpsdrAtuCommandResult>>,
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
    /// The receive input mode the board has been told. Restated like the
    /// frequencies, so a write lost to an I2C failure is sent again rather than
    /// leaving J9 on the old input until the operator moves it (#444).
    sent_input_code: Option<u8>,
    /// When the last frequency went out, for the half-second rate limit. Shared
    /// by both directions: it exists to keep the I2C bus quiet, and the bus does
    /// not care which of them is talking.
    sent_at: Option<Instant>,
    /// When the next probe may go out.
    probe_at: Option<Instant>,
}

impl IoBoard {
    pub(crate) fn new(rx_input: HpsdrIoRxInput) -> IoBoard {
        IoBoard {
            presence: Presence::Probing(PROBE_TRIES),
            tuner_discovery: TunerDiscovery::NotStarted,
            tuner_discovery_deadline: None,
            tuner_poll_at: None,
            tuner_status: None,
            tuner_status_poll_at: None,
            tuner_command_pending: false,
            tuner_command_reply: None,
            rx_input,
            inflight: None,
            queue: VecDeque::new(),
            sent_hz: None,
            sent_rx_code: None,
            sent_input_code: None,
            sent_at: None,
            probe_at: None,
        }
    }

    /// Return the current tuner discovery and status snapshot.
    pub(crate) fn status_snapshot(&self) -> crate::net::HpsdrTunerStatus {
        let present = match self.tuner_discovery {
            TunerDiscovery::Complete { present, .. } => Some(present),
            _ => None,
        };

        let (raw_status, updated_at) = match self.tuner_status {
            Some((status, received_at)) => (Some(status), Some(received_at)),
            None => (None, None),
        };

        crate::net::HpsdrTunerStatus { present, raw_status, updated_at }
    }

    /// Move the receive input while running.
    ///
    /// A board already found is told with the next request; one still being
    /// probed picks it up after the reset that would otherwise undo it.
    pub(crate) fn set_rx_input(&mut self, input: HpsdrIoRxInput) {
        self.rx_input = input;
    }

    /// Request a generic tuner tune operation.
    ///
    /// The request is accepted only when a tuner is positively detected,
    /// its latest status is fresh and exactly idle (0x00), and no tuner
    /// command is already awaiting acknowledgement.
    pub(crate) fn request_tune(
        &mut self,
        now: Instant,
        reply: Sender<HpsdrAtuCommandResult>,
    ) -> bool {
        self.request_tuner_command(TUNER_CMD_TUNE, now, reply)
    }

    /// Request generic tuner bypass.
    pub(crate) fn request_bypass(
        &mut self,
        now: Instant,
        reply: Sender<HpsdrAtuCommandResult>,
    ) -> bool {
        self.request_tuner_command(TUNER_CMD_BYPASS, now, reply)
    }

    fn request_tuner_command(
        &mut self,
        command: u8,
        now: Instant,
        reply: Sender<HpsdrAtuCommandResult>,
    ) -> bool {
        let tuner_present =
            matches!(self.tuner_discovery, TunerDiscovery::Complete { present: true, .. });

        let status_is_idle_and_fresh = self.tuner_status.is_some_and(|(status, received_at)| {
            status == 0x00
                && now
                    .checked_duration_since(received_at)
                    .is_some_and(|age| age <= TUNER_STATUS_MAX_AGE)
        });

        if !tuner_present || !status_is_idle_and_fresh || self.tuner_command_pending {
            tracing::debug!(
                "HL2IOBoard: tuner command 0x{command:02X} rejected \
                 (present={tuner_present}, status={:?}, pending={})",
                self.tuner_status.map(|(status, _)| status),
                self.tuner_command_pending
            );
            let _ = reply.send(HpsdrAtuCommandResult::Rejected);
            return false;
        }

        self.queue.push_back(Op::write(REG_ANTENNA_TUNER, command));
        self.tuner_command_pending = true;
        self.tuner_command_reply = Some(reply);

        // The old idle status must not authorise another command. A fresh
        // status read is required after this command has been acknowledged.
        self.tuner_status = None;
        self.tuner_status_poll_at = None;

        tracing::info!("HL2IOBoard: tuner command 0x{command:02X} queued");
        true
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
        // A request still waiting on its answer holds the bus: the protocol
        // document asks for one outstanding request at a time.
        if let Some(f) = self.inflight.as_mut() {
            if now < f.sent_at + ACK_TIMEOUT {
                return None;
            }
            f.tries += 1;
            if f.tries < MAX_TRIES {
                // EP2 is UDP: a request can simply have been lost, and sending
                // the same one-byte write again is harmless either way.
                f.sent_at = now;
                return Some(f.op.cc(mox));
            }
            let op = self.inflight.take().expect("checked above").op;
            self.give_up(op, now, "no answer");
        }

        // Continue generic tuner discovery without blocking normal I2C traffic.
        if self.tuner_discovery == TunerDiscovery::AwaitingResponse {
            if self.tuner_discovery_deadline.is_some_and(|deadline| now >= deadline) {
                self.tuner_discovery = TunerDiscovery::Unsupported;
                self.tuner_discovery_deadline = None;
                self.tuner_poll_at = None;

                // Remove any queued discovery read, but preserve normal I2C work.
                self.queue.retain(|op| {
                    !(op.cookie == I2C_READ && op.addr == IOB_ADDR && op.reg == REG_ANTENNA_TUNER)
                });

                tracing::warn!(
                    "HL2IOBoard: generic tuner discovery timed out; \
                     firmware may not support the interface"
                );
            } else if self.tuner_poll_at.is_some_and(|poll_at| now >= poll_at) {
                self.tuner_poll_at = None;
                self.queue.push_front(Op {
                    cookie: I2C_READ,
                    addr: IOB_ADDR,
                    reg: REG_ANTENNA_TUNER,
                    data: 0,
                });
            }
        }

        // Poll the generic tuner status while a positively detected tuner
        // is available. Do not poll while a command write is awaiting ACK.
        if matches!(self.tuner_discovery, TunerDiscovery::Complete { present: true, .. })
            && !self.tuner_command_pending
            && self.tuner_status_poll_at.is_some_and(|poll_at| now >= poll_at)
        {
            let read_already_queued = self.queue.iter().any(|op| {
                op.cookie == I2C_READ && op.addr == IOB_ADDR && op.reg == REG_ANTENNA_TUNER
            });

            let read_in_flight = self.inflight.as_ref().is_some_and(|f| {
                f.op.cookie == I2C_READ && f.op.addr == IOB_ADDR && f.op.reg == REG_ANTENNA_TUNER
            });

            self.tuner_status_poll_at = None;

            if !read_already_queued && !read_in_flight {
                self.queue.push_back(Op {
                    cookie: I2C_READ,
                    addr: IOB_ADDR,
                    reg: REG_ANTENNA_TUNER,
                    data: 0,
                });
            }
        }

        self.refill(now, tx_freq_hz, rx_freq_hz);
        let op = self.queue.pop_front()?;
        self.inflight = Some(InFlight { op, sent_at: now, tries: 0 });
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
                let input = self.rx_input.code();
                if self.sent_input_code != Some(input) {
                    self.sent_input_code = Some(input);
                    self.queue.push_back(Op::write(REG_RF_INPUTS, input));
                    tracing::info!(
                        "HL2IOBoard: receive input -> {} (mode {input})",
                        self.rx_input.label()
                    );
                }
                let rx_code = hertz_to_code(rx_freq_hz);
                let tx_stale = self.sent_hz != Some(tx_freq_hz);
                let rx_stale = self.sent_rx_code != Some(rx_code);
                if !tx_stale && !rx_stale {
                    return;
                }
                // Rate limit the *sending*, not the value: whatever the
                // frequency is when the interval expires is what goes out, so a
                // band change during a fast tune is never the one that gets
                // dropped.
                if self.sent_at.is_some_and(|t| now < t + FREQ_INTERVAL) {
                    return;
                }
                self.sent_at = Some(now);
                // Transmit first: it is the one an amplifier switches on, and
                // the one that has to be right before any RF appears.
                if tx_stale {
                    self.sent_hz = Some(tx_freq_hz);
                    self.queue.extend(freq_writes(tx_freq_hz));
                    tracing::debug!("HL2IOBoard: transmit frequency -> {tx_freq_hz} Hz");
                }
                if rx_stale {
                    self.sent_rx_code = Some(rx_code);
                    self.queue.push_back(Op::write(REG_FCODE_RX1, rx_code));
                    tracing::debug!(
                        "HL2IOBoard: receive frequency -> {rx_freq_hz} Hz (code {rx_code})"
                    );
                }
            }
            Presence::Absent => {}
        }
    }

    /// An acknowledgement came back on EP6. `raddr` is the address the gateware
    /// echoed and `data` is C1..C4 — for a read, the four bytes it fetched.
    pub(crate) fn on_ack(&mut self, raddr: u8, data: [u8; 4], now: Instant) {
        // Only ACKs for the I2C bus this drives are ours. Nothing else in this
        // crate sets RQST, but taking someone else's answer would strand the
        // real request forever.
        let ours = raddr == CC_I2C2 >> 1 || raddr == RADDR_ERROR;
        let Some(f) = (if ours { self.inflight.take() } else { None }) else {
            return;
        };
        if raddr == RADDR_ERROR {
            // Nothing acknowledged that I2C address, or the bus was busy.
            self.give_up(f.op, now, "the radio reported an I2C error");
            return;
        }
        // Start generic tuner discovery after the normal board reset.
        if self.tuner_discovery == TunerDiscovery::NotStarted
            && f.op.cookie == I2C_WRITE
            && f.op.reg == REG_CONTROL
        {
            self.tuner_discovery = TunerDiscovery::AwaitingCommandAck;
            self.queue.push_front(Op::write(REG_ANTENNA_TUNER, TUNER_DISCOVER));
            tracing::info!("HL2IOBoard: requesting generic tuner discovery");
        }
        // The discovery command was acknowledged. Read registers 7 through 10.
        else if self.tuner_discovery == TunerDiscovery::AwaitingCommandAck
            && f.op.cookie == I2C_WRITE
            && f.op.reg == REG_ANTENNA_TUNER
        {
            self.tuner_discovery = TunerDiscovery::AwaitingResponse;
            self.tuner_discovery_deadline = Some(now + TUNER_DISCOVERY_TIMEOUT);
            self.tuner_poll_at = None;
            self.queue.push_front(Op {
                cookie: I2C_READ,
                addr: IOB_ADDR,
                reg: REG_ANTENNA_TUNER,
                data: 0,
            });
        }
        // Validate the discovery response.
        else if self.tuner_discovery == TunerDiscovery::AwaitingResponse
            && f.op.cookie == I2C_READ
            && f.op.addr == IOB_ADDR
            && f.op.reg == REG_ANTENNA_TUNER
        {
            tracing::info!(
                "HL2IOBoard: tuner discovery raw bytes C1..C4 = {:02X} {:02X} {:02X} {:02X}",
                data[0],
                data[1],
                data[2],
                data[3]
            );

            // The Pico returns the four registers in descending order:
            // C1 = register 10, C2 = register 9, C3 = register 8, C4 = register 7.
            let firmware_minor = data[0];
            let firmware_major = data[1];
            let tuner_code = data[2];
            let tuner_status = data[3];

            if tuner_status == 0 && tuner_code == TUNER_PRESENT {
                self.tuner_discovery =
                    TunerDiscovery::Complete { present: true, firmware_major, firmware_minor };
                self.tuner_discovery_deadline = None;
                self.tuner_poll_at = None;
                tracing::info!(
                    "HL2IOBoard: generic tuner firmware {firmware_major}.{firmware_minor}; tuner present"
                );
            } else if tuner_status == 0 && tuner_code == TUNER_ABSENT {
                self.tuner_discovery =
                    TunerDiscovery::Complete { present: false, firmware_major, firmware_minor };
                self.tuner_discovery_deadline = None;
                self.tuner_poll_at = None;
                tracing::info!(
                    "HL2IOBoard: generic tuner firmware {firmware_major}.{firmware_minor}; no tuner present"
                );
            } else if tuner_status == TUNER_DISCOVER
                && self.tuner_discovery_deadline.is_some_and(|deadline| now < deadline)
            {
                // The Pico has received the command but has not completed
                // its own startup discovery. Schedule another read.
                self.tuner_poll_at = Some(now + TUNER_POLL_INTERVAL);
            } else {
                self.tuner_discovery = TunerDiscovery::Unsupported;
                self.tuner_discovery_deadline = None;
                self.tuner_poll_at = None;
                tracing::info!(
                    "HL2IOBoard: generic tuner interface not recognised \
                     (registers 7-10: {:02X} {:02X} {:02X} {:02X})",
                    data[0],
                    data[1],
                    data[2],
                    data[3]
                );
            }
        }

        // Runtime status reads return register 7 in the fourth response byte.
        // Preserve the raw value: only 0x00 has idle-command permission.
        if f.op.cookie == I2C_READ
            && f.op.addr == IOB_ADDR
            && f.op.reg == REG_ANTENNA_TUNER
            && matches!(self.tuner_discovery, TunerDiscovery::Complete { present: true, .. })
        {
            let status = data[3];
            self.tuner_status = Some((status, now));
            self.tuner_status_poll_at = Some(now + TUNER_STATUS_POLL_INTERVAL);

            tracing::debug!("HL2IOBoard: tuner status -> 0x{status:02X}");
        }

        // A command ACK confirms the I2C write was received, not that
        // the tuner operation has completed. Require a new status read.
        if f.op.cookie == I2C_WRITE
            && f.op.addr == IOB_ADDR
            && f.op.reg == REG_ANTENNA_TUNER
            && matches!(f.op.data, TUNER_CMD_TUNE | TUNER_CMD_BYPASS)
        {
            self.tuner_command_pending = false;
            if let Some(reply) = self.tuner_command_reply.take() {
                let _ = reply.send(HpsdrAtuCommandResult::Acknowledged);
            }
            self.tuner_status = None;
            self.tuner_status_poll_at = Some(now);
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
                self.sent_input_code = Some(0);
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
    }

    /// A transaction that will not complete.
    fn give_up(&mut self, op: Op, now: Instant, why: &str) {
        if let Presence::Probing(_) = self.presence {
            return self.probe_failed(now, why);
        }
        // A failed tuner-discovery transaction must not disrupt ordinary
        // frequency or receive-input traffic.
        if op.addr == IOB_ADDR
            && op.reg == REG_ANTENNA_TUNER
            && matches!(
                self.tuner_discovery,
                TunerDiscovery::AwaitingCommandAck | TunerDiscovery::AwaitingResponse
            )
        {
            self.tuner_discovery = TunerDiscovery::Unsupported;
            self.tuner_discovery_deadline = None;
            self.tuner_poll_at = None;

            self.queue
                .retain(|queued| !(queued.addr == IOB_ADDR && queued.reg == REG_ANTENNA_TUNER));

            tracing::warn!(
                "HL2IOBoard: tuner discovery transaction failed ({why}); \
                 continuing without generic tuner support"
            );
            return;
        }

        // A failed runtime tuner read or command must not disrupt the
        // ordinary frequency and receive-input update sequence.
        if op.addr == IOB_ADDR
            && op.reg == REG_ANTENNA_TUNER
            && matches!(self.tuner_discovery, TunerDiscovery::Complete { present: true, .. })
        {
            if op.cookie == I2C_WRITE && matches!(op.data, TUNER_CMD_TUNE | TUNER_CMD_BYPASS) {
                self.tuner_command_pending = false;
                if let Some(reply) = self.tuner_command_reply.take() {
                    let _ = reply.send(HpsdrAtuCommandResult::Failed);
                }
            }

            self.tuner_status = None;
            self.tuner_status_poll_at = Some(now + TUNER_STATUS_POLL_INTERVAL);

            tracing::warn!(
                "HL2IOBoard: tuner transaction failed ({why}); \
                 waiting for another status read"
            );
            return;
        }

        // Abandon the rest of the sequence with it. A frequency the board only
        // half received is not a frequency — and the byte that went missing may
        // have been the one that latches — so forget that it was ever sent and
        // say the whole five bytes again from the top.
        // An input write that failed or is dropped with the queue has to be
        // said again too; one that already landed does not.
        if op.reg == REG_RF_INPUTS || self.queue.iter().any(|q| q.reg == REG_RF_INPUTS) {
            self.sent_input_code = None;
        }
        self.queue.clear();
        self.sent_hz = None;
        self.sent_rx_code = None;
        self.sent_at = None;
        tracing::warn!(
            "HL2IOBoard: I2C write to register {} failed ({why}); sending the transmit \
             frequency again",
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

    /// Complete the board reset and generic tuner discovery exchange.
    /// Returns the four-byte read request, which the individual tests answer.
    fn request_tuner_discovery(board: &mut IoBoard, now: Instant) -> [u8; 5] {
        find(board, now);
        // Acknowledge the normal I/O Board reset.
        let reset = board.next_request(now, RX_HZ, RX_HZ, 0).expect("board reset");
        assert_eq!((reset[3], reset[4]), (REG_CONTROL, 1));
        ack(board, now, [0; 4]);

        // The host should now write discovery command 0x03.
        let discover = board.next_request(now, RX_HZ, RX_HZ, 0).expect("tuner discovery command");
        assert_eq!(discover[1], I2C_WRITE);
        assert_eq!((discover[3], discover[4]), (REG_ANTENNA_TUNER, TUNER_DISCOVER));
        ack(board, now, [0; 4]);

        // The command ACK must be followed by a separate read of registers 7-10.
        let read = board.next_request(now, RX_HZ, RX_HZ, 0).expect("tuner discovery response");
        assert_eq!(read[1], I2C_READ);
        assert_eq!(read[2], IOB_ADDR);
        assert_eq!(read[3], REG_ANTENNA_TUNER);
        read
    }

    /// Complete discovery after the normal board reset has been acknowledged.
    /// The existing frequency and input tests do not require a connected tuner.
    fn complete_tuner_discovery(board: &mut IoBoard, now: Instant) {
        let discover = board.next_request(now, RX_HZ, RX_HZ, 0).expect("tuner discovery command");
        assert_eq!((discover[3], discover[4]), (REG_ANTENNA_TUNER, TUNER_DISCOVER));
        ack(board, now, [0; 4]);

        let read = board.next_request(now, RX_HZ, RX_HZ, 0).expect("tuner discovery response");
        assert_eq!((read[1], read[3]), (I2C_READ, REG_ANTENNA_TUNER));
        ack(board, now, [9, 7, TUNER_ABSENT, 0x00]);

        assert!(matches!(board.tuner_discovery, TunerDiscovery::Complete { present: false, .. }));
    }

    #[test]
    fn generic_tuner_discovery_detects_present_tuner_and_firmware_version() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        let read = request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        assert_eq!(
            board.tuner_discovery,
            TunerDiscovery::Complete { present: true, firmware_major: 7, firmware_minor: 9 }
        );

        // The discovery result must not prevent normal frequency updates.
        assert_eq!(board.presence, Presence::Present);
        assert!(!board.queue.iter().any(|op| op.reg == REG_ANTENNA_TUNER));
        assert_eq!(read[3], REG_ANTENNA_TUNER);
    }

    #[test]
    fn generic_tuner_discovery_waits_for_pico_startup() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();

        request_tuner_discovery(&mut board, now);

        // The Pico has not finished its own startup discovery.
        ack(&mut board, now, [0x00, 0x00, 0x00, TUNER_DISCOVER]);

        assert_eq!(board.tuner_discovery, TunerDiscovery::AwaitingResponse);

        // Wait for the scheduled 50 ms polling interval.
        now += TUNER_POLL_INTERVAL;

        let poll = board.next_request(now, RX_HZ, RX_HZ, 0).expect("next discovery poll");

        assert_eq!((poll[1], poll[3]), (I2C_READ, REG_ANTENNA_TUNER));

        // The Pico has now completed startup and reports a tuner.
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        assert_eq!(
            board.tuner_discovery,
            TunerDiscovery::Complete { present: true, firmware_major: 7, firmware_minor: 9 }
        );

        assert_eq!(board.tuner_discovery_deadline, None);
        assert_eq!(board.tuner_poll_at, None);
    }

    #[test]
    fn failed_tuner_discovery_does_not_block_frequency_updates() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        // Start discovery and obtain its response-read request.
        let read = request_tuner_discovery(&mut board, now);
        assert_eq!((read[1], read[3]), (I2C_READ, REG_ANTENNA_TUNER));

        // Simulate the Hermes-Lite reporting an I2C transaction error.
        board.on_ack(RADDR_ERROR, [0; 4], now);

        assert_eq!(board.tuner_discovery, TunerDiscovery::Unsupported);
        assert_eq!(board.tuner_discovery_deadline, None);
        assert_eq!(board.tuner_poll_at, None);

        // Normal frequency traffic must still proceed.
        let next =
            board.next_request(now, RX_HZ, RX_HZ, 0).expect("frequency update should proceed");

        assert_eq!((next[1], next[3]), (I2C_WRITE, REG_TX_FREQ_BYTE4));
    }

    #[test]
    fn generic_tuner_discovery_detects_firmware_without_a_tuner() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_ABSENT, 0x00]);

        assert_eq!(
            board.tuner_discovery,
            TunerDiscovery::Complete { present: false, firmware_major: 7, firmware_minor: 9 }
        );
    }

    #[test]
    fn generic_tuner_discovery_rejects_older_firmware() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();

        request_tuner_discovery(&mut board, now);

        // Older firmware ignores command 0x03, leaving register 7 unchanged.
        // The host should keep polling until its discovery deadline expires.
        ack(&mut board, now, [0x00, 0x00, 0x00, TUNER_DISCOVER]);

        let deadline = now + TUNER_DISCOVERY_TIMEOUT;

        loop {
            now += TUNER_POLL_INTERVAL;
            let request = board.next_request(now, RX_HZ, RX_HZ, 0);

            if now >= deadline {
                assert_eq!(board.tuner_discovery, TunerDiscovery::Unsupported);
                break;
            }

            let poll = request.expect("next tuner discovery poll");
            assert_eq!((poll[1], poll[3]), (I2C_READ, REG_ANTENNA_TUNER));

            // Older firmware continues to report the unprocessed command.
            ack(&mut board, now, [0x00, 0x00, 0x00, TUNER_DISCOVER]);
        }
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
            complete_tuner_discovery(&mut board, now);
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
        complete_tuner_discovery(&mut board, now);
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
        complete_tuner_discovery(&mut board, now);
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
        complete_tuner_discovery(&mut board, now);
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
        complete_tuner_discovery(&mut board, now);

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

        // Nothing more to say while the dial sits still.
        now += FREQ_INTERVAL * 2;
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

        // Complete the reset and generic tuner discovery.
        let reset = board.next_request(now, 14_074_000, 14_074_000, 0).expect("board reset");
        assert_eq!((reset[3], reset[4]), (REG_CONTROL, 1));
        ack(&mut board, now, [0; 4]);
        complete_tuner_discovery(&mut board, now);

        // Drain the transmit frequency and the first receive code.
        for _ in 0..6 {
            board.next_request(now, 14_074_000, 14_074_000, 0).expect("startup traffic");
            ack(&mut board, now, [0; 4]);
        }
        // Tuning around 20 m: the transmit frequency is unchanged here, and the
        // receive code does not move either, so the bus stays silent.
        for hz in [14_000_000, 14_050_000, 14_100_000, 14_074_000] {
            now += FREQ_INTERVAL;
            assert_eq!(board.next_request(now, 14_074_000, hz, 0), None, "still 20 m at {hz}");
        }
        // Moving to 40 m says it, once.
        now += FREQ_INTERVAL;
        let cc = board.next_request(now, 14_074_000, 7_074_000, 0).expect("a band change");
        ack(&mut board, now, [0; 4]);
        assert_eq!((cc[3], cc[4]), (REG_FCODE_RX1, 92));
        now += FREQ_INTERVAL;
        assert_eq!(board.next_request(now, 14_074_000, 7_100_000, 0), None, "still 40 m");
    }

    #[test]
    fn frequency_updates_are_rate_limited_but_never_lost() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        board.next_request(now, 7_074_000, RX_HZ, 0); // the reset
        ack(&mut board, now, [0; 4]);
        complete_tuner_discovery(&mut board, now);

        // Drain the first frequency and the receive code that follows it.
        for _ in 0..6 {
            board.next_request(now, 7_074_000, RX_HZ, 0).expect("first frequency");
            ack(&mut board, now, [0; 4]);
        }

        // A band change inside the rate-limit window is held, not dropped...
        now += Duration::from_millis(50);
        assert_eq!(board.next_request(now, 14_074_000, RX_HZ, 0), None, "too soon");
        // ...and a second change on top of it simply replaces the first: what
        // goes out when the window opens is where the operator actually is.
        now += Duration::from_millis(50);
        assert_eq!(board.next_request(now, 21_074_000, RX_HZ, 0), None, "still too soon");
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
    fn an_unanswered_write_is_retried_and_then_re_sent_with_the_next_update() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let mut now = Instant::now();
        find(&mut board, now);
        board.next_request(now, 7_074_000, RX_HZ, 0); // the reset
        ack(&mut board, now, [0; 4]);
        complete_tuner_discovery(&mut board, now);

        let first = board.next_request(now, 7_074_000, RX_HZ, 0).expect("first frequency byte");
        // Nothing answers. It is re-sent, unchanged, up to the try limit.
        for _ in 1..MAX_TRIES {
            now += ACK_TIMEOUT;
            assert_eq!(
                board.next_request(now, 7_074_000, RX_HZ, 0),
                Some(first),
                "retried verbatim"
            );
        }
        // Then abandoned — and because the board's idea of the frequency is now
        // unknown, the next poll says it again rather than assuming it landed.
        now += ACK_TIMEOUT;
        let retry = board.next_request(now, 7_074_000, RX_HZ, 0).expect("says it again");
        assert_eq!(retry[3], REG_TX_FREQ_BYTE4, "starting from the top");
    }

    /// Issue #444: a receive-input write lost to an I2C failure is sent again,
    /// rather than leaving J9 on the old input until the operator moves it.
    #[test]
    fn a_failed_receive_input_write_is_sent_again() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();
        find(&mut board, now);
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("the reset");
        ack(&mut board, now, [0; 4]);
        while board.next_request(now, RX_HZ, RX_HZ, 0).is_some() {
            ack(&mut board, now, [0; 4]);
        }
        board.set_rx_input(HpsdrIoRxInput::IoBoard);
        let sel = board.next_request(now, RX_HZ, RX_HZ, 0).expect("the input mode");
        assert_eq!((sel[3], sel[4]), (REG_RF_INPUTS, 1));
        board.on_ack(RADDR_ERROR, [0; 4], now);
        let again = board.next_request(now, RX_HZ, RX_HZ, 0).expect("said again");
        assert_eq!((again[3], again[4]), (REG_RF_INPUTS, 1));
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

    #[test]
    fn tuner_commands_require_a_fresh_idle_status() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        assert!(board.request_tune(now, std::sync::mpsc::channel().0));
        assert!(board.tuner_command_pending);

        // A second command cannot be accepted while the first is pending.
        assert!(!board.request_bypass(now, std::sync::mpsc::channel().0));
    }

    #[test]
    fn tuner_command_is_rejected_when_status_is_not_idle() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        // Simulate a fresh status read reporting a non-idle value.
        let read = board
            .next_request(now + TUNER_STATUS_POLL_INTERVAL, RX_HZ, RX_HZ, 0)
            .expect("status poll");

        assert_eq!((read[1], read[3]), (I2C_READ, REG_ANTENNA_TUNER));

        ack(&mut board, now + TUNER_STATUS_POLL_INTERVAL, [9, 7, TUNER_PRESENT, 0x04]);

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        assert!(!board.request_tune(now + TUNER_STATUS_POLL_INTERVAL, reply_tx));
        assert_eq!(
            reply_rx.try_recv(),
            Ok(HpsdrAtuCommandResult::Rejected)
        );
        assert!(!board.tuner_command_pending);
    }

    #[test]
    fn tuner_command_is_rejected_when_status_is_stale() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        assert!(!board.request_tune(
            now + TUNER_STATUS_MAX_AGE + Duration::from_millis(1),
            std::sync::mpsc::channel().0,
        ));
    }

    #[test]
    fn tuner_command_i2c_failure_is_reported() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        assert!(board.request_tune(now, reply_tx));
        board.next_request(now, RX_HZ, RX_HZ, 0).expect("tune command");

        board.on_ack(RADDR_ERROR, [0; 4], now);

        assert_eq!(reply_rx.try_recv(), Ok(HpsdrAtuCommandResult::Failed));
        assert!(!board.tuner_command_pending);
    }

    #[test]
    fn tuner_command_ack_requires_a_new_status_read() {
        let mut board = IoBoard::new(HpsdrIoRxInput::Radio);
        let now = Instant::now();

        request_tuner_discovery(&mut board, now);
        ack(&mut board, now, [9, 7, TUNER_PRESENT, 0x00]);

        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        assert!(board.request_tune(now, reply_tx));

        let command = board.next_request(now, RX_HZ, RX_HZ, 0).expect("tune command");
        assert_eq!(
            (command[1], command[3], command[4]),
            (I2C_WRITE, REG_ANTENNA_TUNER, TUNER_CMD_TUNE)
        );

        ack(&mut board, now, [0; 4]);

        assert_eq!(
            reply_rx.try_recv(),
            Ok(HpsdrAtuCommandResult::Acknowledged)
        );
        assert!(!board.tuner_command_pending);
        assert_eq!(board.tuner_status, None);

        // The next request should be a status read, not another tuner command.
        let poll = board.next_request(now, RX_HZ, RX_HZ, 0).expect("post-command status read");
        assert_eq!((poll[1], poll[3]), (I2C_READ, REG_ANTENNA_TUNER));
    }
}
