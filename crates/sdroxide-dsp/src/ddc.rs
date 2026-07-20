//! Digital down-converter: NCO mix to baseband, then multistage decimation
//! from the device rate to a ~48 kHz channel rate.

use crate::decim::{FirDecim, HalfbandDecim};
use crate::nco::Nco;
use crate::Complex32;

pub struct Ddc {
    nco: Nco,
    in_rate: f64,
    halfbands: Vec<HalfbandDecim>,
    final_decim: Option<FirDecim>,
    out_rate: f64,
    tmp_a: Vec<Complex32>,
    tmp_b: Vec<Complex32>,
}

impl Ddc {
    /// Build a chain from `in_rate` down to as close to `target_rate` as an
    /// integer decimation allows. The exact rate is [`Self::out_rate`] —
    /// callers resample audio afterwards if they need the target exactly.
    pub fn new(in_rate: f64, target_rate: f64) -> Self {
        let mut rate = in_rate;
        let mut halfbands = Vec::new();
        while rate > target_rate * 8.0 {
            halfbands.push(HalfbandDecim::new());
            rate /= 2.0;
        }
        let m = (rate / target_rate).round().max(1.0) as usize;
        let final_decim = (m > 1).then(|| FirDecim::new(m));
        let out_rate = rate / m as f64;

        Ddc {
            nco: Nco::new(0.0, in_rate),
            in_rate,
            halfbands,
            final_decim,
            out_rate,
            tmp_a: Vec::new(),
            tmp_b: Vec::new(),
        }
    }

    pub fn out_rate(&self) -> f64 {
        self.out_rate
    }

    /// Tune: `offset_hz` is the wanted signal's offset from the hardware
    /// center frequency; it gets mixed down to DC.
    pub fn set_offset_hz(&mut self, offset_hz: f64) {
        self.nco.set_freq(-offset_hz, self.in_rate);
    }

    /// Appends channel-rate samples to `out`.
    pub fn process(&mut self, input: &[Complex32], out: &mut Vec<Complex32>) {
        self.tmp_a.clear();
        self.nco.mix(input, &mut self.tmp_a);

        for hb in &mut self.halfbands {
            self.tmp_b.clear();
            hb.process(&self.tmp_a, &mut self.tmp_b);
            std::mem::swap(&mut self.tmp_a, &mut self.tmp_b);
        }

        match &mut self.final_decim {
            Some(d) => d.process(&self.tmp_a, out),
            None => out.extend_from_slice(&self.tmp_a),
        }
    }
}
