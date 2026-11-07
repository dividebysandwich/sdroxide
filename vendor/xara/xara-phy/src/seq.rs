use crate::rnd::VbRnd;

pub const HF_SEED: f64 = 292.0;
pub const FM_SEED: f64 = 43.0;
pub const WHITENING_LEN: usize = 150_000;
pub const PSK_CARRIERS: usize = 13;
pub const PSK_SYMBOLS: usize = 471;
pub const OFDM_BINS: usize = 128;
pub const OFDM_SYMBOLS: usize = 469;
pub const DATA_DAT_LEN: usize = 65_000;

pub fn whitening(seed: f64) -> Vec<u8> {
    let mut r = VbRnd::stream(seed);
    (0..WHITENING_LEN).map(|_| r.rand_int(1) as u8).collect()
}

pub fn psk_phasor_index() -> Vec<[u8; PSK_SYMBOLS]> {
    let mut r = VbRnd::stream(HF_SEED);
    let mut d = vec![[0u8; PSK_SYMBOLS]; PSK_CARRIERS];
    for ks in [0..411, 411..PSK_SYMBOLS] {
        for row in d.iter_mut() {
            for k in ks.clone() {
                row[k] = r.rand_int(128) as u8;
            }
        }
    }
    d
}

pub fn data_dat() -> Vec<u8> {
    let mut r = VbRnd::stream(HF_SEED);
    (0..DATA_DAT_LEN).map(|_| r.rand_int(128) as u8).collect()
}

pub fn ofdm_phasor_index() -> Vec<[u8; OFDM_SYMBOLS]> {
    let d = data_dat();
    let mut it = d.iter().copied();
    let mut out = vec![[0u8; OFDM_SYMBOLS]; OFDM_BINS];
    for ms in [0..409, 409..OFDM_SYMBOLS] {
        for row in out.iter_mut() {
            for m in ms.clone() {
                row[m] = it.next().unwrap();
            }
        }
    }
    out
}
