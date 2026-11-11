//! The whole LRPT receiver: lane-rate I/Q in, image segments out.
//!
//! The Costas loop locks the constellation on the diagonals but cannot tell
//! which diagonal is which, and the I and Q wires may be crossed: the soft
//! symbols can be read four ways that matter (two more are the first two with
//! every bit inverted, which the frame sync handles). One Viterbi decoder and one
//! frame sync run for each, and the one whose frames pass Reed-Solomon is
//! followed alone until it stops delivering.

use sdroxide_dsp::{Complex32, ConvCode, ViterbiStream};

use super::demod::{QpskDemod, SYMBOL_RATE_HZ, Symbol};
use super::frame::{FrameDecoder, FrameSync, PacketAssembler};
use super::jpeg::{Segment, parse_image_packet};

/// The code of the LRPT downlink: constraint length 7, `0o171`/`0o133` in the
/// usual notation, which is `0x4f`/`0x6d` with the newest bit at the low end.
const CODE: ConvCode = ConvCode { poly_a: 0x4F, poly_b: 0x6D, invert_b: false };
/// Seconds without a good frame before the receiver looks at all four readings again.
const LOSS_S: f64 = 2.0;

struct Pipe {
    conj: bool,
    rot: u8,
    vit: ViterbiStream,
    sync: FrameSync,
}

impl Pipe {
    fn new(conj: bool, rot: u8) -> Self {
        Self { conj, rot, vit: ViterbiStream::new(CODE), sync: FrameSync::new() }
    }

    fn reset(&mut self) {
        self.vit.reset();
        self.sync.reset();
    }

    /// One symbol read this pipe's way; a frame when one completes.
    fn push(&mut self, s: Symbol) -> Option<super::frame::Frame> {
        let (mut i, mut q) = (s.i, if self.conj { -s.q } else { s.q });
        if self.rot == 1 {
            (i, q) = (-q, i);
        }
        // The decoder wants a positive value for a received 1; a negative
        // component is a 1 here.
        let bit = self.vit.push(-i, -q)?;
        self.sync.push(bit)
    }
}

/// What the receiver has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RxStats {
    pub symbols: u64,
    pub frames_ok: u64,
    pub frames_bad: u64,
    pub segments: u64,
    /// Bytes Reed-Solomon repaired in the good frames.
    pub bytes_repaired: u64,
}

pub struct LrptRx {
    demod: QpskDemod,
    pipes: Vec<Pipe>,
    active: Option<usize>,
    decoder: FrameDecoder,
    assembler: PacketAssembler,
    stats: RxStats,
    symbols_since_ok: u64,
    buf: Vec<Symbol>,
}

impl Default for LrptRx {
    fn default() -> Self {
        Self::new()
    }
}

impl LrptRx {
    pub fn new() -> Self {
        let pipes =
            vec![Pipe::new(false, 0), Pipe::new(false, 1), Pipe::new(true, 0), Pipe::new(true, 1)];
        Self {
            demod: QpskDemod::new(),
            pipes,
            active: None,
            decoder: FrameDecoder::new(),
            assembler: PacketAssembler::new(),
            stats: RxStats::default(),
            symbols_since_ok: 0,
            buf: Vec::new(),
        }
    }

    pub fn stats(&self) -> RxStats {
        self.stats
    }

    /// Whether a good frame has come in lately.
    pub fn locked(&self) -> bool {
        self.active.is_some()
    }

    /// The demodulator's own figures: carrier offset in Hz and the distance of
    /// the symbols from their ideal points.
    pub fn demod_figures(&self) -> (f64, f32) {
        (self.demod.carrier_hz(), self.demod.evm())
    }

    /// Feed lane-rate samples ([`super::demod::LRPT_LANE_RATE_HZ`]); the image
    /// segments that come out are returned.
    pub fn process(&mut self, iq: &[Complex32]) -> Vec<Segment> {
        self.buf.clear();
        let mut syms = std::mem::take(&mut self.buf);
        self.demod.process(iq, &mut syms);
        let mut out = Vec::new();
        for &s in &syms {
            self.stats.symbols += 1;
            self.symbols_since_ok += 1;
            let range = match self.active {
                Some(a) => a..a + 1,
                None => 0..self.pipes.len(),
            };
            for k in range {
                let Some(frame) = self.pipes[k].push(s) else { continue };
                match self.decoder.decode(&frame) {
                    Some(v) => {
                        self.stats.frames_ok += 1;
                        self.stats.bytes_repaired += v.corrected as u64;
                        self.symbols_since_ok = 0;
                        if self.active != Some(k) {
                            self.active = Some(k);
                        }
                        for p in self.assembler.push(&v) {
                            if let Some(seg) = parse_image_packet(&p) {
                                self.stats.segments += 1;
                                out.push(seg);
                            }
                        }
                    }
                    None => self.stats.frames_bad += 1,
                }
            }
            if self.active.is_some() && self.symbols_since_ok as f64 > LOSS_S * SYMBOL_RATE_HZ {
                self.active = None;
                self.pipes.iter_mut().for_each(Pipe::reset);
            }
        }
        self.buf = syms;
        out
    }
}
