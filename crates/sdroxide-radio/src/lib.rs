//! Hardware I/O layer: SoapySDR device handling and IQ sources.
//!
//! NATIVE ONLY — this crate links the SoapySDR C library and must never be a
//! dependency of any wasm-targeted crate.

pub mod audio_wav;
#[cfg(feature = "soapy")]
mod device;
pub mod engine;
mod error;
mod image_store;
pub mod iq_wav;
pub mod playback;
mod recorder;
pub mod scanner;
mod source;
mod tr_switch;
mod tx_gate;
mod voice;

#[cfg(feature = "soapy")]
pub use device::{DeviceInfo, SoapyDevice, SoapyRxSource, enumerate_devices};
pub use engine::{
    AudioParams, EngineConfig, EngineHandles, EngineSwap, MicParams, ReopenFn,
    start as start_engine,
};
pub use error::RadioError;
pub use source::{
    ControlUpdate, ConvertedSource, ConverterPlan, ConverterStep, DC_BLOCK_HZ, FileSource,
    IqSource, SigGenSource, converted_caps, converter_open_hz, lo_offset_for, override_caps_ranges,
    plan_caps, shift_caps,
};
pub use tr_switch::TrSwitch;
pub use tx_gate::{RadeWatch, StoreSync, TxGate};

// Re-exported so frontends can name handle types without direct deps.
pub use crossbeam_channel;
pub use rtrb;
pub use triple_buffer;

pub type Complex32 = num_complex::Complex<f32>;
pub type Result<T> = std::result::Result<T, RadioError>;
