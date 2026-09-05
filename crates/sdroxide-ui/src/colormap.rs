//! Waterfall colormap LUTs: 256×1 RGBA8.

pub const NAMES: [&str; 10] = [
    "Classic",
    "Viridis",
    "Gray",
    "Icom",
    "Neon",
    "Synthwave",
    "Matrix",
    "Tron",
    "Amber",
    "Rainbow",
];

/// Piecewise-linear gradient through (position, RGB) anchor points.
/// Anchors must start at 0.0 and end at 1.0.
fn gradient(anchors: &[(f32, [u8; 3])]) -> [u8; 256 * 4] {
    let mut out = [0u8; 256 * 4];
    for i in 0..256 {
        let t = i as f32 / 255.0;
        let seg = anchors.windows(2).find(|w| t <= w[1].0).unwrap_or(&anchors[anchors.len() - 2..]);
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
        // Amber — the waterfall to wear with the Amber Phosphor UI theme: a
        // single warm phosphor family, black through ember and amber to a
        // white-hot peak, with no second hue anywhere in it. Anchored on that
        // theme's own inks: 0xffb000 is its accent and 0xffeacc its strong
        // text, so a loud signal here is the same amber as the chrome around
        // it.
        8 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.18, [26, 12, 0]),
            (0.38, [92, 42, 0]),
            (0.56, [176, 86, 0]),
            (0.72, [255, 176, 0]),
            (0.88, [255, 210, 96]),
            (1.00, [255, 234, 204]),
        ]),
        // Rainbow — the full spectrum in order, to wear with the Rainbow UI
        // theme: black into violet, then blue, cyan, green, yellow, orange and
        // red, ending white-hot. Unlike Classic it spends no range on a long
        // dark blue run, so the noise floor shows its texture and every step
        // up the scale changes hue.
        9 => gradient(&[
            (0.00, [0, 0, 0]),
            (0.10, [40, 0, 70]),
            (0.22, [90, 0, 200]),
            (0.34, [0, 80, 255]),
            (0.46, [0, 200, 230]),
            (0.58, [0, 220, 80]),
            (0.70, [200, 240, 0]),
            (0.80, [255, 180, 0]),
            (0.90, [255, 70, 30]),
            (1.00, [255, 255, 255]),
        ]),
        // Gray (index 2) and any out-of-range fallback.
        _ => gradient(&[(0.0, [0, 0, 0]), (1.0, [255, 255, 255])]),
    }
}

// ── Propagation ─────────────────────────────────────────────────────────────

/// The propagation heat ramp: blue → green → yellow → red.
///
/// Not one of the waterfall palettes. Those start at black because a waterfall
/// is mostly noise floor and the empty parts should recede; a propagation cell
/// with no evidence is drawn with no alpha at all rather than in black, so this
/// ramp spends its whole range on the four steps an operator reads as "barely",
/// "workable", "good", "loud".
///
/// This is the one definition of that ramp. The globe samples it as a 256×1
/// texture and the flat map indexes the same array on the CPU, so the two
/// cannot drift apart — which they would within a week if the anchors were
/// written out again in WGSL.
pub fn prop_ramp() -> [u8; 256 * 4] {
    gradient(&[
        (0.00, [26, 62, 190]),  // blue
        (0.35, [30, 180, 150]), // teal
        (0.55, [60, 210, 70]),  // green
        (0.78, [235, 220, 45]), // yellow
        (1.00, [235, 45, 40]),  // red
    ])
}

/// Look the ramp up at `t` in 0..1.
pub fn prop_ramp_at(t: f32) -> [u8; 3] {
    let lut = prop_ramp();
    let i = ((t.clamp(0.0, 1.0) * 255.0) as usize) * 4;
    [lut[i], lut[i + 1], lut[i + 2]]
}

