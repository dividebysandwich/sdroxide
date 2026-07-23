//! Hardware I/O layer: SoapySDR device handling and IQ sources.
//!
//! NATIVE ONLY — this crate links the SoapySDR C library and must never be a
//! dependency of any wasm-targeted crate.

#[cfg(feature = "soapy")]
mod device;
pub mod engine;
mod error;
mod source;

#[cfg(feature = "soapy")]
pub use device::{DeviceInfo, SoapyDevice, SoapyRxSource, enumerate_devices};
pub use engine::{
    AudioParams, EngineConfig, EngineHandles, EngineSwap, MicParams, ReopenFn,
    start as start_engine,
};
pub use error::RadioError;
pub use source::{ControlUpdate, FileSource, IqSource, SigGenSource};

// Re-exported so frontends can name handle types without direct deps.
pub use crossbeam_channel;
pub use rtrb;
pub use triple_buffer;

pub type Complex32 = num_complex::Complex<f32>;
pub type Result<T> = std::result::Result<T, RadioError>;
