//! Waterfall colormap LUTs: 256×1 RGBA8.

pub const NAMES: [&str; 3] = ["Classic", "Viridis", "Gray"];

/// Piecewise-linear gradient through (position, RGB) anchor points.
/// Anchors must start at 0.0 and end at 1.0.
fn gradient(anchors: &[(f32, [u8; 3])]) -> [u8; 256 * 4] {
    let mut out = [0u8; 256 * 4];
    for i in 0..256 {
        let t = i as f32 / 255.0;
        let seg = anchors
            .windows(2)
            .find(|w| t <= w[1].0)
            .unwrap_or(&anchors[anchors.len() - 2..]);
        let (t0, c0) = seg[0];
        let (t1, c1) = seg[1];
        let f = if t1 > t0 { ((t - t0) / (t1 - t0)).clamp(0.0, 1.0) } else { 0.0 };
        for ch in 0..3 {
            out[i * 4 + ch] = (c0[ch] as f32 + f * (c1[ch] as f32 - c0[ch] as f32)) as u8;
        }
        out[i * 4 + 3] = 255;
    }
    out
}

pub fn lut(index: usize) -> [u8; 256 * 4] {
    match index {
        // PowerSDR-style: black → blue → cyan → green → yellow → red → white
        0 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.25, [0, 0, 160]),
            (0.45, [0, 180, 200]),
            (0.60, [40, 200, 60]),
            (0.75, [230, 230, 40]),
            (0.90, [240, 60, 30]),
            (1.00, [255, 255, 255]),
        ]),
        // Viridis approximation
        1 => gradient(&[
            (0.00, [68, 1, 84]),
            (0.25, [59, 82, 139]),
            (0.50, [33, 145, 140]),
            (0.75, [94, 201, 98]),
            (1.00, [253, 231, 37]),
        ]),
        _ => gradient(&[(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]),
    }
}
