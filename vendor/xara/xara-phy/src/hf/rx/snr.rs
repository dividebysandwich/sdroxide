use crate::dsp::median;
use crate::hf::level::Level;
use num_complex::Complex64;

pub const SNR_REF_DB: f64 = 9.4;
pub const FSK_K_DB: f64 = 17.2;
pub const PSK_K_DB: f64 = 20.91;
pub const OFDM_K_DB: f64 = 20.81;

pub fn lg(x: f64) -> f64 {
    if x > 0.0 { x.log10() } else { f64::NEG_INFINITY }
}

pub fn normalise(raw: f64, l: &Level) -> f64 {
    raw - (SNR_REF_DB - f64::from(l.papr_db))
}

pub fn phase_error_snr(prods: &[Complex64], l: &Level, k_db: f64) -> f64 {
    let unit: Vec<Complex64> = prods.iter().map(|&p| if p.norm() > 0.0 { p / p.norm() } else { p }).collect();
    let s: Complex64 = unit.iter().sum();
    let r = Complex64::from_polar(1.0, -s.arg());
    let v: Vec<f64> = unit.iter().map(|&p| (p * r).im.abs()).collect();
    10.0 * lg(64.0 / f64::from(l.sps())) + 10.0 * lg(f64::from(l.nc())) - 20.0 * lg(median(&v)) - k_db
        + f64::from(l.snr_corr_db)
}
