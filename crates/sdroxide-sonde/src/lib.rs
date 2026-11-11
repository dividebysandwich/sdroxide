//! The radiosonde (Vaisala RS41) decoder: the threading wrapper and the
//! wire-type mapping around `xng-mode-sonde`'s channel decoder. Native-only
//! (runs in the engine's audio-block loop); see [`controller`] for how it is
//! driven and [`tracks`] for how frames become per-sonde flight tracks.
//!
//! The demodulation *is* xng's: a 48 kS/s complex baseband lane centred on the
//! channel (the decoder's own channel rate, so it runs without a second
//! downconverter). This crate owns the worker thread, the bounded I/Q queue
//! and the per-serial track book.

pub mod bank;
pub mod c34;
pub mod cf06;
pub mod controller;
pub mod dfm;
pub mod fsk;
pub mod imet;
pub mod imet54;
pub mod lms6;
pub mod m10;
pub mod m20;
pub mod meisei;
pub mod mk2a;
pub mod model;
pub mod mrz;
pub mod mts01;
pub mod rs255;
pub mod rs41meteo;
pub mod scan;
pub mod tracks;
pub mod weathex;

pub use bank::SondeBank;
pub use controller::SondeController;
pub use model::{ModelDecoder, SondeFix, SondeModel, decoder_for};

/// The lane rate the engine feeds the decoder: the decoder's own channel rate.
pub const SONDE_LANE_RATE_HZ: f64 = xng_mode_sonde::CHANNEL_RATE;
