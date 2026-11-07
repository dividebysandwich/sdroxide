use crate::dat::RecordTables;
use crate::dsp::db_amp;
use crate::fec::{frame_bits, interleave, turbo_encode, whiten};
use crate::hf::level::{Bandwidth, Record, Waveform};
use crate::hf::{self, fsk, ofdm, psk};
use std::f64::consts::PI;

pub const TUNE_BACKOFF_DB: f64 = 6.4;
pub const CWID_BACKOFF_DB: f64 = 9.6;

pub fn backoff_db(u: f64) -> f64 {
    u + 2.0 + (10.0 - u) / 5.0
}

pub fn gain(u: f64, drive_db: f64) -> f64 {
    db_amp(drive_db - backoff_db(u))
}

pub fn tune_symbol(drive_db: f64) -> Vec<f64> {
    let a = std::f64::consts::SQRT_2 * db_amp(drive_db - TUNE_BACKOFF_DB);
    let n = crate::hf::ctrl::SYMBOL_SAMPLES;
    (0..n).map(|i| a * (2.0 * PI * f64::from(crate::hf::ctrl::TONE_BIN) * i as f64 / n as f64).sin()).collect()
}

pub struct Modulator {
    pub bw: Bandwidth,
    pub tables: &'static [RecordTables],
}

impl Modulator {
    pub fn new(bw: Bandwidth) -> Self {
        Self { bw, tables: hf::tables(bw) }
    }

    pub fn encode(&self, record: Record, frame: &[u8]) -> Vec<u8> {
        let l = self.bw.level(record);
        let t = &self.tables[record.index()];
        let mut info = frame_bits(frame, l.k as usize);
        whiten(&mut info, hf::whitening());
        interleave(&turbo_encode(&info, &t.turbo, l.puncture_period(), l.n() as usize), &t.chan)
    }

    pub fn gain(&self, record: Record, drive_db: f64) -> f64 {
        gain(f64::from(self.bw.level(record).papr_db), drive_db)
    }

    pub fn transmission(&self, record: Record, frames: &[&[u8]], rand: &mut dyn FnMut(u32) -> u32) -> Vec<f64> {
        let l = self.bw.level(record);
        let other = &self.tables[record.index()].other;
        let txs: Vec<Vec<u8>> = frames.iter().map(|f| self.encode(record, f)).collect();
        let mut x = match l.waveform {
            Waveform::Fsk { .. } => fsk::modulate(l, other, &txs, rand),
            Waveform::Psk { .. } => psk::modulate(l, other, hf::psk_phasors(), &txs, rand),
            Waveform::Ofdm { .. } => ofdm::modulate(l, other, hf::ofdm_phasors(), &txs, rand),
        };
        let norm = self.bw.amp_norm(record);
        x.iter_mut().for_each(|v| *v /= norm);
        x
    }
}
