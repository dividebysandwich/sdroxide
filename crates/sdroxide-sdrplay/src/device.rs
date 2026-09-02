//! Per-model policy: sample-rate plumbing, bandwidth choice, LNA state
//! limits, antenna routing. Pure functions over the config types, so every
//! table here is unit-tested without hardware.

use sdroxide_types::{SdrPlayConfig, SdrPlayModel};

use crate::ffi;

/// The ADC floor: below this the ADC still runs at 2 Msps and the service
/// decimates down to the requested rate.
pub const MIN_FS_HZ: f64 = 2_000_000.0;
/// The ADC ceiling the API accepts.
pub const MAX_FS_HZ: f64 = 10_660_000.0;

/// Turn a requested effective rate into the ADC rate and decimation factor.
///
/// At or above 2 Msps the ADC runs at the requested rate directly. Below, the
/// ADC stays at the closest power-of-two multiple within reach and the
/// service's decimator makes up the difference — factor 2..=32, which puts
/// the effective floor at 62.5 kHz.
pub fn fs_and_decim(rate_hz: f64) -> (f64, u8) {
    let rate = rate_hz.clamp(62_500.0, MAX_FS_HZ);
    let mut fs = rate;
    let mut factor: u8 = 1;
    while fs < MIN_FS_HZ && factor < 32 {
        fs *= 2.0;
        factor *= 2;
    }
    (fs, factor)
}

/// The ADC clock the API insists on with both of an RSPduo's tuners running.
///
/// Not a choice: in dual-tuner mode both tuners hand the service a low IF
/// rather than baseband, and 6 MHz with a 1.620 MHz IF is the pairing its
/// downconverter is built around (8 MHz with 2.048 MHz is the other one).
pub const DUAL_FS_HZ: f64 = 6_000_000.0;
/// What that downconverter delivers, before any further decimation.
pub const DUAL_OUT_HZ: f64 = 2_000_000.0;

/// Turn a requested effective rate into the decimation factor to apply to
/// [`DUAL_OUT_HZ`], for an RSPduo running both tuners.
///
/// The rate is *clamped*, not refused: a configuration carried over from
/// single-tuner operation will ask for something wider than 2 Msps, and
/// opening 2 Msps wide beats refusing to open.
pub fn dual_decim(rate_hz: f64) -> u8 {
    let want = rate_hz.clamp(DUAL_OUT_HZ / 32.0, DUAL_OUT_HZ);
    let mut factor: u8 = 1;
    // The nearest available rate, chosen in the log domain — halving is the
    // step, so the crossover between two neighbours is their geometric mean
    // rather than their average.
    while factor < 32 && want <= DUAL_OUT_HZ / f64::from(factor) / std::f64::consts::SQRT_2 {
        factor *= 2;
    }
    factor
}

/// The widest analog IF filter that fits inside the effective rate — wider
/// would alias, narrower gives the panadapter rolled-off edges for no reason.
pub fn bw_for_rate(effective_hz: f64) -> ffi::BwType {
    let mut best = SdrPlayConfig::BANDWIDTHS_KHZ[0];
    for &khz in &SdrPlayConfig::BANDWIDTHS_KHZ {
        if (khz as f64) * 1000.0 <= effective_hz {
            best = khz;
        }
    }
    best as ffi::BwType
}

/// The bandwidth to program: the operator's override when it names a real
/// filter, else automatic from the rate.
///
/// `dual` says both of an RSPduo's tuners are running, where the low IF the
/// API works from leaves room for nothing wider than 1.536 MHz — a wider
/// filter is refused by the service, so a setting carried over from
/// single-tuner operation is clamped rather than passed on.
pub fn bw_khz_for(cfg_bw_khz: u32, effective_hz: f64, dual: bool) -> ffi::BwType {
    let khz = if SdrPlayConfig::BANDWIDTHS_KHZ.contains(&cfg_bw_khz) {
        cfg_bw_khz as ffi::BwType
    } else {
        bw_for_rate(effective_hz)
    };
    if dual { khz.min(SdrPlayConfig::DUAL_MAX_BW_KHZ as ffi::BwType) } else { khz }
}

