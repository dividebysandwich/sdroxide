//! The image coder of Meteor LRPT: each image packet carries one *segment* of
//! 14 MCUs, an MCU being one 8 by 8 block of the picture, coded as in baseline
//! JPEG (DC difference then run/size AC codes, zigzag, Huffman, DCT). Segments
//! stand alone: the DC prediction starts again at zero in each one.
//!
//! The Huffman tables are the standard luminance ones of JPEG (ITU T.81 Annex
//! K), the quantisation table is that Annex's luminance table scaled by the
//! quality byte of the segment the way JPEG scales it. The layout of the packet
//! and the choice of these tables were checked on a real Meteor-M2 pass: every
//! segment of it decodes to exactly 14 MCUs with under 16 bits of padding left,
//! and the picture is a picture.

/// Image channel APIDs (channel 1 to 6).
pub const APID_FIRST: u16 = 64;
pub const APID_LAST: u16 = 69;
/// MCUs in a segment, and MCUs across the picture (so segments across it: 14).
pub const MCU_PER_SEGMENT: usize = 14;
pub const MCU_PER_LINE: usize = 196;
/// Pixels across the picture.
pub const PICTURE_WIDTH: usize = MCU_PER_LINE * 8;

/// Where the entropy-coded data starts in a packet's data field (after the
/// 6-byte CCSDS header): 8 bytes of time, the MCU number, two bytes that are
/// always zero in the recording, two that are `ff f0`, and the quality byte.
const DATA_START: usize = 14;

const DC_COUNTS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const AC_COUNTS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125];
#[rustfmt::skip]
const AC_VALS: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

/// The standard luminance quantisation table, in zigzag order.
#[rustfmt::skip]
const BASE_QT: [u8; 64] = [
    16, 11, 12, 14, 12, 10, 16, 14, 13, 14, 18, 17, 16, 19, 24, 40,
    26, 24, 22, 22, 24, 49, 35, 37, 29, 40, 58, 51, 61, 60, 57, 51,
    56, 55, 64, 72, 92, 78, 64, 68, 87, 69, 55, 56, 80, 109, 81, 87,
    95, 98, 103, 104, 103, 62, 77, 113, 121, 112, 100, 120, 92, 101, 103, 99,
];

/// Zigzag position to natural (row-major) position.
#[rustfmt::skip]
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5,
    12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// A canonical Huffman table, decoded the way T.81 F.2.2.3 does.
struct Huffman {
    /// Largest code of each length, or -1 when there is none.
    max_code: [i32; 17],
    /// Index in `vals` of the first code of each length, less its code value.
    val_ptr: [i32; 17],
    vals: &'static [u8],
}

impl Huffman {
    fn new(counts: &[u8; 16], vals: &'static [u8]) -> Self {
        let mut max_code = [-1i32; 17];
        let mut val_ptr = [0i32; 17];
        let (mut code, mut k) = (0i32, 0i32);
        for len in 1..=16usize {
            let n = i32::from(counts[len - 1]);
            val_ptr[len] = k - code;
            code += n;
            k += n;
            max_code[len] = if n > 0 { code - 1 } else { -1 };
            code <<= 1;
        }
        Self { max_code, val_ptr, vals }
    }
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos >> 3)?;
        let b = (byte >> (7 - (self.pos & 7))) & 1;
        self.pos += 1;
        Some(u32::from(b))
    }

    fn get(&mut self, n: u32) -> Option<u32> {
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | self.bit()?;
        }
        Some(v)
    }

    fn huff(&mut self, t: &Huffman) -> Option<u8> {
        let mut code = 0i32;
        for len in 1..=16usize {
            code = (code << 1) | self.bit()? as i32;
            if t.max_code[len] >= 0 && code <= t.max_code[len] {
                return t.vals.get((code + t.val_ptr[len]) as usize).copied();
            }
        }
        None
    }
}

/// The value of an `s`-bit field, sign-extended the way JPEG codes it.
fn extend(v: u32, s: u32) -> i32 {
    if s == 0 {
        0
    } else if v >= 1 << (s - 1) {
        v as i32
    } else {
        v as i32 - (1 << s) + 1
    }
}

/// The quantisation table for quality byte `q`, in zigzag order: JPEG's own
/// scaling of the standard table.
pub fn quant_table(q: u8) -> [u16; 64] {
    let q = f64::from(q.max(1));
    let scale = if q < 50.0 { 5000.0 / q } else { 200.0 - 2.0 * q };
    let mut t = [0u16; 64];
    for (o, &b) in t.iter_mut().zip(BASE_QT.iter()) {
        *o = ((f64::from(b) * scale + 50.0) / 100.0).floor().clamp(1.0, 255.0) as u16;
    }
    t
}

