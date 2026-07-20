use std::f32::consts::TAU;

/// 4-term Blackman-Harris window (-92 dB sidelobes).
pub fn blackman_harris(n: usize) -> Vec<f32> {
    const A: [f32; 4] = [0.35875, 0.48829, 0.14128, 0.01168];
    (0..n)
        .map(|i| {
            let x = TAU * i as f32 / (n as f32 - 1.0);
            A[0] - A[1] * x.cos() + A[2] * (2.0 * x).cos() - A[3] * (3.0 * x).cos()
        })
        .collect()
}
