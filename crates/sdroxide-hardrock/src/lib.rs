//! NATIVE ONLY — links `serialport`; must never be a dependency of any
//! wasm-targeted crate.
//!
//! Monitor and control a HobbyPCB Hardrock amplifier over its USB (or ACC)
//! serial port.
//!
//! [`protocol`] is the command set with no I/O. [`Amp`] puts it on a port: one
//! command, one answer, with a timeout. A caller that wants a live display polls
//! [`Amp::status`] (`HRST`), which carries every meter in one round trip.
//!
//! Written against the HR500 manual. The HR50 shares the command set, and the
//! HR100 is expected to; neither has been checked against a real unit yet.

pub mod link;
pub mod protocol;

pub use link::AmpLink;

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

pub use protocol::{
    AtuParams, Band, Command, Framer, KeyMode, Lights, Meter, Response, Status, TempScale,
    TuneResult,
};

/// Baud rates the amplifier offers on its USB port, most likely first. 19200 is
/// the factory default the manual's PowerSDR setup assumes.
pub const BAUD_RATES: [u32; 4] = [19200, 38400, 9600, 4800];

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Serial(serialport::Error),
    /// Nothing that answers the command arrived in time.
    Timeout(Command),
    /// No port and rate answered `HRAA;`.
    NotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Serial(e) => write!(f, "serial port: {e}"),
            Error::Timeout(c) => write!(f, "no answer to {}", c.encode()),
            Error::NotFound => {
                f.write_str("no Hardrock amplifier answered on any port or baud rate")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serialport::Error> for Error {
    fn from(e: serialport::Error) -> Self {
        Error::Serial(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// An amplifier on a byte stream — a serial port in use, anything `Read +
/// Write` in tests.
pub struct Amp<P: Read + Write> {
    port: P,
    framer: Framer,
    timeout: Duration,
    /// Frames that arrived while waiting for a different answer — the amplifier
    /// echoes CAT it overhears, and a reply can trail a timed-out query.
    pub unsolicited: Vec<Response>,
}

impl Amp<Box<dyn serialport::SerialPort>> {
    /// Open `path` (`COM5`, `/dev/ttyUSB0`) at `baud` and check that a Hardrock
    /// answers.
    pub fn open(path: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(path, baud).timeout(Duration::from_millis(50)).open()?;
        let mut amp = Amp::new(port);
        amp.query(&Command::Ping)?;
        Ok(amp)
    }

    /// Try every serial port at every rate in [`BAUD_RATES`] until one answers
    /// `HRAA;` — what the HR500 bootloader app's SCAN button does.
    ///
    /// Only sends `HRAA;`, which changes nothing, so it is safe on a port that
    /// turns out to be a radio.
    pub fn find() -> Result<(String, u32, Self)> {
        for info in serialport::available_ports()? {
            for baud in BAUD_RATES {
                if let Ok(amp) = Amp::open(&info.port_name, baud) {
                    return Ok((info.port_name, baud, amp));
                }
            }
        }
        Err(Error::NotFound)
    }
}

impl<P: Read + Write> Amp<P> {
    pub fn new(port: P) -> Self {
        Amp {
            port,
            framer: Framer::default(),
            timeout: Duration::from_millis(500),
            unsolicited: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send a command that gets no answer.
    pub fn send(&mut self, cmd: &Command) -> Result<()> {
        self.port.write_all(cmd.encode().as_bytes())?;
        self.port.flush()?;
        Ok(())
    }

    /// Send a command and wait for the frame that answers it. A SET with no
    /// answer returns [`Response::Ack`] once written.
    pub fn query(&mut self, cmd: &Command) -> Result<Response> {
        self.send(cmd)?;
        let Some(prefix) = cmd.reply_prefix() else {
            return Ok(Response::Ack);
        };
        let deadline = Instant::now() + self.timeout;
        let mut buf = [0u8; 128];
        while Instant::now() < deadline {
            let n = match self.port.read(&mut buf) {
                Ok(n) => n,
                Err(e)
                    if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) =>
                {
                    0
                }
                Err(e) => return Err(e.into()),
            };
            for frame in self.framer.push(&buf[..n]) {
                let answers = frame.to_ascii_uppercase().starts_with(prefix)
                    // HRTR's answer as the manual prints it (see protocol.rs).
                    || (prefix == "HRTR" && frame.to_ascii_uppercase().starts_with("HRTT"));
                let r = Response::parse(&frame);
                if answers {
                    return Ok(r);
                }
                self.unsolicited.push(r);
            }
        }
        Err(Error::Timeout(cmd.clone()))
    }

    /// Collect every frame that arrives in `window`, answering nothing in
    /// particular — for `raw` and for watching what the amp says on its own.
    pub fn listen(&mut self, window: Duration) -> Result<Vec<Response>> {
        let deadline = Instant::now() + window;
        let mut buf = [0u8; 128];
        let mut out = std::mem::take(&mut self.unsolicited);
        while Instant::now() < deadline {
            let n = match self.port.read(&mut buf) {
                Ok(n) => n,
                Err(e)
                    if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) =>
                {
                    0
                }
                Err(e) => return Err(e.into()),
            };
            out.extend(self.framer.push(&buf[..n]).iter().map(|f| Response::parse(f)));
        }
        Ok(out)
    }

    /// Every reading at once.
    pub fn status(&mut self) -> Result<Status> {
        match self.query(&Command::Status)? {
            Response::Status(s) => Ok(s),
            _ => Err(Error::Timeout(Command::Status)),
        }
    }

    pub fn set_band(&mut self, band: Band) -> Result<()> {
        self.send(&Command::SetBand(band))
    }

    /// Tell the amplifier the radio's frequency; it picks the band itself.
    pub fn set_frequency(&mut self, hz: u64) -> Result<()> {
        self.send(&Command::SetFrequency(hz))
    }

    pub fn set_mode(&mut self, mode: KeyMode) -> Result<()> {
        self.send(&Command::SetMode(mode))
    }

    pub fn set_antenna(&mut self, n: u8) -> Result<()> {
        self.send(&Command::SetAntenna(n))
    }

    pub fn into_inner(self) -> P {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A fake amplifier: records what was written and plays back canned bytes.
    struct Fake {
        written: Vec<u8>,
        replies: VecDeque<Vec<u8>>,
    }

    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.replies.pop_front() {
                Some(r) => {
                    buf[..r.len()].copy_from_slice(&r);
                    Ok(r.len())
                }
                None => Err(io::ErrorKind::TimedOut.into()),
            }
        }
    }

    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn fake(replies: &[&str]) -> Amp<Fake> {
        Amp::new(Fake {
            written: Vec::new(),
            replies: replies.iter().map(|r| r.as_bytes().to_vec()).collect(),
        })
        .with_timeout(Duration::from_millis(20))
    }

    #[test]
    fn status_skips_unrelated_frames() {
        let mut amp = fake(&["FA00007074000;HRST-100-002-0", "20-011-058-008-30-1-07-1-;"]);
        let s = amp.status().unwrap();
        assert_eq!(s.forward_w, 100.0);
        assert_eq!(s.band, Some(Band::M40));
        assert_eq!(amp.unsolicited, vec![Response::Frequency(7_074_000)]);
        assert_eq!(amp.into_inner().written, b"HRST;");
    }

    #[test]
    fn silence_is_a_timeout() {
        let mut amp = fake(&[]);
        assert!(matches!(amp.query(&Command::Ping), Err(Error::Timeout(Command::Ping))));
    }

    #[test]
    fn sets_are_written_and_not_waited_on() {
        let mut amp = fake(&[]);
        amp.set_band(Band::M20).unwrap();
        amp.set_frequency(14_074_000).unwrap();
        amp.set_mode(KeyMode::Off).unwrap();
        assert_eq!(amp.into_inner().written, b"HRBN5;FA00014074000;HRMD0;");
    }
}
