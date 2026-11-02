use crate::ax25::{CALL_ALPHABET, Call};
use xara_phy::crc::crc16_genibus;
use xara_phy::hf::level::{Bandwidth, Record};

pub const RECORD: Record = Record::new(15).unwrap();
pub const PAYLOAD: usize = 6;

const fn bw_code(bw: Bandwidth) -> u64 {
    match bw {
        Bandwidth::Hz500 => 1,
        Bandwidth::Hz2300 => 2,
        Bandwidth::Hz2750 => 3,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bw {
    Known(Bandwidth),
    Unknown,
}

impl std::fmt::Display for Bw {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Bw::Known(b) => write!(f, "{}", b.hz()),
            Bw::Unknown => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cq {
    pub call: String,
    pub bw: Option<Bw>,
}

impl Cq {
    pub fn host_line(&self) -> String {
        match self.bw {
            Some(bw) => format!("CQFRAME {} {bw}", self.call),
            None => format!("CQFRAME {}", self.call),
        }
    }
}

pub fn valid_call(call: &str) -> bool {
    Call::parse(call).is_some()
}

pub fn payload(call: &str, bw: Bandwidth) -> Option<[u8; PAYLOAD]> {
    let Call { base, ssid } = Call::parse(call)?;
    let mut v: u64 = 0;
    for c in format!("{base:<7}").bytes() {
        v = (v << 6) | CALL_ALPHABET.iter().position(|&a| a == c)? as u64;
    }
    v = (v << 6) | (bw_code(bw) * 16 + u64::from(ssid));
    v.to_be_bytes()[2..].try_into().ok()
}

pub fn frame(call: &str, bw: Bandwidth) -> Option<Vec<u8>> {
    let p = payload(call, bw)?;
    Some([&p[..], &crc16_genibus(&p).to_be_bytes()].concat())
}

pub fn parse(frame: &[u8]) -> Option<Cq> {
    let p: [u8; 8] = [&[0, 0], frame.get(..PAYLOAD)?].concat().try_into().ok()?;
    let v = u64::from_be_bytes(p);
    let call: String =
        (0..7).filter_map(|k| CALL_ALPHABET.get(((v >> (42 - 6 * k)) & 63) as usize)).map(|&c| c as char).collect();
    let mut call = call.trim_end().to_string();
    let x = v & 63;
    if x & 15 != 0 {
        call += &format!("-{}", x & 15);
    }
    let bw = Bandwidth::ALL.into_iter().find(|&b| bw_code(b) == x >> 4).map_or(Bw::Unknown, Bw::Known);
    Some(Cq { call, bw: Some(bw) })
}