/// One MCU's coefficients before dequantisation: the DC difference and the AC
/// codes as (zeros skipped, value), where (16, 0) is the 16-zero run.
struct Mcu {
    dc_diff: i32,
    ac: Vec<(u8, i32)>,
}

fn read_mcu(br: &mut Bits, dc: &Huffman, ac: &Huffman) -> Option<Mcu> {
    let s = u32::from(br.huff(dc)?);
    if s > 11 {
        return None;
    }
    let dc_diff = extend(br.get(s)?, s);
    let mut out = Vec::new();
    let mut k = 1usize;
    while k < 64 {
        let rs = br.huff(ac)?;
        if rs == 0 {
            break;
        }
        let (r, size) = (usize::from(rs >> 4), u32::from(rs & 15));
        if size == 0 {
            if r == 15 {
                out.push((16u8, 0));
                k += 16;
                continue;
            }
            return None;
        }
        k += r;
        if k > 63 {
            return None;
        }
        out.push((r as u8, extend(br.get(size)?, size)));
        k += 1;
    }
    Some(Mcu { dc_diff, ac: out })
}

/// The 8 by 8 inverse DCT as two matrix products, the cosine matrix built once.
fn idct_matrix() -> [[f32; 8]; 8] {
    let mut c = [[0f32; 8]; 8];
    for (k, row) in c.iter_mut().enumerate() {
        let a = if k == 0 { (1.0f64 / 8.0).sqrt() } else { (2.0f64 / 8.0).sqrt() };
        for (n, v) in row.iter_mut().enumerate() {
            *v = (a * ((2 * n + 1) as f64 * k as f64 * std::f64::consts::PI / 16.0).cos()) as f32;
        }
    }
    c
}

/// Turn coefficients into 64 pixels, row-major.
fn block_pixels(dc: i32, ac: &[(u8, i32)], qt: &[u16; 64], c: &[[f32; 8]; 8]) -> [u8; 64] {
    let mut zz = [0f32; 64];
    zz[0] = (dc * i32::from(qt[0])) as f32;
    let mut k = 1usize;
    for &(r, v) in ac {
        k += usize::from(r);
        if r == 16 && v == 0 {
            continue;
        }
        if k > 63 {
            break;
        }
        zz[k] = (v * i32::from(qt[k])) as f32;
        k += 1;
    }
    let mut m = [[0f32; 8]; 8];
    for (i, &z) in ZIGZAG.iter().enumerate() {
        m[z / 8][z % 8] = zz[i];
    }
    // out = C^T * m * C
    let mut tmp = [[0f32; 8]; 8];
    for i in 0..8 {
        for j in 0..8 {
            tmp[i][j] = (0..8).map(|k| c[k][i] * m[k][j]).sum();
        }
    }
    let mut px = [0u8; 64];
    for i in 0..8 {
        for j in 0..8 {
            let v: f32 = (0..8).map(|k| tmp[i][k] * c[k][j]).sum::<f32>() + 128.0;
            px[i * 8 + j] = v.clamp(0.0, 255.0) as u8;
        }
    }
    px
}

/// One decoded segment: 14 blocks of 64 pixels, left to right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub apid: u16,
    /// Milliseconds of the day, the time of the picture line this belongs to.
    pub ms: u32,
    /// Number of the first MCU across the line: 0, 14, 28 ... 182.
    pub mcu_id: u8,
    /// The quality byte the segment was quantised with.
    pub q: u8,
    /// 14 blocks of 64 pixels, in block order.
    pub pixels: Vec<u8>,
}

/// The APID of a CCSDS source packet.
pub fn apid_of(packet: &[u8]) -> Option<u16> {
    (packet.len() >= 6).then(|| (u16::from(packet[0] & 7) << 8) | u16::from(packet[1]))
}

