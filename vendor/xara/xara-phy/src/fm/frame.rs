use super::level::Record;
use crate::call::{ssid_code, with_ssid};
use crate::crc::{crc_ok, crc16_genibus};
use crate::hf::frame::TERMINATOR;

pub const RECORD0_LEN: usize = 39;

const fn overhead(record: Record, vias: bool) -> usize {
    match record {
        Record::Connect => 4,
        Record::Broadcast => 3,
        Record::Start => 5 + vias as usize,
        Record::Data(_) | Record::Short => 4 + vias as usize,
    }
}

pub const fn payload_len(record: Record, frame_len: usize, vias: bool) -> Option<usize> {
    frame_len.checked_sub(overhead(record, vias))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Control {
    pub compressed: bool,
    pub hint: u8,
    pub last: bool,
    pub full: bool,
}

impl Control {
    pub const fn byte(self) -> u8 {
        ((self.compressed as u8) << 5) | ((self.hint & 7) << 2) | ((self.last as u8) << 1) | self.full as u8
    }

    pub const fn from_byte(b: u8) -> Self {
        Self { compressed: b & 0x20 != 0, hint: (b >> 2) & 7, last: b & 2 != 0, full: b & 1 != 0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trailer {
    pub lead_report: u8,
    pub level: u8,
    pub chain_snr: Option<u8>,
}

impl Trailer {
    fn from_bytes(record: Record, t: &[u8]) -> Self {
        match record {
            Record::Broadcast => Trailer::default(),
            Record::Start => Trailer { lead_report: t[0], level: t[1], chain_snr: t.get(2).copied() },
            _ => Trailer { lead_report: t[0], level: 0, chain_snr: t.get(1).copied() },
        }
    }

    fn write(self, record: Record, out: &mut Vec<u8>) {
        if record != Record::Broadcast {
            out.push(self.lead_report);
            if record == Record::Start {
                out.push(self.level);
            }
            out.extend(self.chain_snr);
        }
    }

    pub fn read(record: Record, frame: &[u8], vias: bool) -> Option<Self> {
        let p = payload_len(record, frame.len(), vias)?;
        (record != Record::Connect).then(|| Self::from_bytes(record, &frame[p + 1..frame.len() - 2]))
    }

    pub fn restamp(self, record: Record, frame: &[u8]) -> Option<Vec<u8>> {
        let p = payload_len(record, frame.len(), self.chain_snr.is_some() && record != Record::Broadcast)?;
        let mut out = frame[..=p].to_vec();
        self.write(record, &mut out);
        Some(seal(out))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataFrame {
    pub data: Vec<u8>,
    pub control: Control,
    pub trailer: Trailer,
}

fn seal(mut out: Vec<u8>) -> Vec<u8> {
    let crc = crc16_genibus(&out);
    out.extend(crc.to_be_bytes());
    out
}

pub fn build_data(
    record: Record,
    frame_len: usize,
    data: &[u8],
    compressed: bool,
    hint: u8,
    last: bool,
    trailer: Trailer,
) -> Vec<u8> {
    assert!(record != Record::Connect, "record 0 has its own layout");
    let vias = trailer.chain_snr.is_some() && record != Record::Broadcast;
    let room = payload_len(record, frame_len, vias).expect("frame too short");
    let n = data.len().min(room);
    let full = n == room;
    let mut out = Vec::with_capacity(frame_len);
    out.extend_from_slice(&data[..n]);
    if !full {
        out.push(TERMINATOR);
        out.resize(room, 0);
    }
    out.push(Control { compressed, hint, last, full }.byte());
    trailer.write(record, &mut out);
    debug_assert_eq!(out.len(), frame_len - 2);
    seal(out)
}

pub fn parse_data(record: Record, frame: &[u8], vias: bool) -> Option<DataFrame> {
    if record == Record::Connect {
        return None;
    }
    let p = payload_len(record, frame.len(), vias)?;
    if !crc_ok(frame) {
        return None;
    }
    let control = Control::from_byte(frame[p]);
    let n = if control.full { p } else { frame[..p].iter().rposition(|&b| b == TERMINATOR)? };
    let trailer = Trailer::from_bytes(record, &frame[p + 1..frame.len() - 2]);
    Some(DataFrame { data: frame[..n].to_vec(), control, trailer })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Connect = 0,
    AutoTune = 1,
    Ping = 2,
    Reply = 3,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record0 {
    pub kind: Kind,
    pub hop: u8,
    pub registered: bool,
    pub wide: bool,
    pub source: String,
    pub via1: String,
    pub via2: String,
    pub target: String,
    pub aes: bool,
    pub compression: u8,
    pub control: u8,
    pub chain_snr: u8,
}

fn call_field(call: &str) -> Option<[u8; 8]> {
    let base = call.split('-').next().unwrap_or("").to_ascii_uppercase();
    if base.len() > 7 || !base.is_ascii() {
        return None;
    }
    let mut f = [b' '; 8];
    f[..base.len()].copy_from_slice(base.as_bytes());
    f[7] = call.split_once('-').map_or(Some(0), |(_, s)| ssid_code(s))?;
    Some(f)
}

fn parse_call(f: &[u8]) -> String {
    if f[0] == 0 {
        return String::new();
    }
    let base: String = f[..7].iter().map(|&b| b as char).collect::<String>().trim_end().to_string();
    with_ssid(base, f[7])
}

pub fn snr_byte(snr_db: f64) -> u8 {
    ((snr_db.clamp(0.0, 35.0) * 7.0).round_ties_even() as u8).max(1)
}

impl Record0 {
    pub fn connect(source: &str, target: &str, via1: &str, via2: &str, registered: bool, wide: bool) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            via1: via1.into(),
            via2: via2.into(),
            registered,
            wide,
            control: 0x07,
            ..Self::default()
        }
    }

    pub fn vias(&self) -> usize {
        usize::from(!self.via1.is_empty()) + usize::from(!self.via2.is_empty())
    }

    fn head(&self) -> u8 {
        (self.kind as u8) << 5 | (self.hop & 3) << 3 | u8::from(self.registered) << 2 | u8::from(self.wide) << 1
    }

    fn flags(&self, comp: u8) -> u8 {
        u8::from(self.aes) << 6 | 0x08 | (comp & 3) << 1
    }

    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        let comp = if self.registered { self.compression } else { 0 };
        let mut f = vec![self.head()];
        for c in [&self.source, &self.via1, &self.via2, &self.target] {
            f.extend(call_field(c)?);
        }
        f.extend([0, self.flags(comp), self.control, self.chain_snr]);
        Some(seal(f))
    }

    pub fn restamp(&self, raw: &[u8]) -> Vec<u8> {
        let mut f = raw[..RECORD0_LEN - 2].to_vec();
        f[0] = self.head();
        f[34] = self.flags(self.compression) | raw[34] & 0x80;
        f[36] = self.chain_snr;
        seal(f)
    }

    pub fn parse(f: &[u8]) -> Option<Self> {
        if f.len() != RECORD0_LEN || !crc_ok(f) {
            return None;
        }
        let call = |i: usize| parse_call(&f[1 + 8 * i..9 + 8 * i]);
        Some(Self {
            kind: [Kind::Connect, Kind::AutoTune, Kind::Ping, Kind::Reply][usize::from((f[0] >> 5) & 3)],
            hop: (f[0] >> 3) & 3,
            registered: f[0] & 4 != 0,
            wide: f[0] & 2 != 0,
            source: call(0),
            via1: call(1),
            via2: call(2),
            target: call(3),
            aes: f[34] & 0x40 != 0,
            compression: (f[34] >> 1) & 3,
            control: f[35],
            chain_snr: f[36],
        })
    }
}
