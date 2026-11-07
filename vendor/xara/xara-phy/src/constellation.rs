use num_complex::Complex64;
use std::f64::consts::FRAC_1_SQRT_2;
use std::sync::LazyLock;

const QAM16: [f64; 4] = [-0.7778175, -0.2592725, 0.7778175, 0.2592725];
const QAM32: [f64; 3] = [0.2121320, 0.6363961, 1.0606602];

static TABLES: LazyLock<[Vec<Complex64>; 6]> = LazyLock::new(|| {
    let c = Complex64::new;
    let h = FRAC_1_SQRT_2;
    let base: Vec<Complex64> = [(0, 0), (1, 0), (1, 2), (2, 0), (0, 1), (1, 1), (0, 2), (2, 1)]
        .iter()
        .map(|&(i, q)| c(QAM32[i], QAM32[q]))
        .collect();
    let qam32 = [
        base.clone(),
        base.iter().map(|z| z.conj()).collect(),
        base.iter().map(|z| -z.conj()).collect(),
        base.iter().map(|z| -z).collect(),
    ]
    .concat();
    [
        vec![],
        vec![c(1.0, 0.0), c(-1.0, 0.0)],
        vec![c(1.0, 0.0), c(0.0, 1.0), c(0.0, -1.0), c(-1.0, 0.0)],
        vec![c(1.0, 0.0), c(h, h), c(-h, h), c(0.0, 1.0), c(h, -h), c(0.0, -1.0), c(-1.0, 0.0), c(-h, -h)],
        (0..16).map(|v| c(QAM16[v >> 2], QAM16[v & 3])).collect(),
        qam32,
    ]
});

pub fn constellation(bps: u32) -> &'static [Complex64] {
    TABLES.get(bps as usize).map_or(&[], Vec::as_slice)
}

pub(crate) fn take_bits(tx: &[u8], ptr: &mut usize, n: u32) -> usize {
    let mut v = 0;
    for _ in 0..n {
        v = 2 * v + usize::from(tx.get(*ptr).copied().unwrap_or(0));
        *ptr += 1;
    }
    v
}