/// A hue per band, for the all-bands propagation display.
///
/// Band is a categorical variable that happens to have an order, and running it
/// around the hue circle low-to-high is how every propagation site in the hobby
/// already draws it — so an operator reads this without being taught. A rainbow
/// would be the wrong choice for a continuous quantity; the per-band display
/// exists precisely for when the hues cannot be told apart, and the legend
/// names each band beside its swatch rather than relying on hue recall.
///
/// One lap of the circle only holds so many bands. 160 m through 70 cm take it
/// at full saturation, deep red to rose; the microwave bands carry on round the
/// same way — salmon, amber, straw, green, sky — but pale and washed out. So the
/// second lap cannot be mistaken for the first: 23 cm is a pale amber that no HF
/// band comes near, rather than an orange the eye has to tell from 80 m's.
pub fn band_color(band: sdroxide_types::Band) -> [u8; 3] {
    use sdroxide_types::Band;
    match band {
        Band::M160 => [176, 40, 40],  // deep red
        Band::M80 => [214, 92, 32],   // orange
        Band::M60 => [222, 150, 40],  // amber
        Band::M40 => [226, 208, 52],  // yellow
        Band::M30 => [150, 214, 60],  // yellow-green
        Band::M20 => [58, 200, 96],   // green
        Band::M17 => [46, 200, 170],  // teal
        Band::M15 => [52, 168, 226],  // sky
        Band::M12 => [70, 116, 232],  // blue
        Band::M10 => [122, 92, 236],  // indigo
        Band::M6 => [176, 84, 226],   // violet
        Band::M4 => [202, 78, 224],   // purple
        Band::M2 => [226, 76, 190],   // magenta
        Band::M125 => [232, 84, 166], // magenta-rose
        Band::M70 => [236, 96, 140],  // rose
        // The microwave bands: the same rotation, carried on pale.
        Band::Cm33 => [250, 152, 122], // pale salmon
        Band::Cm23 => [246, 192, 112], // pale amber
        Band::Cm13 => [226, 224, 140], // pale straw
        Band::Cm9 => [160, 220, 172],  // pale green
        Band::Cm6 => [150, 204, 236],  // pale sky
        Band::Cm3 => [166, 178, 240],  // pale blue
        // Not a band: nothing is ever binned here.
        Band::Gen => [128, 128, 128],
    }
}

#[cfg(test)]
mod lut_tests {
    use super::*;

    /// A palette added to [`NAMES`] without a matching arm in [`lut`] falls
    /// through to the Gray fallback and is silently the wrong picture — the
    /// name is in the combo, the waterfall is grey. Gray itself (index 2) is
    /// the one that is meant to look like that.
    #[test]
    fn every_named_palette_has_its_own_lut() {
        let gray = lut(2);
        for (i, name) in NAMES.iter().enumerate() {
            if i == 2 {
                continue;
            }
            assert!(lut(i) != gray, "{name} (index {i}) has no arm in lut() — it is drawing Gray");
        }
    }

    /// Every waterfall palette has to start dark and end brighter than it
    /// started: the noise floor is most of the picture and it is the low end,
    /// so a ramp that starts bright paints a wall. Not monotone all the way —
    /// Icom deliberately comes back *down* to red at the top rather than
    /// blowing out to white — so only the two ends are pinned.
    #[test]
    fn every_palette_runs_dark_to_bright() {
        let sum = |c: &[u8; 256 * 4], i: usize| {
            c[i * 4] as u32 + c[i * 4 + 1] as u32 + c[i * 4 + 2] as u32
        };
        for (i, name) in NAMES.iter().enumerate() {
            let c = lut(i);
            assert!(sum(&c, 0) < sum(&c, 128), "{name}: the floor is not darker than mid-scale");
            assert!(sum(&c, 0) < sum(&c, 255), "{name}: the peak is not brighter than the floor");
            assert!(c.chunks_exact(4).all(|p| p[3] == 255), "{name}: a transparent entry");
        }
    }

    /// Amber is the waterfall for the Amber Phosphor theme, so it has to stay
    /// in that one phosphor family — no blue anywhere, and never bluer than it
    /// is red.
    #[test]
    fn the_amber_palette_stays_amber() {
        let amber = NAMES.iter().position(|n| *n == "Amber").expect("an Amber palette");
        for p in lut(amber).chunks_exact(4) {
            assert!(p[2] <= p[1] && p[1] <= p[0], "not a warm ramp: {:?}", &p[..3]);
        }
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;

    /// The ramp has to actually run blue → green → yellow → red, because that
    /// order is the whole of what it communicates.
    #[test]
    fn the_propagation_ramp_runs_cold_to_hot() {
        let cold = prop_ramp_at(0.0);
        let warm = prop_ramp_at(0.6);
        let hot = prop_ramp_at(1.0);
        assert!(cold[2] > cold[0], "the cold end is not blue: {cold:?}");
        assert!(warm[1] > warm[0] && warm[1] > warm[2], "the middle is not green: {warm:?}");
        assert!(hot[0] > hot[1] && hot[0] > hot[2], "the hot end is not red: {hot:?}");
    }

    /// Every band gets its own hue, or two bands on one cell would be
    /// indistinguishable in the combined display.
    #[test]
    fn no_two_bands_share_a_colour() {
        use sdroxide_types::Band;
        let mut seen: Vec<[u8; 3]> = Vec::new();
        for b in Band::ALL {
            if b == Band::Gen {
                continue;
            }
            let c = band_color(b);
            assert!(!seen.contains(&c), "{} reuses {c:?}", b.label());
            seen.push(c);
        }
    }
}
