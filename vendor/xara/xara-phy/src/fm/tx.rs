use super::code::{BINS, Codec, Phasors, phasors, record_key, tables};
use super::constellation::constellation;
use super::ctrl::data_lead_in;
use super::level::{Level, Record, qam_pilot};
use crate::dat::marker_map;
use crate::fm::level::FmBandwidth;
use crate::ofdm::{self, Layout};
use num_complex::Complex64;

pub use crate::ofdm::{NCP, NFFT, NWIN};
pub const PRE_SIGNAL_SYMBOLS: usize = 42;
pub const PRE_SIGNAL_BIN: usize = 32;
pub const TUNE_BIN: usize = 32;
pub const TUNE_BACKOFF_DB: f64 = 5.007;

pub type Row = [Complex64; BINS];

pub fn symbol_map(l: &Level, other: &[u16]) -> Vec<Vec<bool>> {
    marker_map(other, l.nc(), l.layout.markers as usize, l.nsym())
}

pub fn frame_rows(l: &Level, other: &[u16], p: &Phasors, tx: &[u8]) -> Vec<Row> {
    let map = symbol_map(l, other);
    let layout = Layout {
        spf: l.spf(),
        bps: l.layout.bps,
        n: l.n(),
        qam: l.is_qam(),
        cst: constellation(l.layout.bps),
        map: &map,
    };
    let mut rows = vec![[Complex64::default(); BINS]; l.nsym()];
    layout.cells(l.bins(), qam_pilot, tx, |_, b, r, cell| rows[r][b] = cell * p[b][r].conj());
    rows
}

pub fn lead_rows(l: &Level, n: usize, rand: &mut dyn FnMut(u32) -> u32) -> Vec<Row> {
    let cst = constellation(l.layout.bps);
    let mut rows = vec![[Complex64::default(); BINS]; n];
    for b in l.bins() {
        for row in rows.iter_mut() {
            row[b] = cst[rand(cst.len() as u32) as usize];
        }
    }
    rows
}

pub fn pre_signal_rows() -> Vec<Row> {
    let mut r = [Complex64::default(); BINS];
    r[PRE_SIGNAL_BIN] = Complex64::new(0.0, -7.2);
    vec![r; PRE_SIGNAL_SYMBOLS]
}

pub fn synth(rows: &[Row], scale: f64) -> Vec<f64> {
    ofdm::synth(0, rows.iter().map(|row| &row[..]), scale)
}

pub fn tune_block(drive_db: f64) -> Vec<f64> {
    let a = std::f64::consts::SQRT_2 * crate::dsp::db_amp(drive_db - TUNE_BACKOFF_DB);
    (0..NFFT).map(|n| a * (2.0 * std::f64::consts::PI * (TUNE_BIN * n) as f64 / NFFT as f64).cos()).collect()
}

pub const CONNECT_LEAD_IN: usize = data_lead_in(23);
pub const IDLE_BEACON_LEAD_IN: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TxOptions {
    pub drive_db: f64,
    pub pre_signal: bool,
}

impl Default for TxOptions {
    fn default() -> Self {
        Self { drive_db: 0.0, pre_signal: false }
    }
}

pub struct Modulator {
    pub bw: FmBandwidth,
    pub codec: Codec,
}

impl Modulator {
    pub fn new(bw: FmBandwidth) -> Self {
        Self { bw, codec: Codec::new(bw) }
    }

    pub fn level(&self, record: Record) -> Level {
        self.bw.level(record)
    }

    pub fn encode(&self, record: Record, frame: &[u8]) -> Vec<u8> {
        self.codec.encode(record, frame)
    }

    pub fn rows(&self, record: Record, key: u32, frames: &[&[u8]]) -> Vec<Row> {
        let l = self.level(record);
        let p = phasors(record_key(record, key));
        let other = &tables(self.bw)[record.index()].other;
        frames.iter().flat_map(|f| frame_rows(&l, other, &p, &self.encode(record, f))).collect()
    }

    pub fn transmission(
        &self,
        record: Record,
        key: u32,
        frames: &[&[u8]],
        lead: usize,
        opt: TxOptions,
        rand: &mut dyn FnMut(u32) -> u32,
    ) -> Vec<f64> {
        let l = self.level(record);
        let mut rows = if opt.pre_signal { pre_signal_rows() } else { Vec::new() };
        rows.extend(lead_rows(&l, lead, rand));
        rows.extend(self.rows(record, key, frames));
        let g = l.gain(opt.drive_db) / f64::from(l.amp_norm);
        let mut x = synth(&rows, g);
        x.iter_mut().for_each(|v| *v = v.clamp(-1.0, 1.0));
        x
    }
}

pub fn to_i16(x: &[f64]) -> Vec<i16> {
    x.iter().map(|&v| (v * 32767.0).round_ties_even() as i16).collect()
}
