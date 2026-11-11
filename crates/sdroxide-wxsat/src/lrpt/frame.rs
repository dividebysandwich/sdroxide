//! From a stream of decoded bits to image packets: the frame sync marker, the
//! CCSDS randomiser, Reed-Solomon, the VCDU and M-PDU headers, and the source
//! packets that run across frames.
//!
//! The frame is 1024 bytes: the marker `1ACFFC1D`, then 1020 bytes that are
//! four interleaved Reed-Solomon (255,223) code words, all XORed with the CCSDS
//! pseudo-random sequence. 892 bytes remain once the 128 parity bytes are gone:
//! a 6-byte VCDU header, 2 bytes of insert zone, the 2-byte M-PDU header (an
//! 11-bit pointer to the first packet header), then 882 bytes of packets.

use std::collections::HashMap;

use crate::rs255::Rs;

pub const ASM: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];
pub const FRAME_BYTES: usize = 1024;
const FRAME_BITS: usize = FRAME_BYTES * 8;
/// Bytes of a frame after the marker, and of data once parity is removed.
const BODY: usize = FRAME_BYTES - 4;
pub const DATA_BYTES: usize = 892;
/// Bit errors tolerated in the marker: when searching, and when checking it
/// where it should be.
const SEARCH_ERRORS: u32 = 4;
const VERIFY_ERRORS: u32 = 10;
/// Frames in a row without the marker in its place before the sync is dropped.
const MAX_MISSES: u8 = 3;
/// "No packet starts in this frame" in the M-PDU pointer.
const NO_HEADER: u16 = 0x7FF;

/// The CCSDS randomiser, `x^8 + x^7 + x^5 + x^3 + 1` from all ones: period of
/// 255 bits, `FF 48 0E C0 9A 0D 70 BC ...`.
fn pn_sequence() -> [u8; BODY] {
    let mut s = [1u8; 8];
    let mut out = [0u8; BODY];
    for byte in out.iter_mut() {
        for _ in 0..8 {
            *byte = (*byte << 1) | s[0];
            let fb = s[0] ^ s[3] ^ s[5] ^ s[7];
            s.rotate_left(1);
            s[7] = fb;
        }
    }
    out
}

/// A frame as it came off the air: marker included, polarity already fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub bytes: Vec<u8>,
}

/// Finds the marker in a bit stream and cuts the stream into frames.
///
/// A marker found by chance in noise would otherwise cost a frame's time of
/// not looking: the first frame after a find is checked by the marker of the
/// next one, and a find that is not followed is dropped at once.
#[derive(Default)]
pub struct FrameSync {
    sr: u32,
    collecting: bool,
    inverted: bool,
    /// The marker was seen again where it should be at least once.
    confirmed: bool,
    misses: u8,
    buf: Vec<u8>,
    nbits: usize,
}

impl FrameSync {
    pub fn new() -> Self {
        Self::default()
    }

    /// True while frames are being followed at their regular place.
    pub fn locked(&self) -> bool {
        self.collecting && self.confirmed
    }

    /// Forget the sync and look for the marker again.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn start(&mut self, inverted: bool) {
        self.collecting = true;
        self.inverted = inverted;
        self.confirmed = false;
        self.misses = 0;
        self.buf = ASM.to_vec();
        self.nbits = 32;
    }

    /// Feed one decoded bit (0 or 1); a finished frame comes back when the
    /// last bit of one arrives.
    pub fn push(&mut self, bit: u8) -> Option<Frame> {
        let bit = bit & 1;
        self.sr = (self.sr << 1) | u32::from(bit);
        if !self.collecting {
            let d = (self.sr ^ u32::from_be_bytes(ASM)).count_ones();
            if d <= SEARCH_ERRORS {
                self.start(false);
            } else if 32 - d <= SEARCH_ERRORS {
                self.start(true);
            }
            return None;
        }
        let b = bit ^ u8::from(self.inverted);
        if self.nbits.is_multiple_of(8) {
            self.buf.push(0);
        }
        let last = self.buf.len() - 1;
        self.buf[last] |= b << (7 - self.nbits % 8);
        self.nbits += 1;
        if self.nbits == 32 && self.buf.len() == 4 {
            // The marker of a frame that follows another.
            let m = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
            if (m ^ u32::from_be_bytes(ASM)).count_ones() > VERIFY_ERRORS {
                self.misses += 1;
                if !self.confirmed || self.misses >= MAX_MISSES {
                    self.reset();
                    return None;
                }
            } else {
                self.misses = 0;
                self.confirmed = true;
            }
        }
        if self.nbits < FRAME_BITS {
            return None;
        }
        let frame = Frame { bytes: std::mem::take(&mut self.buf) };
        self.nbits = 0;
        Some(frame)
    }
}

