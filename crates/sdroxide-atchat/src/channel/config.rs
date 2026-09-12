//! A port of `channel_server.py::_apply_channel` — channel impairments.

use rand::Rng;
use rand_distr::{Distribution, Normal};

pub const SAMPLE_RATE: u32 = crate::netproto::SAMPLE_RATE;

/// Channel impairment settings (matching the `channel_server.py` arguments).
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// AWGN level (dB). `None` -> no noise added (a clean channel).
    pub snr_db: Option<f64>,
    /// Multipath echo delay (ms). The guard interval is 8 ms.
    pub multipath_delay_ms: f64,
    /// Echo gain relative to the direct signal (0–1).
    pub multipath_gain: f64,
    /// TEST hook: 1-indexed delivered burst numbers — the ones in the list
    /// are zeroed (demod is guaranteed to fail). For triggering ARQ
    /// deterministically. Empty -> disabled.
    pub corrupt_burst_nums: Vec<u64>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            snr_db: None,
            multipath_delay_ms: 0.0,
            multipath_gain: 0.0,
            corrupt_burst_nums: Vec::new(),
        }
    }
}

impl ChannelConfig {
    pub fn multipath_delay_samples(&self) -> usize {
        // Python: int(multipath_delay_ms / 1000 * SAMPLE_RATE)
        (self.multipath_delay_ms / 1000.0 * SAMPLE_RATE as f64) as usize
    }
}

/// Applies the REAL channel impairments: multipath echo first, then AWGN,
/// then clamping to i16. If none are configured the signal passes through
/// unchanged.
///
/// The ordering matches `channel_server.py::_apply_channel` exactly: the noise
/// power is computed against the signal AFTER the echo has been added.
pub fn apply_channel<R: Rng + ?Sized>(
    samples: &[i16],
    cfg: &ChannelConfig,
    rng: &mut R,
) -> Vec<i16> {
    let mut x: Vec<f64> = samples.iter().map(|&s| s as f64).collect();

    let d = cfg.multipath_delay_samples();
    if cfg.multipath_gain > 0.0 && d > 0 && d < x.len() {
        // echo = a copy of x delayed by d samples (the front is zero).
        let echo: Vec<f64> =
            std::iter::repeat_n(0.0, d).chain(x[..x.len() - d].iter().copied()).collect();
        for (xi, ei) in x.iter_mut().zip(echo) {
            *xi += cfg.multipath_gain * ei;
        }
    }

    if let Some(snr_db) = cfg.snr_db {
        let mean_sq = x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64;
        // Python: `np.mean(x**2) or 1.0` -> falls back to 1.0 only if it is EXACTLY 0.0.
        let sig_power = if mean_sq == 0.0 { 1.0 } else { mean_sq };
        let noise_power = sig_power / 10f64.powf(snr_db / 10.0);
        let normal = Normal::new(0.0, noise_power.sqrt()).expect("std >= 0");
        for xi in x.iter_mut() {
            *xi += normal.sample(rng);
        }
    }

    x.iter().map(|v| v.clamp(-32768.0, 32767.0) as i16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn clean_config_is_identity() {
        let mut rng = StdRng::seed_from_u64(1);
        let x: Vec<i16> = (0..500).map(|i| ((i * 137) % 9001 - 4000) as i16).collect();
        let y = apply_channel(&x, &ChannelConfig::default(), &mut rng);
        assert_eq!(x, y);
    }

    #[test]
    fn awgn_injects_measurable_noise() {
        let mut rng = StdRng::seed_from_u64(2);
        let x: Vec<i16> = (0..4000).map(|i| (8000.0 * (i as f64 * 0.05).sin()) as i16).collect();
        let cfg = ChannelConfig { snr_db: Some(15.0), ..Default::default() };
        let y = apply_channel(&x, &cfg, &mut rng);
        let err_power: f64 = x
            .iter()
            .zip(&y)
            .map(|(a, b)| {
                let e = *a as f64 - *b as f64;
                e * e
            })
            .sum::<f64>()
            / x.len() as f64;
        let sig_power: f64 = x.iter().map(|a| (*a as f64).powi(2)).sum::<f64>() / x.len() as f64;
        let measured_snr = 10.0 * (sig_power / err_power).log10();
        assert!(
            (measured_snr - 15.0).abs() < 2.0,
            "measured SNR {measured_snr:.1} dB, expected ~15"
        );
    }

    #[test]
    fn multipath_changes_signal_within_default_guard() {
        let mut rng = StdRng::seed_from_u64(3);
        let x: Vec<i16> = (0..2000).map(|i| (10000.0 * (i as f64 * 0.1).sin()) as i16).collect();
        let cfg =
            ChannelConfig { multipath_delay_ms: 3.0, multipath_gain: 0.3, ..Default::default() };
        let y = apply_channel(&x, &cfg, &mut rng);
        assert_ne!(x, y);
        assert_eq!(x.len(), y.len());
    }
}
