pub mod blind;
pub mod ctrl;
pub mod frontend;
pub mod fsk;
pub mod ofdm;
pub mod psk;
pub mod receiver;
pub mod snr;

use num_complex::Complex64;
use std::f64::consts::PI;

pub use crate::ofdm::FS;
pub const FS3: f64 = 3000.0;
pub const DEC: i64 = 16;

pub fn rot(n0: i64, n: usize, f_hz: f64) -> impl Iterator<Item = Complex64> {
    (0..n).map(move |k| Complex64::from_polar(1.0, -2.0 * PI * f_hz * (n0 + k as i64) as f64 / FS3))
}
