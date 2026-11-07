//! The USB relay boards that speak over a serial port: LCUS, KMtronic, Numato.
//!
//! One transport for all three, because they differ only in what "close channel
//! 2" looks like on the wire and that is all in [`crate::frame`]. What they
//! share is more interesting than what they do not: none of them needs an
//! acknowledgement to have worked, all of them are a USB-serial bridge in front
//! of a microcontroller, and all of them are cheap enough that an operator has
//! two.
//!
//! # Why nothing here waits for a reply
//!
//! The LCUS never answers at all. KMtronic and Numato will if asked, but asking
//! at key-down would put a round trip between the operator's thumb and their
//! antenna relay for no gain: the reply says the board received the byte, not
//! that the contact has moved, and the contact moving is what the lead time is
//! for. So the write is the transaction, and the read-back is a separate
//! question the driver asks when it has time — to notice a board that has
//! quietly stopped agreeing, which is a different fault from a dead cable.

use std::io::{Read, Write};
use std::time::Duration;

use sdroxide_types::{LineState, RelayFamily, SenseLine, SerialConfig};

use crate::error::{Error, Result};
use crate::frame::{self, ChannelMask};
use crate::transport::RelayTransport;

/// The serial read timeout. Only the read-back and the input sense ever read at
/// all, and both would rather have nothing than block.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// How long a set of read-back bytes is given to turn up.
const REPLY_TIMEOUT: Duration = Duration::from_millis(250);

/// The pause between two LCUS frames. Its microcontroller drops a frame that
/// arrives hard on the heels of the last one, and it never answers, so a
/// multi-contact word sent back to back left contacts wrong (#627).
const LCUS_FRAME_GAP: Duration = Duration::from_millis(30);

pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
    family: RelayFamily,
    path: String,
    /// The contacts this configuration drives. Anything outside it is left
    /// exactly as the board had it — an operator using channels 3 and 4 of a
    /// four-way board for something else keeps them.
    managed: ChannelMask,
    /// What was last written and acknowledged by the wire going out. `None`
    /// means "unknown", which is the starting state and what a failure resets
    /// it to, so the next apply writes everything.
    last: Option<ChannelMask>,
    sense: SenseLine,
}

impl SerialTransport {
    pub fn open(
        cfg: &SerialConfig,
        family: RelayFamily,
        managed: ChannelMask,
        sense: SenseLine,
    ) -> Result<SerialTransport> {
        let path = cfg.path.trim().to_string();
        if path.is_empty() {
            return Err(Error::Config("no serial port chosen for the T/R switch".into()));
        }
        // The board's own rate, not the CAT panel's: these three families have
        // one each and an operator has no reason to know them.
        let baud = family.baud();
        let mut port = serialport::new(&path, baud)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(READ_TIMEOUT)
            .open()
            .map_err(|e| Error::opening_serial(&path, e))?;

        // Some of these boards take their supply, or their reset, from a
        // handshake line. Honour the operator's forced levels the same way the
        // CAT link does — and only when they asked, because driving RTS on a
        // board that does not care is how a relay ends up chattering.
        if let Some(level) = force(cfg.force_rts) {
            let _ = port.write_request_to_send(level);
        }
        if let Some(level) = force(cfg.force_dtr) {
            let _ = port.write_data_terminal_ready(level);
        }
        // Anything the bridge buffered before we arrived is not an answer to
        // anything we asked.
        let _ = port.clear(serialport::ClearBuffer::All);

        Ok(SerialTransport { port, family, path, managed, last: None, sense })
    }

    /// Write one channel's state, and swallow whatever the board echoes.
    ///
    /// Numato echoes every command and prints a prompt; left unread that fills
    /// the bridge's buffer and eventually blocks a write. The other two say
    /// nothing, and clearing an empty buffer costs nothing.
    fn write_channel(&mut self, ch: u8, on: bool) -> Result<()> {
        let bytes = frame::serial_set(self.family, ch, on).ok_or_else(|| {
            Error::Config(format!("channel {ch} is out of range for a {}", self.family.label()))
        })?;
        self.port.write_all(&bytes)?;
        self.port.flush()?;
        if self.family == RelayFamily::Numato {
            let _ = self.port.clear(serialport::ClearBuffer::Input);
        }
        Ok(())
    }

