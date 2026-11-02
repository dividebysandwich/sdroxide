use crate::crc::crc_ok;
use crate::dat::RecordTables;
use crate::hf;
use crate::hf::level::{Bandwidth, Record};
use crate::turbo::{CrcCheck, Decoder, split, used_len};

pub const ITERATIONS: usize = 10;
pub const EXTRINSIC_SCALE: f64 = 0.7;
pub const MIN_ITERATIONS: usize = 2;

#[derive(Clone, Debug)]
pub struct FrameOut {
    pub bytes: Vec<u8>,
    pub ok: bool,
    pub iterations: usize,
}

pub struct FrameDecoder {
    pub bw: Bandwidth,
    pub tables: &'static [RecordTables],
    pub white: &'static [u8],
    turbo: Decoder,
    pub early: bool,
}

impl FrameDecoder {
    pub fn new(bw: Bandwidth) -> Self {
        Self { bw, tables: hf::tables(bw), white: hf::whitening(), turbo: Decoder::default(), early: true }
    }

    pub fn decode(&self, record: Record, llr_tx: &[f64]) -> FrameOut {
        let l = self.bw.level(record);
        let shape = (l.k as usize, l.n() as usize, l.puncture_period() as usize);
        decode_frame(&self.turbo, &self.tables[record.index()], shape, self.white, self.early, llr_tx)
    }
}

pub(crate) fn decode_frame(
    turbo: &Decoder,
    t: &RecordTables,
    (k, n, p): (usize, usize, usize),
    white: &[u8],
    early: bool,
    llr_tx: &[f64],
) -> FrameOut {
    assert_eq!(llr_tx.len(), t.chan.len(), "demapper and channel interleaver disagree on frame length");
    let mut soft = vec![0.0; n];
    for (&c, &l) in t.chan.iter().zip(llr_tx) {
        soft[c as usize] = l;
    }
    let sp = split(&soft[..used_len(k, p)], &t.turbo, p);
    let check = CrcCheck { white, nbytes: k / 8 };
    let early = early.then_some((MIN_ITERATIONS, &check));
    let d = turbo.decode(&sp, &t.turbo, ITERATIONS, EXTRINSIC_SCALE, early);
    let bytes = check.bytes(&d.bits);
    let ok = crc_ok(&bytes);
    FrameOut { bytes, ok, iterations: d.iterations }
}