/// What a good frame carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vcdu {
    pub vcid: u8,
    pub counter: u32,
    /// Offset in `zone` of the first packet header, or `None` when no packet
    /// starts in this frame.
    pub first_header: Option<usize>,
    /// The 882 bytes of packet data.
    pub zone: Vec<u8>,
    /// Bytes Reed-Solomon had to repair.
    pub corrected: usize,
}

/// Derandomiser and Reed-Solomon, frame to VCDU.
pub struct FrameDecoder {
    rs: Rs,
    pn: [u8; BODY],
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self { rs: Rs::new(), pn: pn_sequence() }
    }

    /// `None` when a code word has more errors than Reed-Solomon can repair.
    pub fn decode(&self, frame: &Frame) -> Option<Vcdu> {
        if frame.bytes.len() != FRAME_BYTES {
            return None;
        }
        let mut body = [0u8; BODY];
        for (o, (&b, &p)) in body.iter_mut().zip(frame.bytes[4..].iter().zip(self.pn.iter())) {
            *o = b ^ p;
        }
        let mut corrected = 0;
        for j in 0..4 {
            let mut cw: Vec<u8> = (0..255).map(|i| body[i * 4 + j]).collect();
            corrected += self.rs.correct(&mut cw)?;
            for (i, &v) in cw.iter().enumerate() {
                body[i * 4 + j] = v;
            }
        }
        let v = &body[..DATA_BYTES];
        let fhp = (u16::from(v[8] & 7) << 8) | u16::from(v[9]);
        let zone = v[10..].to_vec();
        Some(Vcdu {
            vcid: v[1] & 0x3F,
            counter: u32::from_be_bytes([0, v[2], v[3], v[4]]),
            first_header: (fhp != NO_HEADER && usize::from(fhp) < zone.len())
                .then_some(usize::from(fhp)),
            zone,
            corrected,
        })
    }
}

#[derive(Default)]
struct Stream {
    last: Option<u32>,
    synced: bool,
    buf: Vec<u8>,
}

/// Puts the packets back together where they run from one frame to the next.
#[derive(Default)]
pub struct PacketAssembler {
    streams: HashMap<u8, Stream>,
}

impl PacketAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a frame; whole source packets (header included) come back. A gap in
    /// the frame counter drops what was half built, and nothing is built again
    /// until a frame says where a packet starts.
    pub fn push(&mut self, v: &Vcdu) -> Vec<Vec<u8>> {
        let s = self.streams.entry(v.vcid).or_default();
        if s.last.is_some_and(|l| v.counter != (l + 1) & 0xFF_FFFF) {
            s.buf.clear();
            s.synced = false;
        }
        s.last = Some(v.counter);
        if !s.synced {
            let Some(at) = v.first_header else { return Vec::new() };
            s.buf = v.zone[at..].to_vec();
            s.synced = true;
        } else {
            s.buf.extend_from_slice(&v.zone);
        }
        let mut out = Vec::new();
        while s.buf.len() >= 6 {
            let total = 6 + usize::from(u16::from_be_bytes([s.buf[4], s.buf[5]])) + 1;
            if s.buf.len() < total {
                break;
            }
            out.push(s.buf.drain(..total).collect());
        }
        out
    }
}

/// A transmitter for tests: data to frame bytes, the way the satellite makes
/// them. Public so the demodulator's tests can build a whole signal.
pub mod synth {
    use super::*;

