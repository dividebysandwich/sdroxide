use crate::ax25::{self, AprsType};
use crate::bits::{Bits, Reader};
use crate::cq;
use xara_phy::crc::crc16_genibus;
use xara_phy::hf::ctrl::code::{CQ, KISS, KISS_L1, KISS_L2, KISS_L4};
use xara_phy::hf::ctrl::{Codebook, Fmt, Role};
use xara_phy::hf::frame::{Control, TERMINATOR, build_frame};
use xara_phy::hf::level::{Bandwidth, Record};
use xara_phy::hf::rx::ctrl::CtrlEntry;

pub const BW: Bandwidth = Bandwidth::Hz500;
pub const MAX_FRAGMENTS: usize = 18;
pub const FM_MAX_FRAGMENTS: usize = 10;
pub const RX_BUFFER: usize = 5000;
pub const RX_MAX_FRAGMENTS: usize = 255;

pub fn unproto_codebook() -> Codebook {
    Codebook::new(0, [0; 5], Role::Caller, BW)
}

pub fn unproto_entries(cb: &Codebook) -> Vec<CtrlEntry> {
    Fmt::ALL
        .into_iter()
        .flat_map(|fmt| KISS.into_iter().chain([CQ]).map(move |code| (fmt, code)))
        .map(|(fmt, code)| CtrlEntry { code, fmt, tag: 0, codeword: cb.tx(fmt, code).clone() })
        .collect()
}
pub const MAX_QUEUED_BYTES: usize = 5000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Ax25(ax25::Error),
    TooLong,
}

impl From<ax25::Error> for Error {
    fn from(e: ax25::Error) -> Self {
        Self::Ax25(e)
    }
}

pub fn generic_encode(data: &[u8]) -> Vec<u8> {
    let mut b = Bits::default();
    let seven = data.iter().all(|&c| c <= 0x7F);
    if seven {
        b.push_text7(data);
    } else {
        for &c in data {
            b.push(u64::from(c), 8);
        }
    }
    [vec![0x20 | u8::from(seven)], b.pack()].concat()
}

pub fn generic_decode(buf: &[u8]) -> Vec<u8> {
    let Some((&hdr, body)) = buf.split_first() else { return Vec::new() };
    if hdr & 1 == 0 {
        return body.to_vec();
    }
    Reader::new(&Bits::unpack(body).0).text7().unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Ax25 = 0,
    Ax25Long = 1,
    Generic = 2,
}

impl Command {
    pub const fn byte(self) -> u8 {
        self as u8
    }

