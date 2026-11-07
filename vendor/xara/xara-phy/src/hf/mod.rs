pub mod ctrl;
pub mod frame;
pub mod fsk;
pub mod level;
pub mod ofdm;
pub mod psk;
pub mod rx;
pub mod tx;

use crate::dat::{RecordTables, hf_tables};
use crate::seq;
use level::Bandwidth;
use num_complex::Complex64;
use std::f64::consts::PI;
use std::sync::{LazyLock, OnceLock};

type Phasors = Vec<Vec<Complex64>>;

static TABLES: [OnceLock<Vec<RecordTables>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
static WHITE: LazyLock<Vec<u8>> = LazyLock::new(|| seq::whitening(seq::HF_SEED));
static PSK_PHASORS: LazyLock<Phasors> = LazyLock::new(|| phasors(seq::psk_phasor_index()));
static OFDM_PHASORS: LazyLock<Phasors> = LazyLock::new(|| phasors(seq::ofdm_phasor_index()));

fn phasors<const N: usize>(idx: Vec<[u8; N]>) -> Phasors {
    idx.iter()
        .map(|r| r.iter().map(|&d| Complex64::from_polar(1.0, 2.0 * PI * f64::from(d) / 128.0)).collect())
        .collect()
}

pub fn tables(bw: Bandwidth) -> &'static [RecordTables] {
    TABLES[bw as usize].get_or_init(|| hf_tables(bw))
}

pub fn whitening() -> &'static [u8] {
    &WHITE
}

pub fn warm(bw: Bandwidth) {
    tables(bw);
    whitening();
    psk_phasors();
    ofdm_phasors();
}

pub fn psk_phasors() -> &'static [Vec<Complex64>] {
    &PSK_PHASORS
}

pub fn ofdm_phasors() -> &'static [Vec<Complex64>] {
    &OFDM_PHASORS
}
