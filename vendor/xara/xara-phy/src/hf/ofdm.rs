use crate::constellation::constellation;
use crate::hf::level::{Level, Waveform};
use crate::hf::psk::symbol_map;
use crate::ofdm::{self, Layout};
use num_complex::Complex64;

pub use crate::ofdm::{NCP, NFFT, NWIN, fall};

pub const PILOT_PERIOD: u32 = 8;

pub const fn qam_pilot(bin: u32, s: u32) -> bool {
    if bin % 2 == 0 { s % PILOT_PERIOD == 2 } else { s % PILOT_PERIOD == 3 }
}

fn bins(l: &Level) -> (u32, usize) {
    let Waveform::Ofdm { first, nc, .. } = l.waveform else { panic!("not an OFDM level") };
    (first, nc as usize)
}

pub fn frame_rows(l: &Level, other: &[u16], phasors: &[Vec<Complex64>], tx: &[u8]) -> Vec<Vec<Complex64>> {
    let (first, nc) = bins(l);
    let map = symbol_map(l, other);
    let layout = Layout {
        spf: l.spf() as usize,
        bps: l.bps,
        n: l.n() as usize,
        qam: l.is_qam(),
        cst: constellation(l.bps),
        map: &map,
    };
    let mut rows = vec![vec![Complex64::default(); nc]; layout.spf - 1];
    let bins = (0..nc).map(|bi| first as usize + bi);
    let pilot = |b: usize, s: usize| qam_pilot(b as u32, s as u32);
    layout.cells(bins, pilot, tx, |bi, b, r, cell| rows[r][bi] = cell * phasors[b][r].conj());
    rows
}

pub fn synth(first: u32, rows: &[Vec<Complex64>]) -> Vec<f64> {
    ofdm::synth(first as usize, rows.iter().map(Vec::as_slice), 1.0)
}

pub fn modulate(
    l: &Level,
    other: &[u16],
    phasors: &[Vec<Complex64>],
    txs: &[Vec<u8>],
    rand: &mut dyn FnMut(u32) -> u32,
) -> Vec<f64> {
    let (first, nc) = bins(l);
    let cst = constellation(l.bps);
    let lead = l.lead_in() as usize;
    let mut draw = || {
        let mut rows = vec![vec![Complex64::default(); nc]; lead];
        for bi in 0..nc {
            for row in rows.iter_mut() {
                row[bi] = cst[rand(cst.len() as u32) as usize];
            }
        }
        rows
    };
    let mut sets: Vec<_> = (0..if txs.len() > 1 { 3 } else { 1 }).map(|_| draw()).collect();
    let mut rows = sets.swap_remove(sets.len() / 2);
    for tx in txs {
        rows.extend(frame_rows(l, other, phasors, tx));
    }
    synth(first, &rows)
}
