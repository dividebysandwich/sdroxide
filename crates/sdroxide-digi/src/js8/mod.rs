//! JS8 — the keyboard/messaging mode built on FT8's 8-FSK waveform.
//!
//! JS8 shares FT8's frame *geometry* (79 symbols: 21 sync, 58 data, three
//! 7-tone Costas blocks at symbols 0/36/72) and almost nothing else. The
//! differences that matter, each transcribed from JS8Call rather than inferred
//! — see `vendor/js8call/PROVENANCE.md` for the citations:
//!
//! | | FT8 | JS8 |
//! |---|---|---|
//! | Costas | one array repeated | two *sets*: Normal repeats one array, the faster submodes use three distinct ones |
//! | Tone map | Gray-coded | **identity** — `genjs8.f90` writes the raw 3-bit index |
//! | FEC | LDPC(174,91) + CRC-14 | LDPC(174,87) + CRC-12, and the CRC is XORed with 42 |
//! | Codeword | systematic prefix | parity first, message at `cw[87..174]` |
//! | Payload | 77 bits of packed callsigns | 72 bits + a 3-bit frame type, framed by a varicode layer |
//! | Timing | one 15 s slot | four submodes from 6 s to 30 s, each with its own start delay |
//!
//! Any one of those, got wrong, produces a mode that round-trips against itself
//! perfectly and is deaf and mute on the air. That is why the tests in this
//! subtree lean on values captured from JS8Call itself wherever one exists.
//!
//! **Licensing:** everything here is derived from JS8Call (GPL-3.0-or-later),
//! which is why it lives in `sdroxide-digi` — the one crate already carrying
//! that obligation — and not in `sdroxide-dsp`, which the wasm client links.

pub mod assembler;
pub mod autorule;
pub mod crc12;
pub mod decode;
pub mod directed;
pub mod frame;
pub mod huff;
pub mod jsc;
pub mod ldpc;
pub mod ldpc_tables;
pub mod message;
pub mod modem;
pub mod osd;
pub mod protocol;
pub mod varicode;