/// Highest usable LNA state for a model on a band, from the one table in
/// [`SdrPlayModel::max_lna_state_on`].
///
/// A thin forward and deliberately so: the settings UI is wasm-safe and cannot
/// link this crate, so the table has to live where both can reach it. What is
/// left here is the clamp's own vocabulary — a state past this is rejected by
/// the service, so the driver clamps and reports what it kept.
pub fn max_lna_state(model: SdrPlayModel, freq_hz: f64, hiz: bool, hdr: bool) -> u8 {
    model.max_lna_state_on(freq_hz, hiz, hdr)
}

/// RSP2 antenna routing for a name from [`SdrPlayModel::antennas`]:
/// `(antennaSel, amPortSel)`. The Hi-Z input rides the AM-port switch; the
/// A/B selector is only meaningful on the 50 Ω path.
pub fn rsp2_antenna(name: &str) -> Option<(i32, i32)> {
    match name {
        "Antenna A" => Some((ffi::RSP2_ANTENNA_A, ffi::RSP2_AMPORT_2)),
        "Antenna B" => Some((ffi::RSP2_ANTENNA_B, ffi::RSP2_AMPORT_2)),
        "Hi-Z" => Some((ffi::RSP2_ANTENNA_A, ffi::RSP2_AMPORT_1_HIZ)),
        _ => None,
    }
}

/// RSPdx antenna selector for a name from [`SdrPlayModel::antennas`].
pub fn rspdx_antenna(name: &str) -> Option<i32> {
    match name {
        "Antenna A" => Some(ffi::RSPDX_ANTENNA_A),
        "Antenna B" => Some(ffi::RSPDX_ANTENNA_B),
        "Antenna C" => Some(ffi::RSPDX_ANTENNA_C),
        _ => None,
    }
}

/// RSPduo tuner-1 port for a name from [`SdrPlayModel::antennas`].
pub fn rspduo_tuner1_amport(name: &str) -> Option<i32> {
    match name {
        "50 Ohm port" => Some(ffi::RSPDUO_AMPORT_2),
        "Hi-Z port" => Some(ffi::RSPDUO_AMPORT_1_HIZ),
        _ => None,
    }
}

