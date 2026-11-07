pub(crate) const NO_SNR: f32 = 1e35;

pub(crate) fn rint(x: f64) -> i64 {
    x.round_ties_even() as i64
}

pub(crate) fn dec1(x: f32) -> f64 {
    (f64::from(x) * 10.0).round() / 10.0
}
