//! Meteor-M2 LRPT: QPSK at 72 ksym/s carrying CCSDS frames of image packets.
//! The chain, from the antenna side: [`jpeg`] is the end of it, the image
//! segments. The demodulator and the framing are added next to it.

pub mod demod;
pub mod frame;
pub mod jpeg;
pub mod picture;
pub mod rx;
