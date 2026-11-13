//! Native RTL2832U (RTL-SDR) driver — pure Rust, no libusb and no libSoapySDR.
//!
//! The dongle is reached directly over USB with [`nusb`]: vendor control
//! transfers on endpoint 0 program the demodulator and, through its I2C
//! repeater, the tuner; queued bulk transfers on endpoint `0x81` carry the
//! 8-bit I/Q stream. One blocking thread owns the device and pushes interleaved
//! `f32` into an [`rtrb`] ring, exactly like the `sdroxide-hpsdr` and
//! `sdroxide-tci` backends.
//!
//! The same dongle on *another* machine, published with `rtl_tcp`, is reached
//! by [`RtlSdrHandle::connect`] instead: the far end owns the registers, so
//! that path speaks the five-byte command protocol rather than USB, and hands
//! back the identical handle. See [`tcp`].
//!
//! Supported tuners: R820T, R820T2 and R828D — every RTL-SDR Blog V3/V4 and
//! effectively all generic DVB-T sticks still in circulation. The legacy
//! E4000/FC0012/FC0013/FC2580 parts are deliberately not implemented; the probe
//! fails with the chip named so the operator can fall back to SoapySDR.
//!
//! # Provenance
//!
//! The register sequences, the R82xx initialisation array and the gain tables
//! are transcribed from osmocom `librtlsdr` (`src/librtlsdr.c`,
//! `src/tuner_r82xx.c`), and the Blog V4 branches from `rtlsdrblog/rtl-sdr-blog`
//! — both GPL-2.0-or-later, compatible with this workspace's GPL-3.0-or-later.
//! Each transcribed module names its upstream file in its header. Do not mix
//! forks: the two differ in `r82xx_set_bw`, the gain tables and the PLL refdiv
//! branch, and blending them produces a radio that is subtly deaf.

mod device;
mod error;
mod handle;
mod regs;
mod rtl2832;
mod stream;
pub mod tcp;
mod tuner;
mod usb;

pub use device::{DIRECT_SAMPLING_TOP_HZ, Device, TUNER_MAX_HZ, TUNER_MIN_HZ};
pub use error::{Error, Result};
pub use handle::RtlSdrHandle;
pub use sdroxide_r82xx::{STAGE_MAX_INDEX, Stage, ladder_indices, stage_db, stage_index_for};
pub use usb::{Eeprom, list, parse_eeprom};
