use crate::call::{pack_call, unpack_call};
use crate::crc::crc16_genibus;

pub use crate::crc::crc_ok;

pub const TERMINATOR: u8 = 0x14;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Control {
    pub reply: u8,
    pub compressed: bool,
    pub hint: u8,
}

impl Control {
    pub const fn byte(self, last: bool, full: bool) -> u8 {
        (self.reply << 6) | ((self.compressed as u8) << 5) | ((self.hint & 7) << 2) | ((last as u8) << 1) | full as u8
    }

    pub const fn from_byte(b: u8) -> (Self, bool, bool) {
        (Self { reply: b >> 6, compressed: b & 0x20 != 0, hint: (b >> 2) & 7 }, b & 2 != 0, b & 1 != 0)
    }
}

pub fn term_marker(caller: &[u8]) -> u8 {
    match (crc16_genibus(caller) >> 8) as u8 {
        0x00 | TERMINATOR => 0x01,
        m => m,
    }
}

pub const fn payload_capacity(frame_len: usize, last: bool) -> usize {
    frame_len.saturating_sub(3 + last as usize)
}

pub fn build_frame(frame_len: usize, data: &[u8], ctrl: Control, last: Option<u8>, marker: u8) -> Vec<u8> {
    let room = payload_capacity(frame_len, last.is_some());
    let full = data.len() >= room;
    let mut out = Vec::with_capacity(frame_len);
    out.extend_from_slice(&data[..data.len().min(room)]);
    if !full {
        out.push(TERMINATOR);
        if out.len() < room {
            out.push(marker);
        }
        out.resize(room, 0);
    }
    out.extend(last);
    out.push(ctrl.byte(last.is_some(), full));
    let crc = crc16_genibus(&out);
    out.extend(crc.to_be_bytes());
    out
}

pub fn header_payload(call: &[u8], flags: u8) -> [u8; 8] {
    let p = pack_call(call);
    [p[0], p[1], p[2], p[3], p[4], p[5], 0, flags]
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub call: String,
    pub valid: bool,
    pub registered: bool,
    pub comp: u8,
    pub reply: u8,
    pub offer: u8,
}

impl Header {
    fn flags(&self) -> u8 {
        u8::from(self.valid) << 7 | (self.comp & 3) << 1 | u8::from(self.registered)
    }

    pub fn to_frame(&self, frame_len: usize, hint: u8, marker: u8) -> Vec<u8> {
        let ctrl = Control { reply: self.reply, compressed: false, hint };
        build_frame(frame_len, &header_payload(self.call.as_bytes(), self.flags()), ctrl, Some(self.offer), marker)
    }

    pub fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < 11 {
            return None;
        }
        let (ctrl, last, _) = Control::from_byte(b[b.len() - 3]);
        if !last || ctrl.compressed || b[6] != 0 {
            return None;
        }
        let flags = b[7];
        Some(Self {
            call: unpack_call(&b[..6]).unwrap_or_default(),
            valid: flags & 0x80 != 0,
            registered: flags & 1 != 0,
            comp: flags >> 1 & 3,
            reply: ctrl.reply,
            offer: b[b.len() - 4],
        })
    }
}