/// Whether an antenna choice routes a Hi-Z input, for the LNA clamp — from
/// [`SdrPlayModel::antenna_is_hiz`], for the same reason as above.
pub fn antenna_is_hiz(model: SdrPlayModel, antenna: &str) -> bool {
    model.antenna_is_hiz(antenna)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_rates_ride_a_2msps_adc_with_decimation() {
        assert_eq!(fs_and_decim(2_000_000.0), (2_000_000.0, 1));
        assert_eq!(fs_and_decim(1_000_000.0), (2_000_000.0, 2));
        assert_eq!(fs_and_decim(500_000.0), (2_000_000.0, 4));
        assert_eq!(fs_and_decim(250_000.0), (2_000_000.0, 8));
        assert_eq!(fs_and_decim(62_500.0), (2_000_000.0, 32));
        assert_eq!(fs_and_decim(10_000_000.0), (10_000_000.0, 1));
        // A nonsense rate cannot produce a factor the API rejects.
        let (fs, factor) = fs_and_decim(1.0);
        assert!(factor <= 32 && fs >= MIN_FS_HZ);
    }

    #[test]
    fn bandwidth_never_exceeds_the_rate() {
        assert_eq!(bw_for_rate(2_000_000.0), 1536);
        assert_eq!(bw_for_rate(250_000.0), 200);
        assert_eq!(bw_for_rate(600_000.0), 600);
        assert_eq!(bw_for_rate(6_000_000.0), 6000);
        assert_eq!(bw_for_rate(10_000_000.0), 8000);
        // An override is obeyed when it names a real filter and ignored when
        // it does not — a stale config cannot program a bandwidth the tuner
        // lacks.
        assert_eq!(bw_khz_for(600, 2_000_000.0, false), 600);
        assert_eq!(bw_khz_for(0, 2_000_000.0, false), 1536);
        assert_eq!(bw_khz_for(1234, 2_000_000.0, false), 1536);
        // With both RSPduo tuners running the low IF caps the filter, and a
        // setting left over from a 10 Msps single-tuner session must not be
        // handed to the service.
        assert_eq!(bw_khz_for(6000, 6_000_000.0, true), 1536);
        assert_eq!(bw_khz_for(600, 2_000_000.0, true), 600);
    }

    /// The dual-tuner rate ladder is 2 Msps halved, and every entry in the
    /// list the settings panel offers must land on itself.
    #[test]
    fn dual_tuner_rates_are_two_msps_decimated() {
        for (rate, want) in [
            (2_000_000.0, 1),
            (1_000_000.0, 2),
            (500_000.0, 4),
            (250_000.0, 8),
            (125_000.0, 16),
            (62_500.0, 32),
        ] {
            assert_eq!(dual_decim(rate), want, "{rate}");
            assert!((DUAL_OUT_HZ / f64::from(dual_decim(rate)) - rate).abs() < 1.0);
        }
        // A rate carried over from single-tuner operation is clamped to the
        // widest the mode has rather than refused.
        assert_eq!(dual_decim(10_000_000.0), 1);
        assert_eq!(dual_decim(0.0), 32);
        // And one between two rungs takes the nearer, in the log domain.
        assert_eq!(dual_decim(1_500_000.0), 1);
        assert_eq!(dual_decim(600_000.0), 4);
    }

    #[test]
    fn lna_limits_follow_the_documented_band_edges() {
        use SdrPlayModel::*;
        // RSP1A/1B: 7 states on HF, 10 on VHF/UHF, 9 in L-band.
        assert_eq!(max_lna_state(Rsp1b, 14e6, false, false), 6);
        assert_eq!(max_lna_state(Rsp1b, 145e6, false, false), 9);
        assert_eq!(max_lna_state(Rsp1b, 1_300e6, false, false), 8);
        // The generic ceiling in sdroxide-types is the band maximum.
        assert_eq!(Rsp1b.max_lna_state(), 9);
        // RSP1: 4 states, 3 in L-band.
        assert_eq!(max_lna_state(Rsp1, 14e6, false, false), 3);
        assert_eq!(max_lna_state(Rsp1, 1_300e6, false, false), 2);
        // RSP2: the Hi-Z port has fewer states than either 50 Ω path.
        assert_eq!(max_lna_state(Rsp2, 7e6, true, false), 4);
        assert_eq!(max_lna_state(Rsp2, 7e6, false, false), 8);
        assert_eq!(max_lna_state(Rsp2, 435e6, false, false), 5);
        // RSPduo mirrors the RSP1A with the RSP2's Hi-Z case.
        assert_eq!(max_lna_state(RspDuo, 7e6, true, false), 4);
        assert_eq!(max_lna_state(RspDuo, 7e6, false, false), 6);
        // RSPdx: dense but band-dependent; HDR is its own path.
        assert_eq!(max_lna_state(RspDx, 1e6, true, true), 21);
        assert_eq!(max_lna_state(RspDx, 7e6, false, false), 18);
        assert_eq!(max_lna_state(RspDx, 300e6, false, false), 27);
        assert_eq!(max_lna_state(RspDx, 1_300e6, false, false), 18);
        assert_eq!(RspDx.max_lna_state(), 27);
    }

    #[test]
    fn antenna_names_route_to_the_documented_selectors() {
        // The RSP2's A/B values are 5 and 6 — not 0 and 1 — and Hi-Z is a
        // different switch entirely.
        assert_eq!(rsp2_antenna("Antenna A"), Some((5, 0)));
        assert_eq!(rsp2_antenna("Antenna B"), Some((6, 0)));
        assert_eq!(rsp2_antenna("Hi-Z"), Some((5, 1)));
        assert_eq!(rsp2_antenna("Antenna C"), None);
        assert_eq!(rspdx_antenna("Antenna C"), Some(2));
        assert_eq!(rspduo_tuner1_amport("Hi-Z port"), Some(1));
        assert!(antenna_is_hiz(SdrPlayModel::Rsp2, "Hi-Z"));
        assert!(!antenna_is_hiz(SdrPlayModel::Rsp1b, "Hi-Z"));
    }
}
