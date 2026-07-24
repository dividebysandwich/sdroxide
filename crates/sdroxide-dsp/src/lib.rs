//! DSP building blocks: spectrum analysis, and (in later milestones) DDC,
//! demodulators, AGC, and modulators.
//!
//! Pure Rust, no native dependencies — kept wasm-clean by policy.

mod agc;
mod ddc;
mod decim;
mod demod;
mod fec;
mod fir;
mod fsq;
mod fsq_image;
mod interp;
mod mfsk;
mod modulator;
mod nb;
mod nco;
mod notch;
mod nr;
mod olivia;
mod psk;
mod rtty;
mod resample;
mod spectrum;
mod spectrum_paint;
mod sstv;
mod thor;
mod window;

pub use agc::Agc;
pub use ddc::Ddc;
pub use decim::{FirDecim, HalfbandDecim, lowpass_taps};
pub use demod::{DcBlock, Demodulator, channel_target, make_demod};
pub use fir::{ComplexFir, RealFir, bandpass_taps};
pub use fsq::{FsqRx, FsqTx};
pub use fsq_image::{FsqImageRx, FsqImageTx, IMG_H as FSQ_IMG_H, IMG_W as FSQ_IMG_W};
pub use interp::{Duc, HalfbandInterp};
pub use modulator::{Modulator, make_modulator};
pub use nb::NoiseBlanker;
pub use nco::Nco;
pub use notch::AutoNotch;
pub use nr::SpectralNr;
pub use olivia::{OliviaRx, OliviaTx};
pub use psk::{BpskCore, PskRx, PskTx, VaricodeRx};
pub use rtty::{BaudotRx, RttyRx, RttyTx};
pub use resample::{ComplexResampler, MonoResampler};
pub use spectrum::SpectrumAnalyzer;
pub use spectrum_paint::{
    BAND_HI_HZ as RF_PAINT_BAND_HI, BAND_LO_HZ as RF_PAINT_BAND_LO, CENTER_HZ as RF_PAINT_CENTER,
    SpectrumPaintTx,
};
pub use sstv::{SstvEvent, SstvRx, SstvTx};
pub use thor::{ThorRx, ThorTx};
pub use window::blackman_harris;

pub type Complex32 = num_complex::Complex<f32>;
