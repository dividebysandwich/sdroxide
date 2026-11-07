use num_complex::Complex64;
use std::f64::consts::SQRT_2;
use std::sync::LazyLock;

const fn ginv(mut n: u32) -> u32 {
    let mut s = 1;
    while s < 8 {
        n ^= n >> s;
        s <<= 1;
    }
    n
}

fn amp(p: f64, l: f64, x: i32) -> f64 {
    f64::from((p * f64::from(x) / (l * SQRT_2)) as f32)
}

fn square(bps: u32, p: f64, l: f64) -> Vec<Complex64> {
    let h = bps / 2;
    let m = (1 << h) - 1;
    let c = |g: u32| 2 * ginv(g) as i32 - m as i32;
    (0..1u32 << bps).map(|v| Complex64::new(amp(p, l, c(v >> h)), amp(p, l, c(v & m)))).collect()
}

fn signed(v: u32, si: u32, sq: u32, x: i32, y: i32, p: f64, l: f64) -> Complex64 {
    let sx = if v & si != 0 { -x } else { x };
    let sy = if v & sq != 0 { -y } else { y };
    Complex64::new(amp(p, l, sx), amp(p, l, sy))
}

fn bit(v: u32, b: u32) -> u32 {
    (v >> b) & 1
}

static TABLES: LazyLock<[Vec<Complex64>; 9]> = LazyLock::new(|| {
    let c = Complex64::new;
    let a = f64::from(std::f64::consts::FRAC_1_SQRT_2 as f32);
    let q8 = [(-1, -1), (-1, 0), (0, -1), (-1, 1), (1, 0), (1, 1), (1, -1), (0, 1)];
    let q32 = [(1, 1), (3, 1), (3, 5), (5, 1), (1, 3), (3, 3), (1, 5), (5, 3)];
    let p128 = |v: u32| {
        let x = 2 * ginv(bit(v, 3) << 2 | bit(v, 2) << 1 | bit(v, 0)) as i32 + 1;
        let y = 2 * ginv(bit(v, 4) << 1 | bit(v, 1)) as i32 + 1;
        let (x, y) = if x >= 13 { if y <= 3 { (x - 8, y + 8) } else { (16 - x, 16 - y) } } else { (x, y) };
        signed(v, 64, 32, x, y, 1.8, 11.0)
    };
    let p256 = |v: u32| {
        let x = 2 * ginv(bit(v, 4) << 2 | bit(v, 2) << 1 | bit(v, 0)) as i32 + 1;
        let y = 2 * ginv(bit(v, 5) << 2 | bit(v, 3) << 1 | bit(v, 1)) as i32 + 1;
        signed(v, 128, 64, x, y, 1.7, 15.0)
    };
    [
        vec![],
        vec![c(1.0, 0.0), c(-1.0, 0.0)],
        vec![c(1.0, 0.0), c(0.0, 1.0), c(0.0, -1.0), c(-1.0, 0.0)],
        q8.iter().map(|&(i, q)| c(a * f64::from(i), a * f64::from(q))).collect(),
        square(4, 1.1, 3.0),
        (0..32).map(|v| signed(v, 16, 8, q32[v as usize & 7].0, q32[v as usize & 7].1, 1.5, 5.0)).collect(),
        square(6, 1.6, 7.0),
        (0..128).map(p128).collect(),
        (0..256).map(p256).collect(),
    ]
});

pub fn constellation(bps: u32) -> &'static [Complex64] {
    TABLES.get(bps as usize).map_or(&[], Vec::as_slice)
}