    pub const fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::Ax25,
            1 => Self::Ax25Long,
            2 => Self::Generic,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buffer {
    pub bytes: Vec<u8>,
    pub aprs_type: AprsType,
}

pub fn encode(cmd: Command, data: &[u8]) -> Result<Buffer, Error> {
    match cmd {
        Command::Ax25 | Command::Ax25Long => {
            let (bytes, rec) = ax25::encode(data, cmd)?;
            Ok(Buffer { bytes, aprs_type: rec.aprs_type })
        }
        Command::Generic => Ok(Buffer { bytes: generic_encode(data), aprs_type: AprsType::Text }),
    }
}

pub fn decode(buf: &[u8]) -> Option<(Command, Vec<u8>)> {
    match (buf.first()? >> 4) & 3 {
        2 => Some((Command::Generic, generic_decode(buf))),
        0 | 1 => ax25::decode(buf),
        _ => None,
    }
}

const L2: Record = Record::new(2).unwrap();
const L4: Record = Record::new(4).unwrap();

pub const fn code_for_level(level: Record) -> u32 {
    match level.get() {
        1 => KISS_L1,
        2 => KISS_L2,
        _ => KISS_L4,
    }
}

pub const fn level_for_code(code: u32) -> Option<Record> {
    match code {
        KISS_L1 => Some(Record::ONE),
        KISS_L2 => Some(L2),
        KISS_L4 => Some(L4),
        _ => None,
    }
}

pub fn payload_field(level: Record) -> usize {
    if level == cq::RECORD { cq::PAYLOAD } else { BW.level(level).payload() as usize }
}

pub fn frame_len(level: Record) -> usize {
    BW.level(level).frame_bytes() as usize
}

fn frags_needed(total: usize, field: usize) -> usize {
    (total + 1).saturating_sub(field).div_ceil(field - 1) + 1
}

pub fn choose_level(total: usize, aprs_type: AprsType) -> Record {
    let limit = if aprs_type.has_position() { 3 } else { 2 };
    [Record::ONE, L2].into_iter().find(|&l| frags_needed(total, payload_field(l)) <= limit).unwrap_or(L4)
}

pub fn fragment_count(total: usize, level: Record) -> usize {
    count_for_field(total, payload_field(level))
}

pub fn count_for_field(total: usize, field: usize) -> usize {
    if total <= field { 1 } else { frags_needed(total, field) }
}

pub const HF_CRC_MASK: u8 = 0xF0;
pub const FM_CRC_MASK: u8 = 0xF8;

pub fn crc_nibble(buf: &[u8]) -> u8 {
    crc_bits(buf, HF_CRC_MASK)
}

pub fn crc_bits(buf: &[u8], mask: u8) -> u8 {
    (crc16_genibus(buf) >> 8) as u8 & mask
}

pub fn fragments(buf: &Buffer) -> Result<(Record, Vec<Vec<u8>>), Error> {
    let b = &buf.bytes;
    let level = choose_level(b.len(), buf.aprs_type);
    let count = fragment_count(b.len(), level);
    if count > MAX_FRAGMENTS {
        return Err(Error::TooLong);
    }
    Ok((level, split(b, count, payload_field(level))))
}

pub fn split(b: &[u8], count: usize, field: usize) -> Vec<Vec<u8>> {
    split_masked(b, count, field, HF_CRC_MASK)
}

pub fn split_masked(b: &[u8], count: usize, field: usize, crc_mask: u8) -> Vec<Vec<u8>> {
    let Some((&hdr, mut rest)) = b.split_first() else { return Vec::new() };
    let mut out = Vec::with_capacity(count);
    while out.is_empty() || !rest.is_empty() {
        let mut f = vec![hdr | 0x80];
        if out.is_empty() && count > 1 {
            f.push((count as u8).wrapping_sub(2) | crc_bits(b, crc_mask));
        }
        let Some(room) = field.checked_sub(f.len()).filter(|&r| r > 0 || rest.is_empty()) else {
            return Vec::new();
        };
        let n = rest.len().min(room);
        f.extend_from_slice(&rest[..n]);
        rest = &rest[n..];
        if rest.is_empty() {
            f[0] &= 0x7F;
        }
        out.push(f);
    }
    out
}

pub fn frame(field: &[u8], level: Record, ctrl: Control, marker: u8) -> Vec<u8> {
    build_frame(frame_len(level), field, ctrl, None, marker)
}

#[derive(Clone, Debug)]
pub struct Reassembler {
    buf: Vec<u8>,
    nfrag: usize,
    count: u8,
    crc_mask: u8,
    overflow: bool,
}

impl Default for Reassembler {
    fn default() -> Self {
        Self::with_crc_mask(HF_CRC_MASK)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Push {
    Dropped,
    More,
    Done(Vec<u8>),
}

impl Reassembler {
    pub fn with_crc_mask(crc_mask: u8) -> Self {
        Reassembler { buf: Vec::new(), nfrag: 0, count: 0, crc_mask, overflow: false }
    }

    pub fn push(&mut self, f: &[u8], level: Record) -> Push {
        let field = payload_field(level);
        if f.len() != frame_len(level) {
            return Push::Dropped;
        }
        let (_, _, full) = Control::from_byte(f[f.len() - 3]);
        let n = if full { field } else { f[..field].iter().rposition(|&b| b == TERMINATOR).unwrap_or(0) };
        self.join(f, n)
    }

    pub fn join(&mut self, f: &[u8], n: usize) -> Push {
        let Some(d) = f.get(..n).filter(|d| !d.is_empty()) else { return Push::Dropped };
        let more = d[0] & 0x80 != 0;
        if self.overflow {
            self.overflow = more;
            return Push::Dropped;
        }
        let mut start = 1;
        if self.buf.is_empty() {
            if more {
                let Some(&c) = f.get(1) else { return Push::Dropped };
                start = 2;
                self.count = c;
            }
            self.nfrag = 0;
            self.buf.push(d[0]);
        }
        let tail = &d[start.min(n)..];
        if self.buf.len() + tail.len() > RX_BUFFER || self.nfrag >= RX_MAX_FRAGMENTS {
            self.reset();
            self.overflow = more;
            return Push::Dropped;
        }
        self.buf.extend_from_slice(tail);
        self.nfrag += 1;
        if more {
            return Push::More;
        }
        let mut out = std::mem::take(&mut self.buf);
        out[0] = d[0];
        if self.nfrag >= 2 && crc_bits(&out, self.crc_mask) != self.count & self.crc_mask {
            return Push::Dropped;
        }
        Push::Done(out)
    }

    pub fn first(&self) -> Option<u8> {
        self.buf.first().copied()
    }

    pub fn reset(&mut self) {
        self.buf.clear();
        self.nfrag = 0;
        self.overflow = false;
    }
}
