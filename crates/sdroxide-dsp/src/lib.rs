pub mod acars;
mod adc;
pub mod afsk;
mod agc;
mod binaural;
mod cessb;
mod ctcss;
mod cw;
mod ddc;
mod decim;
mod demod;
mod dfnr;
mod diversity;
mod eq;
mod fec;
mod fir;
mod frame48;
mod fsq;
mod fsq_image;
pub mod g3ruh;
mod hell;
mod hell_font;
mod interp;
mod iqcorrect;
mod mfsk;
mod modulator;
mod navtex;
mod nb;
mod nco;
mod nnr;
pub mod noisefloor;
mod notch;
mod nr;
mod nr2;
mod olivia;
mod predistort;
mod psk;
mod rds;
mod resample;
pub mod rifp;
mod rtty;
mod sbnr;
#[macro_use]
mod simd;
mod spectrum;
mod spectrum_paint;
mod sstv;
mod thor;
mod tonegen;
mod wbddc;
mod wbspectrum;
pub mod wefax;
mod window;

pub use adc::AdcMeter;
pub use afsk::{AFSK_TX_PEAK, AfskProfile, AfskRx, AfskTx};
pub use agc::Agc;
pub use binaural::Binaural;
pub use cessb::Cessb;
pub use ctcss::{SubToneDetect, golay23_decode, golay23_encode};
pub use cw::{CwDecoder, CwRx, CwTx, morse_decode, morse_encode, text_duration_s};
pub use ddc::Ddc;
pub use decim::{Decimator, FirDecim, HalfbandDecim, RealFirDecim, lowpass_taps};
pub use demod::{ComplexDcBlock, DcBlock, Demodulator, channel_target, make_demod};
pub use dfnr::DeepFilterNr;
pub use diversity::{Diversity, DiversityMode};
pub use eq::ParametricEq;
pub use fec::{ConvCode, viterbi_soft};
pub use fir::{ComplexFir, RealFir, bandpass_taps};
pub use fsq::{FsqRx, FsqTx};
pub use fsq_image::{FsqImageRx, FsqImageTx, IMG_H as FSQ_IMG_H, IMG_W as FSQ_IMG_W};
pub use g3ruh::{G3RUH_TX_PEAK, G3ruhRx, G3ruhTx, Scrambler};
pub use hell::{HELL_CELL_COLS, HELL_ROWS, HellRx, HellTx, render_columns as hell_columns};
pub use interp::{Duc, HalfbandInterp};
pub use iqcorrect::IqCorrect;
pub use modulator::{Modulator, PACKET_DEVIATION_HZ, SsbMod, make_modulator};
pub use navtex::{
    CharSource as NavtexCharSource, NAVTEX_BAUD, NAVTEX_CENTER_HZ, NAVTEX_SHIFT_HZ, NavtexRx,
};

/// The NAVTEX transmit side, for tests and for anyone checking a receiver.
///
/// There is no NAVTEX transmitter in sdroxide and there will not be — the
/// service is a coast station's — so this is exported as a signal *generator*
/// rather than as part of a mode. It is what `sdroxide-digi`'s framing tests
/// are built on, and what a bench check of the decoder would use.
pub mod navtex_test {
    pub use crate::navtex::{encode_bits, synth};
}
pub use acars::{AcarsEvent, AcarsRx, CENTER_HZ as ACARS_CENTER_HZ};
pub use nb::NoiseBlanker;
pub use nco::Nco;
pub use nnr::NeuralNr;
pub use notch::AutoNotch;
pub use nr::SpectralNr;
pub use nr2::Nr2;
pub use olivia::{OliviaRx, OliviaTx};
pub use predistort::{MAX_CORRECTION as PS_MAX_CORRECTION, PureSignal};
pub use psk::{BpskCore, PskRx, PskTx, VaricodeRx};
pub use rds::{RDS_MIN_RATE, RdsRx};
pub use resample::{ComplexResampler, MonoResampler, StereoResampler};
pub use rifp::{RifpFrame, RifpRx, RifpTx, Tlv as RifpTlv};
pub use rtty::{BaudotRx, RttyRx, RttyTx};
pub use sbnr::SpecBleachNr;
pub use spectrum::SpectrumAnalyzer;
pub use spectrum_paint::{
    BAND_HI_HZ as RF_PAINT_BAND_HI, BAND_LO_HZ as RF_PAINT_BAND_LO, CENTER_HZ as RF_PAINT_CENTER,
    SpectrumPaintTx, TX_PEAK as RF_PAINT_TX_PEAK,
};
pub use sstv::{SstvEvent, SstvRx, SstvTx};
pub use thor::{ThorRx, ThorTx};
pub use tonegen::{BURST_LEVEL, SUB_TONE_LEVEL, SubToneGen, ToneBurst};
pub use wbddc::{WbDdc, clamp_center_hz, reachable_range_hz};
pub use wbspectrum::WideSpectrum;
pub use wefax::{Ioc as WefaxIoc, Lpm as WefaxLpm, WefaxEvent, WefaxRx};
pub use window::blackman_harris;

pub type Complex32 = num_complex::Complex<f32>;

/// A complex buffer seen as the interleaved `[re, im, re, im, …]` floats every
/// backend's ring carries.
///
/// The two are the same bytes: `num_complex::Complex<f32>` is `#[repr(C)]` over
/// two `f32` fields and so has an `f32`'s alignment and exactly twice its size,
/// with no padding to skip. Every backend used to spell the conversion out a
/// sample at a time — a scratch `Vec<f32>`, a loop pushing `re` and `im`, and
/// another loop reading them back into `Complex32` — which is two passes over
/// the whole stream to change nothing but the type. On an RX-888 at 16.2 Msps
/// that was measured at about 6 % of everything the process was doing.
pub fn as_interleaved(z: &[Complex32]) -> &[f32] {
    // SAFETY: `Complex<f32>` is `#[repr(C)] { re: f32, im: f32 }`, so a slice of
    // `n` of them is `2n` consecutive `f32` at the same address and alignment.
    unsafe { std::slice::from_raw_parts(z.as_ptr().cast::<f32>(), z.len() * 2) }
}

/// [`as_interleaved`] for a buffer the caller wants *written* — a device read
/// that fills interleaved floats can fill the engine's complex block directly.
pub fn as_interleaved_mut(z: &mut [Complex32]) -> &mut [f32] {
    // SAFETY: as `as_interleaved`, and the exclusive borrow is preserved.
    unsafe { std::slice::from_raw_parts_mut(z.as_mut_ptr().cast::<f32>(), z.len() * 2) }
}

#[cfg(test)]
mod interleave_tests {
    use super::*;

    /// The layout this is all built on, checked rather than assumed: a wrong
    /// answer here would silently swap I and Q or halve the block length.
    #[test]
    fn a_complex_slice_is_its_own_interleaved_floats() {
        assert_eq!(size_of::<Complex32>(), 2 * size_of::<f32>());
        assert_eq!(align_of::<Complex32>(), align_of::<f32>());

        let mut z = vec![Complex32::new(1.0, 2.0), Complex32::new(3.0, 4.0)];
        assert_eq!(as_interleaved(&z), &[1.0, 2.0, 3.0, 4.0]);

        as_interleaved_mut(&mut z).copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);
        assert_eq!(z, vec![Complex32::new(5.0, 6.0), Complex32::new(7.0, 8.0)]);

        // An empty block must not produce a dangling non-empty slice.
        assert!(as_interleaved(&[]).is_empty());
    }
}
