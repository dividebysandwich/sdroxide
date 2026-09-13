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
    pub(crate) fn new(rx_input: HpsdrIoRxInput) -> IoBoard {
        IoBoard {
            presence: Presence::Probing(PROBE_TRIES),
            rx_input,
            inflight: None,
            queue: VecDeque::new(),
            sent_hz: None,
            sent_rx_code: None,
            sent_at: None,
            probe_at: None,
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
        // Drain the reset, the transmit frequency and the first receive code.
        for _ in 0..7 {
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
