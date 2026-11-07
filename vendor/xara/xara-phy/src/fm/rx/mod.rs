pub mod ctrl;
pub mod frontend;
pub mod ofdm;
pub mod receiver;

pub use crate::ofdm::FS;
use crate::ofdm::{NCP, NFFT};
pub const FS6: f64 = 6000.0;
pub const DEC: i64 = 8;
pub const NF: usize = NFFT / DEC as usize;
pub const CP: i64 = NCP as i64 / DEC;
pub const SYMLEN: i64 = NF as i64 + CP;

pub const fn to48(m: i64) -> i64 {
    m * DEC
}
