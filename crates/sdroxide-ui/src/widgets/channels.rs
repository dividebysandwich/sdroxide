//! Channel radio on the waterfall: a row of channel tiles along the band-plan
//! strip wherever a channelised licence-free service (CB, Freenet, PMR446) is
//! in view. Which services and which channels is national law, so the radio's
//! country ([`Country`], set in Settings → General under the IARU region)
//! chooses the plans; with no country set nothing is drawn.
// Why: HDZ 2026-11-08: "können wir vorher testen, wie Markierungen sich auf
//      dem normalen Wasserfall ausmachen würden" → tiles; HDZ 2026-11-09:
//      "Freenet und PMR als Bandplan mit der Regions-Erklärung Deutschland in
//      Preferences, wo der Locator ist … das Land steuert die CB-, Freenet-
//      und PMR-Kanalraster".

use eframe::egui::{Color32, FontId, Painter, Rect, Stroke, pos2};

use crate::view::ViewState;

/// Which country's licence-free channel plans a radio draws (see
/// `ViewState::country`). Unset draws none, as before the setting existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Country {
    #[default]
    Unset,
    /// CB 1–80 (BNetzA Vfg. 21/2021), Freenet (Vfg. 45/2025), PMR446
    /// (ECC/DEC/(15)05).
    Germany,
}

impl Country {
    pub const ALL: [Country; 2] = [Country::Unset, Country::Germany];
}

/// One channel: its number, carrier frequency, and whether the plan sets it
/// aside for data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub n: u16,
    pub hz: f64,
    pub data: bool,
}

/// German CB, channels 1–80, from the Allgemeinzuteilung BNetzA Vfg. 21/2021
/// § 1: 1–40 are the CEPT channels with the US numbering quirk (23 above 24
/// and 25), 41–80 the national extension 26.565–26.955 MHz, FM only (§ 2 (2)).
/// Data channels are those of footnote 1 and § 2 (3).
const CB_DE_1_40_KHZ: [u32; 40] = [
    26_965, 26_975, 26_985, 27_005, 27_015, 27_025, 27_035, 27_055, 27_065, 27_075, 27_085, 27_105,
    27_115, 27_125, 27_135, 27_155, 27_165, 27_175, 27_185, 27_205, 27_215, 27_225, 27_255, 27_235,
    27_245, 27_265, 27_275, 27_285, 27_295, 27_305, 27_315, 27_325, 27_335, 27_345, 27_355, 27_365,
    27_375, 27_385, 27_395, 27_405,
];
const CB_DE_DATA: [u16; 8] = [6, 7, 24, 25, 52, 53, 76, 77];
/// Channel spacing of CB, the width a tile stands for.
pub const CB_SPACING_HZ: f64 = 10_000.0;

pub fn cb_de() -> Vec<Channel> {
    let mut v: Vec<Channel> = CB_DE_1_40_KHZ
        .iter()
        .enumerate()
        .map(|(i, khz)| Channel { n: i as u16 + 1, hz: f64::from(*khz) * 1e3, data: false })
        .collect();
    v.extend((41..=80u16).map(|n| Channel {
        n,
        hz: 26_565_000.0 + f64::from(n - 41) * CB_SPACING_HZ,
        data: false,
    }));
    for c in &mut v {
        c.data = CB_DE_DATA.contains(&c.n);
    }
    v
}

/// Freenet (Germany), the six 12.5 kHz channels of BNetzA Vfg. 45/2025
/// (analogue or digital; the 6.25 kHz digital-only channels between them are
/// not drawn): 149.0250, 149.0375, 149.0500, 149.0875, 149.1000, 149.1125 MHz.
const FREENET_KHZ: [f64; 6] = [149_025.0, 149_037.5, 149_050.0, 149_087.5, 149_100.0, 149_112.5];
pub const FREENET_SPACING_HZ: f64 = 12_500.0;

pub fn freenet() -> Vec<Channel> {
    FREENET_KHZ
        .iter()
        .enumerate()
        .map(|(i, khz)| Channel { n: i as u16 + 1, hz: khz * 1e3, data: false })
        .collect()
}

/// PMR446, the 16 analogue channels of ECC Decision (15)05 (amended 2018):
/// 446.00625 MHz upwards in 12.5 kHz steps to 446.19375 MHz.
pub const PMR446_SPACING_HZ: f64 = 12_500.0;

pub fn pmr446() -> Vec<Channel> {
    (1..=16u16)
        .map(|n| Channel {
            n,
            hz: 446_006_250.0 + f64::from(n - 1) * PMR446_SPACING_HZ,
            data: false,
        })
        .collect()
}

/// The channel plans `country` has, with their channel spacing; none
/// without a country.
// Why: HDZ 2026-11-08: "die Channel-Marker sind gut, kannst du die für
//      Freenet und PMR noch machen".
pub fn plans(country: Country) -> Vec<(Vec<Channel>, f64)> {
    match country {
        Country::Unset => Vec::new(),
        Country::Germany => vec![
            (cb_de(), CB_SPACING_HZ),
            (freenet(), FREENET_SPACING_HZ),
            (pmr446(), PMR446_SPACING_HZ),
        ],
    }
}

/// The services as blocks on the band-plan strip, (low, high, name) in Hz,
/// beyond the CEPT CB block (channels 1–40) the strip always draws: each
/// channel's half spacing either side of the outer carriers.
pub fn strip_blocks(country: Country) -> Vec<(f64, f64, &'static str)> {
    match country {
        Country::Unset => Vec::new(),
        Country::Germany => vec![
            (26_560_000.0, 26_960_000.0, "CB 41–80"),
            (149_018_750.0, 149_118_750.0, "Freenet"),
            (446_000_000.0, 446_200_000.0, "PMR446"),
        ],
    }
}