    /// Read until the port goes quiet or the deadline passes, whichever first.
    fn read_reply(&mut self, want: usize) -> Result<Vec<u8>> {
        let deadline = std::time::Instant::now() + REPLY_TIMEOUT;
        let mut out = Vec::with_capacity(want.max(32));
        let mut buf = [0u8; 64];
        while std::time::Instant::now() < deadline {
            match self.port.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                    if out.len() >= want && want > 0 {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if !out.is_empty() {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(out)
    }
}

/// A forced handshake level, or `None` for "leave it alone".
fn force(state: LineState) -> Option<bool> {
    match state {
        LineState::None => None,
        LineState::High => Some(true),
        LineState::Low => Some(false),
    }
}

impl RelayTransport for SerialTransport {
    fn apply(&mut self, want: ChannelMask) -> Result<()> {
        let want = want & self.managed;
        // Unknown state means write everything: after a failure the board and
        // this end have no agreement worth diffing against.
        let changed = match self.last {
            Some(had) => (had ^ want) & self.managed,
            None => self.managed,
        };
        if changed == 0 {
            return Ok(());
        }
        let mut first = true;
        for ch in 1..=u8::try_from(ChannelMask::BITS).unwrap_or(32) {
            let b = frame::bit(ch);
            if changed & b == 0 {
                continue;
            }
            if self.family == RelayFamily::Lcus && !std::mem::take(&mut first) {
                std::thread::sleep(LCUS_FRAME_GAP);
            }
            if let Err(e) = self.write_channel(ch, want & b != 0) {
                // A half-written state is not a state.
                self.last = None;
                let _ = self.port.clear(serialport::ClearBuffer::All);
                return Err(e);
            }
        }
        self.last = Some(want);
        Ok(())
    }

    fn read_back(&mut self) -> Result<Option<ChannelMask>> {
        if !self.family.reads_back() {
            // An LCUS cannot be asked, so restate what it was told instead: a
            // frame it dropped then heals off air rather than at the next
            // change of that one contact (#627).
            if self.family == RelayFamily::Lcus
                && let Some(had) = self.last.take()
            {
                self.apply(had)?;
            }
            return Ok(None);
        }
        let mut state: ChannelMask = 0;
        for ch in 1..=u8::try_from(ChannelMask::BITS).unwrap_or(32) {
            let b = frame::bit(ch);
            if self.managed & b == 0 {
                continue;
            }
            let on = match self.family {
                RelayFamily::KMtronic => {
                    self.port.write_all(&frame::kmtronic::read(ch))?;
                    self.port.flush()?;
                    let reply = self.read_reply(frame::kmtronic::REPLY_LEN)?;
                    frame::kmtronic::decode_read(ch, &reply)
                }
                RelayFamily::Numato => {
                    let q = frame::numato::read(ch)
                        .ok_or_else(|| Error::Config(format!("channel {ch} is out of range")))?;
                    self.port.write_all(&q)?;
                    self.port.flush()?;
                    let reply = self.read_reply(0)?;
                    frame::numato::decode_state(&String::from_utf8_lossy(&reply))
                }
                RelayFamily::Lcus => None,
            };
            match on {
                Some(true) => state |= b,
                Some(false) => {}
                // One channel that would not answer makes the whole reading
                // untrustworthy; a partial picture is worse than none.
                None => return Ok(None),
            }
        }
        Ok(Some(state))
    }

    fn sense(&mut self) -> Result<Option<bool>> {
        Ok(match self.sense {
            SenseLine::Off => None,
            SenseLine::Cts => Some(self.port.read_clear_to_send()?),
            SenseLine::Dsr => Some(self.port.read_data_set_ready()?),
            SenseLine::Dcd => Some(self.port.read_carrier_detect()?),
        })
    }

    fn round_trip(&self) -> Duration {
        match self.family {
            // Four bytes at 9600 8N1 is 4.2 ms of wire time; the board's own
            // handling is the rest. An LCUS word also waits out the gap
            // between its frames.
            RelayFamily::Lcus => {
                Duration::from_millis(10)
                    + LCUS_FRAME_GAP * self.managed.count_ones().saturating_sub(1)
            }
            RelayFamily::KMtronic => Duration::from_millis(10),
            // A CDC ACM port at USB speed. The ASCII is longer and the link is
            // very much faster.
            RelayFamily::Numato => Duration::from_millis(5),
        }
    }

    fn describe(&self) -> String {
        format!("{} relay board on {}", self.family.label(), self.path)
    }
}
