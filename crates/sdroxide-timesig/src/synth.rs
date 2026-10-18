//! A time-signal station in a box: the audio a receiver would hand the
//! decoder, for a chosen moment, at a chosen signal-to-noise ratio.
//!
//! The symbols come from each format's own encoder, so the generator and the
//! decoder meet only at the specification. The keying is a clean step
//! softened over a millisecond, symmetric so the half-way point of every edge
//! is exactly on the tenth of a second it belongs to — which is what lets a
//! test say the decoder was right to a fraction of a millisecond.
//!
//! The SNR is quoted in a 3 kHz bandwidth against the full carrier, like the
//! other decoders' harnesses. The carrier is narrow and the decoder's own
//! bandwidth is a few tens of hertz, so add about 18 dB to compare with an SNR
//! in 50 Hz: −20 dB here is the carrier 2 dB below the noise in 50 Hz.

use sdroxide_types::TimeStation;

use crate::format::{Format, Sym};

/// What a station's reduced carrier is, against its full carrier.
fn low_level(station: TimeStation) -> f32 {
    match station {
        // 15 % — PTB.
        TimeStation::Dcf77 => 0.15,
        // Off — NPL.
        TimeStation::Msf => 0.0,
        // 17 dB down — NIST.
        TimeStation::Wwvb => 0.1413,
        // 10 % — NICT.
        TimeStation::Jjy40 | TimeStation::Jjy60 => 0.1,
    }
}

pub struct Synth {
    pub station: TimeStation,
    pub rate: f64,
    /// Where the carrier lands in the audio, Hz.
    pub tone_hz: f64,
    /// Signal-to-noise ratio in 3 kHz; `None` for no noise at all.
    pub snr_db: Option<f64>,
    /// How far the receiver's sample clock is from true, parts per million:
    /// positive means it runs fast, so a true second spans more samples.
    pub rate_error_ppm: f64,
    pub seed: u64,
}

impl Synth {
    pub fn new(station: TimeStation, rate: f64) -> Self {
        Synth {
            station,
            rate,
            tone_hz: f64::from(sdroxide_types::TIMESIG_TONE_HZ),
            snr_db: None,
            rate_error_ppm: 0.0,
            seed: 1,
        }
    }

    /// `secs` of audio whose first sample is the UTC instant `start` (Unix
    /// seconds, fractional).
    pub fn render(&self, start: f64, secs: f64) -> Vec<f32> {
        let fmt = Format::for_station(self.station);
        let low = low_level(self.station);
        let amp = 0.3f64;
        let true_rate = self.rate * (1.0 + self.rate_error_ppm * 1e-6);
        let sigma = self.snr_db.map(|snr| {
            let carrier_power = amp * amp / 2.0;
            (carrier_power / 10f64.powf(snr / 10.0) * self.rate / 6000.0).sqrt()
        });
        let mut rng = Rng(self.seed.max(1));
        let mut minute: Option<(i64, [Sym; 60])> = None;
        let mut level_at = |t: f64| -> f32 {
            let sec = t.floor() as i64;
            let m = sec.div_euclid(60) * 60;
            if minute.is_none_or(|(at, _)| at != m) {
                minute = Some((m, (fmt.encode)(m)));
            }
            let sym = minute.expect("just set").1[(sec - m) as usize];
            let slot = ((t - sec as f64) * 10.0) as usize;
            let reduced = fmt.template(sym).is_some_and(|tpl| tpl.low[slot.min(9)]);
            if reduced { low } else { 1.0 }
        };
        let n = (secs * self.rate) as usize;
        let mut out = Vec::with_capacity(n);
        let half_ramp = 0.0005;
        for i in 0..n {
            let t = start + i as f64 / true_rate;
            let a = (level_at(t - half_ramp) + level_at(t) + level_at(t + half_ramp)) / 3.0;
            let carrier = amp
                * f64::from(a)
                * (std::f64::consts::TAU * self.tone_hz * i as f64 / self.rate).cos();
            let noise = sigma.map_or(0.0, |s| s * rng.gauss());
            out.push((carrier + noise) as f32);
        }
        out
    }
}

/// xorshift64* with a Box–Muller pair, so the crate needs no RNG dependency
/// and a seed always means the same noise.
struct Rng(u64);

impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d);
        ((x >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    fn gauss(&mut self) -> f64 {
        let (u, v) = (self.next_f64(), self.next_f64());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