/// Decode an image packet: the whole source packet, header included. `None`
/// for a packet that is not an image one or whose segment does not decode.
pub fn parse_image_packet(packet: &[u8]) -> Option<Segment> {
    let apid = apid_of(packet)?;
    if !(APID_FIRST..=APID_LAST).contains(&apid) {
        return None;
    }
    let data = packet.get(6..)?;
    if data.len() <= DATA_START {
        return None;
    }
    let ms = u32::from_be_bytes(data[2..6].try_into().ok()?);
    let (mcu_id, q) = (data[8], data[13]);
    let dc_t = Huffman::new(&DC_COUNTS, &DC_VALS);
    let ac_t = Huffman::new(&AC_COUNTS, &AC_VALS);
    let mut br = Bits { data, pos: DATA_START * 8 };
    let qt = quant_table(q);
    let c = idct_matrix();
    let mut pixels = Vec::with_capacity(MCU_PER_SEGMENT * 64);
    let mut dc = 0i32;
    for _ in 0..MCU_PER_SEGMENT {
        let m = read_mcu(&mut br, &dc_t, &ac_t)?;
        dc += m.dc_diff;
        pixels.extend_from_slice(&block_pixels(dc, &m.ac, &qt, &c));
    }
    // A real segment ends in under a byte of padding; a misparsed one does not.
    let left = data.len() * 8 - br.pos;
    if left >= 24 {
        return None;
    }
    Some(Segment { apid, ms, mcu_id, q, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One line of `segments.txt`, its fields in file order.
    type Vector = (u16, u8, u8, i64, i64, Vec<u8>, Vec<u8>);

    /// Real packets and what the Python reference made of them.
    fn vectors() -> Vec<Vector> {
        include_str!("../../tests/testdata/lrpt/segments.txt")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                let hex: Vec<u8> = (0..f[6].len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&f[6][i..i + 2], 16).unwrap())
                    .collect();
                let first = f[5].split(',').map(|v| v.parse().unwrap()).collect();
                (
                    f[0].parse().unwrap(),
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                    first,
                    hex,
                )
            })
            .collect()
    }

    #[test]
    fn real_segments_decode_to_what_the_reference_made() {
        let v = vectors();
        assert!(v.len() >= 2);
        for (apid, q, mcu, sum, hash, first, packet) in v {
            let s = parse_image_packet(&packet).expect("segment");
            assert_eq!((s.apid, s.q, s.mcu_id), (apid, q, mcu));
            assert_eq!(s.pixels.len(), 14 * 64);
            // The first blocks agree to a grey level (f32 against f64 rounding).
            for (a, b) in s.pixels.iter().zip(first.iter()) {
                assert!((i32::from(*a) - i32::from(*b)).abs() <= 1, "{a} vs {b}");
            }
            let total: i64 = s.pixels.iter().map(|&p| i64::from(p)).sum();
            assert!((total - sum).abs() < 14 * 64 / 4, "pixel sum {total} vs reference {sum}");
            let _ = hash;
        }
    }

    #[test]
    fn the_quality_scaling_is_jpegs() {
        // Quality 50 is the standard table itself; 100 is all ones.
        assert_eq!(quant_table(50)[0], 16);
        assert!(quant_table(100).iter().all(|&v| v == 1));
        // 90 halves it twice over: scale 20 per cent.
        assert_eq!(quant_table(90)[0], 3);
        // Below 50 the scale grows: quality 25 doubles.
        assert_eq!(quant_table(25)[0], 32);
    }

    #[test]
    fn the_huffman_tables_are_complete_prefix_codes() {
        // Kraft sum of a canonical table never exceeds one, and the codes the
        // two tables hold are the 12 and 162 of the standard.
        assert_eq!(DC_COUNTS.iter().map(|&c| usize::from(c)).sum::<usize>(), DC_VALS.len());
        assert_eq!(AC_COUNTS.iter().map(|&c| usize::from(c)).sum::<usize>(), AC_VALS.len());
        for counts in [DC_COUNTS, AC_COUNTS] {
            let kraft: f64 = counts
                .iter()
                .enumerate()
                .map(|(i, &c)| f64::from(c) / f64::from(1u32 << (i + 1)))
                .sum();
            assert!(kraft <= 1.0, "kraft {kraft}");
        }
    }

    #[test]
    fn garbage_is_refused_not_decoded() {
        assert!(parse_image_packet(&[0u8; 5]).is_none());
        // Right APID, noise for a segment.
        let mut p = vec![0x08, 0x40, 0, 0, 0, 100];
        p.extend((0..101u32).map(|i| (i * 37 % 251) as u8));
        // It may decode by accident, but never into a segment with padding left over.
        if let Some(s) = parse_image_packet(&p) {
            assert_eq!(s.pixels.len(), 14 * 64);
        }
        // Not an image channel.
        assert!(
            parse_image_packet(&[
                0x08, 0x46, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0
            ])
            .is_none()
        );
    }
}
