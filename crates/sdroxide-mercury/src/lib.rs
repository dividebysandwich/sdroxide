//! Mercury for sdroxide: Rhizomatica's HF modem (codec2 OFDM data modes, an
//! MFSK floor and a carousel ARQ), linked in as upstream's daemon library.
//!
//! Mercury is built to be embedded this way (`host/mercury_host.h`): the host
//! owns the radio, hands Mercury 8 kHz audio blocks through
//! `mercury_host_audio_exchange`, and is told when to key. Two things shape
//! the wrapper:
//!
//! - **One engine per process.** Mercury keeps global state, and shutting it
//!   down and starting it again is not something upstream exercises. The
//!   engine is started the first time the mode is chosen and runs until the
//!   program exits; leaving the mode sends it `LISTEN OFF` and stops feeding
//!   it audio.
//! - **Its own TCP ports.** Mercury always opens its TNC ports, and replies to
//!   commands given in-process are dropped. So sdroxide talks to it as a TCP
//!   client on a private loopback port pair ([`MercuryModem`]), and offers the
//!   operator's programs sdroxide's own host ports (`sdroxide-tnc`), which
//!   carry their lines to Mercury's. The build binds Mercury's listeners to
//!   the loopback interface only.

#[cfg(not(mercury_unavailable))]
mod engine;
#[cfg(not(mercury_unavailable))]
mod modem;

#[cfg(not(mercury_unavailable))]
pub use engine::{Engine, start};
#[cfg(not(mercury_unavailable))]
pub use modem::MercuryModem;

/// The release linked in.
pub const VERSION: &str = "1.9.23";

/// Mercury's transmit audio at the default drive peaks well under full scale
/// (OFDM, about 10 dB of crest factor held by its own clipper); measured by
/// `tx_peak_at_default_drive`.
pub const TX_PEAK: f32 = 0.5;

/// Whether this build carries Mercury at all (not under MSVC: upstream is POSIX
/// C and builds for Windows only with MinGW).
pub const fn available() -> bool {
    cfg!(not(mercury_unavailable))
}

/// What the engine is started with. The port and the directory hold for the
/// life of the process: Mercury is started once.
#[derive(Clone, Debug)]
pub struct Config {
    /// Mercury's own command port (data on the next), on 127.0.0.1 only.
    pub private_port: u16,
    /// Where Mercury's generated `mercury.ini` (and its log) are written.
    pub config_dir: std::path::PathBuf,
    /// Silence Mercury puts ahead of each frame while the transmitter
    /// settles, milliseconds.
    pub tx_delay_ms: u16,
}