    /// 892 data bytes to a 1024-byte frame (marker, randomised, parity).
    pub fn frame_bytes(data: &[u8]) -> Vec<u8> {
        assert_eq!(data.len(), DATA_BYTES);
        let rs = Rs::new();
        let mut body = [0u8; BODY];
        for j in 0..4 {
            let msg: Vec<u8> = (0..223).map(|i| data[i * 4 + j]).collect();
            let parity = rs.parity(&msg);
            for (i, &v) in msg.iter().chain(parity.iter()).enumerate() {
                body[i * 4 + j] = v;
            }
        }
        let pn = pn_sequence();
        let mut out = ASM.to_vec();
        out.extend(body.iter().zip(pn.iter()).map(|(&b, &p)| b ^ p));
        out
    }

    /// The data of a frame: VCDU header (6 bytes), insert zone (2), M-PDU
    /// header (2, the pointer to the first packet header), then the zone.
    pub fn vcdu_data(vcid: u8, counter: u32, first_header: Option<usize>, zone: &[u8]) -> Vec<u8> {
        assert!(zone.len() <= DATA_BYTES - 10);
        let fhp = first_header.map_or(NO_HEADER, |h| h as u16);
        let mut d = vec![0x40, vcid & 0x3F];
        d.extend_from_slice(&counter.to_be_bytes()[1..]);
        d.push(0); // signalling
        d.extend_from_slice(&[0, 0]); // insert zone
        d.push((fhp >> 8) as u8 & 7);
        d.push(fhp as u8);
        d.extend_from_slice(zone);
        d.resize(DATA_BYTES, 0);
        d
    }

