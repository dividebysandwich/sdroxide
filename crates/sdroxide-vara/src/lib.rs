//! VARA HF for sdroxide, on the vendored xara modem core.
//!
//! [`station`] is xara's own glue between the sans-IO ARQ session and a 48 kHz
//! sample stream; [`tnc`] answers VARA's host command language; [`host`] is
//! who may speak it — an external program on the TCP ports, or an in-process
//! client such as the Winlink window or the operator's panel — one at a time.
//! Everything here is clocked by the samples handed to [`Vara::process`]; the
//! caller owns the audio and the transmitter.

pub mod host;
pub mod station;
pub mod tnc;

pub use host::{Holder, HostMux, LocalClient, VaraHandle};
pub use station::hf::Station;
pub use station::{Link, Modem, Options};
pub use tnc::{Observed, Tnc};
pub use xara_arq::hf::session::{Config, State};
pub use xara_arq::time::{SECOND, Time};
pub use xara_host::response::Response;
pub use xara_phy::hf::level::Bandwidth;

/// The station's sample rate. Every `Time` in the modem counts these.
pub const RATE: f64 = 48_000.0;

/// The whole modem: one HF station behind one host multiplexer.
pub type Vara = Tnc<Station>;

/// Build a modem for `mycall` at `bw`, with no TCP ports open yet.
pub fn new_modem(bw: Bandwidth, mycall: &str, registered: bool, seed: u64) -> Vara {
    let mut cfg = Config::new(bw, mycall);
    cfg.host.registered = registered;
    cfg.seed = seed;
    let opts = Options { drive_db: 0.0, ..Options::default() };
    Tnc::new(Station::new(cfg, opts), HostMux::new())
}