/// Widest view the row is drawn in: past this the tiles are slivers.
const MAX_SPAN_HZ: f64 = 2_500_000.0;

/// Whether the row shows in this view.
pub fn shown(view: &ViewState) -> bool {
    let span = view.span();
    span > 0.0
        && span <= MAX_SPAN_HZ
        && plans(view.country)
            .iter()
            .flat_map(|(v, _)| v.iter())
            .any(|c| (view.view_lo_hz..=view.view_hi_hz).contains(&c.hz))
}

/// Every how many channels a number fits, given the pixels one channel takes
/// and the width of a two-digit label.
pub fn label_step(px_per_channel: f32, label_w: f32) -> u16 {
    [1u16, 2, 5, 10]
        .into_iter()
        .find(|s| f32::from(*s) * px_per_channel >= label_w + 4.0)
        .unwrap_or(20)
}

/// Draw the row of tiles at `top`, `h` high.
pub fn draw(p: &Painter, view: &ViewState, wf: &Rect, top: f32, h: f32, font: f32) {
    for (chans, spacing) in plans(view.country) {
        draw_plan(p, view, wf, top, h, font, &chans, spacing);
    }
}

fn draw_plan(
    p: &Painter,
    view: &ViewState,
    wf: &Rect,
    top: f32,
    h: f32,
    font: f32,
    chans: &[Channel],
    spacing: f64,
) {
    let px = (view.freq_to_x(spacing, wf) - view.freq_to_x(0.0, wf)).abs();
    let probe = p.layout_no_wrap("80".into(), FontId::proportional(font), Color32::WHITE);
    let step = label_step(px, probe.size().x);
    let teal = Color32::from_rgb(20, 64, 60);
    let amber = Color32::from_rgb(90, 67, 16);
    for c in chans {
        let (lo, hi) = (c.hz - 0.45 * spacing, c.hz + 0.45 * spacing);
        if hi <= view.view_lo_hz || lo >= view.view_hi_hz {
            continue;
        }
        let x0 = view.freq_to_x(lo, wf).max(wf.left());
        let x1 = view.freq_to_x(hi, wf).min(wf.right());
        if x1 - x0 < 1.0 {
            continue;
        }
        let r = Rect::from_min_max(pos2(x0, top), pos2(x1, top + h));
        p.rect_filled(r, 2.0, if c.data { amber } else { teal });
        // The two CB channels everyone knows keep their number when the rest
        // thin.
        let keep = c.n % step == 0 || ((c.n == 9 || c.n == 19) && step <= 2);
        if keep {
            let col = if c.data {
                Color32::from_rgb(255, 176, 0)
            } else {
                Color32::from_rgb(79, 209, 197)
            };
            let g = p.layout_no_wrap(c.n.to_string(), FontId::proportional(font), col);
            let pos = pos2((x0 + x1) * 0.5 - g.size().x * 0.5, top + (h - g.size().y) * 0.5);
            p.galley(pos, g, col);
        }
        // The channel's centre, a hairline up into the waterfall's edge.
        let xc = view.freq_to_x(c.hz, wf);
        if (wf.left()..=wf.right()).contains(&xc) {
            p.vline(
                xc,
                (top - 4.0)..=top,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(79, 209, 197, 160)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cb_plan_has_eighty_channels_with_the_23_quirk() {
        let v = cb_de();
        assert_eq!(v.len(), 80);
        let at = |n: u16| v.iter().find(|c| c.n == n).unwrap().hz;
        assert_eq!(at(1), 26_965_000.0);
        assert_eq!(at(23), 27_255_000.0);
        assert_eq!(at(24), 27_235_000.0);
        assert_eq!(at(40), 27_405_000.0);
        assert_eq!(at(41), 26_565_000.0);
        assert_eq!(at(80), 26_955_000.0);
        // Vfg. 21/2021, footnote 1 and § 2 (3): 40 and 41 are relay
        // channels (footnote 2), not data channels.
        let data: Vec<u16> = v.iter().filter(|c| c.data).map(|c| c.n).collect();
        assert_eq!(data, [6, 7, 24, 25, 52, 53, 76, 77]);
    }

    /// The country chooses the plans: none without one, all three in
    /// Germany, and the strip blocks follow.
    #[test]
    fn the_country_chooses_the_plans() {
        assert!(plans(Country::Unset).is_empty());
        assert!(strip_blocks(Country::Unset).is_empty());
        assert_eq!(plans(Country::Germany).len(), 3);
        let b = strip_blocks(Country::Germany);
        assert!(b.iter().any(|(lo, hi, _)| *lo < 26_565_000.0 && *hi > 26_955_000.0));
        assert!(b.iter().any(|(lo, hi, _)| *lo < 149_025_000.0 && *hi > 149_112_500.0));
        assert!(b.iter().any(|(lo, hi, _)| *lo < 446_006_250.0 && *hi > 446_193_750.0));
    }

    #[test]
    fn freenet_and_pmr446_plans() {
        let f = freenet();
        assert_eq!(f.len(), 6);
        assert_eq!(f[0].hz, 149_025_000.0);
        assert_eq!(f[5].hz, 149_112_500.0);
        let p = pmr446();
        assert_eq!(p.len(), 16);
        assert_eq!(p[0].hz, 446_006_250.0);
        assert_eq!(p[15].hz, 446_193_750.0);
    }

    #[test]
    fn numbers_thin_as_the_channels_narrow() {
        assert_eq!(label_step(28.0, 12.0), 1);
        assert_eq!(label_step(15.0, 12.0), 2);
        assert_eq!(label_step(5.0, 12.0), 5);
    }
}
