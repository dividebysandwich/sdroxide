use crate::constellation::take_bits;
use crate::dsp::sine;
use crate::hf::level::{FSK_PILOTS, Level, Waveform};
use crate::ofdm::fall;

pub const GRAY: [u32; 16] = [0, 1, 3, 2, 7, 6, 4, 5, 15, 14, 12, 13, 8, 9, 11, 10];

pub(crate) struct Grid {
    pub symbols: usize,
    pub sps: u32,
    pub first: u32,
    nb: u32,
    step: u32,
}

impl Grid {
    pub(crate) fn of(l: &Level) -> Self {
        let Waveform::Fsk { symbols, sps, first, last, step } = l.waveform else { panic!("not an FSK level") };
        Self { symbols: symbols as usize, sps, first, nb: last - first + 1, step }
    }

    pub(crate) fn bin(&self, base: u16, t: u32) -> u32 {
        self.first + (u32::from(base) + self.step * t - self.first) % self.nb
    }
}

pub fn rows(l: &Level, other: &[u16], tx: &[u8]) -> Vec<u32> {
    let g = Grid::of(l);
    let n = g.symbols - 1;
    let hop = &other[..n];
    let pairs = &other[n + FSK_PILOTS as usize..];
    let mut ptr = 0;
    let out = (0..n)
        .map(|i| {
            let t = if pairs[2 * i] != 0 { u32::from(pairs[2 * i + 1]) } else { GRAY[take_bits(tx, &mut ptr, l.bps)] };
            g.bin(hop[i], t)
        })
        .collect();
    assert_eq!(ptr, l.n() as usize);
    out
}

pub fn modulate(l: &Level, other: &[u16], txs: &[Vec<u8>], rand: &mut dyn FnMut(u32) -> u32) -> Vec<f64> {
    let g = Grid::of(l);
    let lead = l.lead_in() as usize;
    let mut bins = Vec::new();
    for _ in txs {
        bins = (0..lead).map(|i| g.bin(other[i + 1], rand(1 << l.bps))).collect();
    }
    for tx in txs {
        bins.extend(rows(l, other, tx));
    }
    let nsamp = 16 * g.sps as usize;
    let mut x: Vec<f64> = bins.iter().flat_map(|&k| sine(k, nsamp)[..nsamp].iter().map(|&s| 2.0 * s)).collect();
    if let Some(tail) = x.len().checked_sub(nsamp / 32) {
        for (v, f) in x[tail..].iter_mut().zip(fall(nsamp / 32)) {
            *v *= f;
        }
    }
    x
}
