use std::f32::consts::TAU;

/// Hann window: a 2-bin main lobe with -31 dB first sidelobes.
///
/// Chosen over [`blackman_harris`] wherever two close tones have to be told
/// apart rather than one weak tone dug out of leakage — Blackman-Harris buys its
/// -92 dB sidelobes with a main lobe twice as wide, which is the wrong trade
/// when the pair in question is 67.0 and 69.3 Hz.
pub fn hann(n: usize) -> Vec<f32> {
    (0..n).map(|i| 0.5 - 0.5 * (TAU * i as f32 / (n as f32 - 1.0)).cos()).collect()
}

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

/// The derivative of [`blackman_harris`] per sample, for the reassigned
/// spectrogram's second transform (see `SpectrumAnalyzer::set_reassign`).
pub fn blackman_harris_derivative(n: usize) -> Vec<f32> {
    const A: [f32; 4] = [0.35875, 0.48829, 0.14128, 0.01168];
    if n < 2 {
        return vec![0.0; n];
    }
    let w = TAU / (n as f32 - 1.0);
    (0..n)
        .map(|i| {
            let x = w * i as f32;
            w * (A[1] * x.sin() - 2.0 * A[2] * (2.0 * x).sin() + 3.0 * A[3] * (3.0 * x).sin())
        })
        .collect()
}
