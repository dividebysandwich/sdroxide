//! One channel's decoder.
//!
//! This is `xng_mode_hfdl::HfdlChannelDecoder` put together from the same
//! parts (the DDC or channel filter, the burst demodulator, the PDU parser),
//! for one reason: that type throws away what the demodulator knew about each
//! burst — its bit rate and whether it took one slot or two — and dumphfdl's
//! JSON carries both on every line. Here they are kept: every event from a
//! burst gets `details.burst = {"bit_rate": 1200, "slot": "S"}`.
//!
//! The parts are xng's public ones and the assembly is a copy of its own
//! (`vendor/xng/crates/xng-mode-hfdl/src/lib.rs`), so the decoding is xng's,
//! unchanged; xng itself is a pinned submodule and stays as it is.

use num_complex::Complex;
use xng_dsp::{Ddc, Fir, lowpass_taps};
use xng_mode_hfdl::demod::HfdlDemod;
use xng_mode_hfdl::fec::SETTINGS;
use xng_mode_hfdl::pdu::{HfdlEvent, PduParser};
use xng_mode_hfdl::{CHANNEL_PASSBAND_HZ, CHANNEL_RATE, SUBCARRIER_OFFSET_HZ};

pub struct ChannelDecoder {
    ddc: Option<Ddc>,
    selectivity: Option<Fir>,
    select_buf: Vec<Complex<f32>>,
    channel_buf: Vec<Complex<f32>>,
    demod: HfdlDemod,
    parser: PduParser,
}

impl ChannelDecoder {
    /// `freq_offset_hz` is the channel's carrier relative to the capture's
    /// centre; the +1440 Hz subcarrier is added here.
    pub fn new(input_rate: f64, freq_offset_hz: f64) -> Result<Self, String> {
        let sub = freq_offset_hz + SUBCARRIER_OFFSET_HZ;
        let ddc = if (input_rate - CHANNEL_RATE).abs() < 1e-6 && sub.abs() < 1e-6 {
            None
        } else {
            Some(Ddc::new(input_rate, CHANNEL_RATE, sub, CHANNEL_PASSBAND_HZ)?)
        };
        let selectivity = if ddc.is_none() {
            Some(Fir::new(lowpass_taps(CHANNEL_PASSBAND_HZ / CHANNEL_RATE, 101)))
        } else {
            None
        };
        Ok(Self {
            ddc,
            selectivity,
            select_buf: Vec::new(),
            channel_buf: Vec::new(),
            demod: HfdlDemod::new(CHANNEL_RATE),
            parser: PduParser::new(),
        })
    }

    pub fn process(&mut self, input: &[Complex<f32>]) -> Vec<HfdlEvent> {
        let channel: &[Complex<f32>] = match &mut self.ddc {
            Some(ddc) => {
                self.channel_buf.clear();
                ddc.process(input, &mut self.channel_buf);
                &self.channel_buf
            }
            None => match &mut self.selectivity {
                Some(fir) => {
                    self.select_buf.clear();
                    fir.process(input, &mut self.select_buf);
                    &self.select_buf
                }
                None => input,
            },
        };
        let mut out = Vec::new();
        for burst in self.demod.process(channel) {
            let slot = slot_of(burst.bps, burst.payload.len());
            for mut e in self.parser.parse(&burst.payload, burst.bps) {
                e.fec_corrected = Some(burst.fec_corrected);
                e.freq_skew_hz = Some(burst.freq_skew_hz);
                e.snr_db = burst.snr_db;
                if let Some(d) = e.details.as_object_mut() {
                    let mut b = serde_json::json!({ "bit_rate": burst.bps });
                    if let Some(s) = slot {
                        b["slot"] = s.into();
                    }
                    d.insert("burst".into(), b);
                }
                out.push(e);
            }
        }
        out
    }

    pub fn level_dbfs(&self) -> f32 {
        self.demod.level_dbfs()
    }
}

/// "S" or "D": whether a burst of `bps` whose decoded payload is `len` bytes
/// took one slot or two. The eight settings have eight different payload
/// sizes, so the size says which it was; `None` if it matches none (nothing is
/// then guessed).
fn slot_of(bps: u32, len: usize) -> Option<&'static str> {
    SETTINGS
        .iter()
        .find(|s| s.bps == bps && sizes(s).contains(&len))
        .map(|s| if s.double_slot { "D" } else { "S" })
}

/// The payload sizes a setting's burst can come back with: the whole bytes
/// its decoded bits fill, or one more where the last byte is a half one (the
/// 300 bit/s settings: 540 and 1260 bits).
fn sizes(s: &xng_mode_hfdl::fec::Setting) -> [usize; 2] {
    [s.payload_bytes(), s.payload_bits().div_ceil(8)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_size_tells_the_slot() {
        for s in &SETTINGS {
            let want = if s.double_slot { "D" } else { "S" };
            for len in sizes(s) {
                assert_eq!(slot_of(s.bps, len), Some(want), "{s:?} at {len} bytes");
            }
        }
        assert_eq!(slot_of(300, 12), None, "no setting has that size: nothing guessed");
        // All eight sizes differ, which is what makes the lookup safe.
        let mut all: Vec<_> = SETTINGS.iter().flat_map(|s| sizes(s)).collect();
        all.sort_unstable();
        all.dedup();
        let distinct: std::collections::HashSet<_> =
            SETTINGS.iter().map(|s| s.payload_bytes()).collect();
        assert_eq!(distinct.len(), 8, "{all:?}");
    }
}