    pub fn bits(bytes: &[u8]) -> Vec<u8> {
        bytes.iter().flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(seed: u32) -> Vec<u8> {
        let mut x = seed.wrapping_mul(2654435761).wrapping_add(1);
        (0..DATA_BYTES)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8
            })
            .collect()
    }

    fn run(bits: &[u8]) -> Vec<Frame> {
        let mut s = FrameSync::new();
        bits.iter().filter_map(|&b| s.push(b)).collect()
    }

    #[test]
    fn the_randomiser_starts_the_way_ccsds_says() {
        let pn = pn_sequence();
        assert_eq!(&pn[..8], &[0xFF, 0x48, 0x0E, 0xC0, 0x9A, 0x0D, 0x70, 0xBC]);
    }

    #[test]
    fn frames_survive_the_sync_the_randomiser_and_the_code() {
        let dec = FrameDecoder::new();
        let datas: Vec<Vec<u8>> = (0..4).map(payload).collect();
        let mut bits = synth::bits(&[0x55; 37]); // noise before the first marker
        for d in &datas {
            bits.extend(synth::bits(&synth::frame_bytes(d)));
        }
        let frames = run(&bits);
        assert!(frames.len() >= 3, "{} frames", frames.len());
        for (f, d) in frames.iter().zip(&datas) {
            let v = dec.decode(f).expect("clean frame");
            assert_eq!(v.corrected, 0);
            assert_eq!(&d[10..], &v.zone[..]);
        }
    }

    #[test]
    fn the_headers_come_back_as_they_were_written() {
        let dec = FrameDecoder::new();
        let zone = vec![0xC3u8; 882];
        let f = synth::frame_bytes(&synth::vcdu_data(5, 0x09C015, Some(144), &zone));
        let v = dec.decode(&Frame { bytes: f }).unwrap();
        assert_eq!((v.vcid, v.counter, v.first_header), (5, 0x09C015, Some(144)));
        assert_eq!(v.zone, zone);
        let f = synth::frame_bytes(&synth::vcdu_data(5, 1, None, &zone));
        assert_eq!(dec.decode(&Frame { bytes: f }).unwrap().first_header, None);
    }

    #[test]
    fn an_inverted_stream_is_taken_as_it_is() {
        let dec = FrameDecoder::new();
        let d = payload(9);
        let mut bits = Vec::new();
        for _ in 0..3 {
            bits.extend(synth::bits(&synth::frame_bytes(&d)));
        }
        let inv: Vec<u8> = bits.iter().map(|b| b ^ 1).collect();
        let frames = run(&inv);
        assert!(frames.len() >= 2);
        assert_eq!(&dec.decode(&frames[0]).unwrap().zone[..], &d[10..]);
    }

    #[test]
    fn reed_solomon_repairs_what_the_channel_broke_and_refuses_too_much() {
        let dec = FrameDecoder::new();
        let d = payload(5);
        let mut f = synth::frame_bytes(&d);
        // 12 wrong bytes in the first code word (every fourth byte), 3 in the others.
        for k in 0..12 {
            f[4 + k * 4 * 5] ^= 0xA5;
        }
        for k in 0..3 {
            f[4 + 1 + k * 40] ^= 0x11;
        }
        let v = dec.decode(&Frame { bytes: f.clone() }).expect("repairable");
        assert_eq!(&v.zone[..], &d[10..]);
        assert!(v.corrected >= 15);
        // 40 wrong bytes in one word is past what 32 parity bytes can mend.
        for k in 0..40 {
            f[4 + k * 4 * 3] ^= 0xFF;
        }
        assert!(dec.decode(&Frame { bytes: f }).is_none());
    }

    #[test]
    fn a_marker_found_in_noise_is_dropped_at_once() {
        // The marker, then noise: the frame that follows it is emitted (RS will
        // refuse it) but the sync is not kept.
        let mut bits = synth::bits(&ASM);
        let mut x = 7u32;
        for _ in 0..(8192 * 2) {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            bits.push(((x >> 20) & 1) as u8);
        }
        let mut s = FrameSync::new();
        let n = bits.iter().filter_map(|&b| s.push(b)).count();
        assert!(n <= 1, "{n} frames from noise");
        assert!(!s.locked());
    }

    #[test]
    fn packets_are_put_back_together_across_frames() {
        // Three packets of 400, 700 and 120 bytes run over 882-byte zones.
        let pk = |len: usize, tag: u8| {
            let mut p = vec![0x08, 0x40, 0xC0, 0, 0, 0];
            let l = (len - 6 - 1) as u16;
            p[4..6].copy_from_slice(&l.to_be_bytes());
            p.extend((0..len - 6).map(|i| tag.wrapping_add(i as u8)));
            p
        };
        let packets = [pk(400, 1), pk(700, 2), pk(120, 3), pk(500, 4)];
        let stream: Vec<u8> = packets.concat();
        let mut zones: Vec<&[u8]> = stream.chunks(882).collect();
        let mut asm = PacketAssembler::new();
        let mut got = Vec::new();
        // Header offsets: 0 in the first, then where the next packet starts.
        let starts = [0usize, 400, 1100, 1220];
        for (n, z) in zones.drain(..).enumerate() {
            let lo = n * 882;
            let first = starts.iter().find(|&&s| s >= lo && s < lo + 882).map(|s| s - lo);
            let v = Vcdu {
                vcid: 5,
                counter: 100 + n as u32,
                first_header: first,
                zone: z.to_vec(),
                corrected: 0,
            };
            got.extend(asm.push(&v));
        }
        // The last packet (500 bytes from 1220) is in two frames; with 1720 bytes
        // in all it ends inside the second one.
        assert_eq!(got.len(), 4);
        for (g, p) in got.iter().zip(&packets) {
            assert_eq!(g, p);
        }
    }

    #[test]
    fn a_gap_in_the_counter_drops_the_half_built_packet() {
        let long = {
            let mut p = vec![0x08, 0x40, 0xC0, 0, 0x03, 0xE7]; // 1000 bytes of data
            p.extend(std::iter::repeat_n(7u8, 1000));
            p
        };
        let mut asm = PacketAssembler::new();
        let first = Vcdu {
            vcid: 5,
            counter: 10,
            first_header: Some(0),
            zone: long[..882].to_vec(),
            corrected: 0,
        };
        assert!(asm.push(&first).is_empty());
        // Frame 11 is lost; 12 holds the rest but names no start.
        let third =
            Vcdu { vcid: 5, counter: 12, first_header: None, zone: vec![7u8; 882], corrected: 0 };
        assert!(asm.push(&third).is_empty(), "nothing is built across a hole");
    }
}
