use crate::ax25;
use crate::kiss::{self, Command};
use xara_phy::fm::frame::{Trailer, build_data, parse_data, payload_len};
use xara_phy::fm::level::{FmBandwidth, Record};

pub const CQ_DEST: &str = "VCHAT";
pub(crate) const HINT: u8 = 1;

fn address(call: &str, last: bool) -> Option<[u8; 7]> {
    let ax25::Call { base, ssid } = ax25::Call::parse(call)?;
    if base.len() > 6 {
        return None;
    }
    let mut a = [0x40u8; 7];
    for (o, c) in a.iter_mut().zip(base.bytes()) {
        *o = c << 1;
    }
    a[6] = 0x60 | (ssid << 1) | u8::from(last);
    Some(a)
}

pub fn cq_ax25(call: &str, word: Option<&str>) -> Option<Vec<u8>> {
    let mut f = address(CQ_DEST, false)?.to_vec();
    f.extend(address(call, word.is_none())?);
    if let Some(w) = word {
        f.extend(address(w, true)?);
    }
    f.extend([ax25::UI_CTRL, ax25::PID_NO_L3]);
    Some(f)
}

pub fn cq_buffer(call: &str, word: Option<&str>) -> Option<Vec<u8>> {
    kiss::encode(Command::Ax25, &cq_ax25(call, word)?).ok().map(|b| b.bytes)
}

pub fn frame_len(bw: FmBandwidth) -> usize {
    bw.level(Record::Broadcast).frame_bytes()
}

pub fn room(bw: FmBandwidth) -> usize {
    payload_len(Record::Broadcast, frame_len(bw), false).expect("broadcast frame too short")
}

pub fn frame(bw: FmBandwidth, field: &[u8]) -> Option<Vec<u8>> {
    (field.len() <= room(bw))
        .then(|| build_data(Record::Broadcast, frame_len(bw), field, false, HINT, true, Trailer::default()))
}

pub fn frames(bw: FmBandwidth, buffer: &[u8]) -> Option<Vec<Vec<u8>>> {
    let count = kiss::count_for_field(buffer.len(), room(bw));
    if buffer.is_empty() || count > kiss::FM_MAX_FRAGMENTS {
        return None;
    }
    kiss::split_masked(buffer, count, room(bw), kiss::FM_CRC_MASK).iter().map(|f| frame(bw, f)).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Received {
    Cq(crate::cq::Cq),
    Kiss(Command, Vec<u8>),
}

pub fn interpret(buffer: &[u8]) -> Option<Received> {
    let (cmd, f) = kiss::decode(buffer)?;
    if cmd == Command::Ax25 {
        let r = ax25::parse(&f, cmd).ok()?;
        if r.dest == CQ_DEST && r.ctrl == ax25::UI_CTRL && r.pid == ax25::PID_NO_L3 {
            return Some(Received::Cq(crate::cq::Cq { call: r.src, bw: None }));
        }
    }
    Some(Received::Kiss(cmd, f))
}

pub fn receive(frame: &[u8]) -> Option<Received> {
    Reassembler::default().push(frame)
}

#[derive(Clone, Debug)]
pub struct Reassembler(kiss::Reassembler);

impl Default for Reassembler {
    fn default() -> Self {
        Reassembler(kiss::Reassembler::with_crc_mask(kiss::FM_CRC_MASK))
    }
}

impl Reassembler {
    pub fn push(&mut self, frame: &[u8]) -> Option<Received> {
        let d = parse_data(Record::Broadcast, frame, false)?.data;
        if self.0.first().zip(d.first()).is_some_and(|(a, b)| (a ^ b) & 0x7F != 0) {
            self.0.reset();
        }
        match self.0.join(&d, d.len()) {
            kiss::Push::Done(out) => interpret(&out),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }
}
