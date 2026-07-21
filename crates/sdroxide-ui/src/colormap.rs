//! Waterfall colormap LUTs: 256×1 RGBA8.

pub const NAMES: [&str; 8] =
    ["Classic", "Viridis", "Gray", "Icom", "Neon", "Synthwave", "Matrix", "Tron"];

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
        // Icom SDR waterfall: floor black, rising through blue → cyan → green →
        // yellow → orange, peaking at red (no white blow-out at the top).
        3 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.12, [0, 0, 92]),
            (0.30, [0, 40, 210]),
            (0.46, [0, 170, 220]),
            (0.58, [0, 210, 180]),
            (0.70, [40, 210, 40]),
            (0.83, [235, 235, 30]),
            (0.93, [242, 130, 20]),
            (1.00, [230, 20, 20]),
        ]),
        // Neon — cyberpunk magenta-and-cyan glow: black → violet → magenta →
        // hot pink → neon cyan → white.
        4 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.18, [24, 0, 48]),
            (0.38, [96, 0, 140]),
            (0.56, [210, 0, 190]),
            (0.72, [255, 44, 130]),
            (0.86, [70, 220, 255]),
            (1.00, [235, 255, 255]),
        ]),
        // Synthwave — retro-future sunset: deep indigo → purple → magenta →
        // coral → orange → hot yellow.
        5 => gradient(&[
            (0.00, [8, 0, 20]),
            (0.22, [58, 0, 92]),
            (0.42, [150, 12, 130]),
            (0.60, [240, 40, 110]),
            (0.75, [255, 96, 74]),
            (0.88, [255, 158, 44]),
            (1.00, [255, 232, 120]),
        ]),
        // Matrix — green phosphor rain: black → dim green → green → bright
        // green → pale green.
        6 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.30, [0, 36, 8]),
            (0.55, [0, 150, 40]),
            (0.78, [46, 240, 88]),
            (1.00, [200, 255, 205]),
        ]),
        // Tron — electric grid: black → deep blue → cyan → white, spiking to an
        // amber peak.
        7 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.28, [0, 18, 58]),
            (0.52, [0, 168, 232]),
            (0.72, [120, 238, 255]),
            (0.86, [244, 252, 255]),
            (1.00, [255, 150, 26]),
        ]),
        // Gray (index 2) and any out-of-range fallback.
        _ => gradient(&[(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]),
    }
}
