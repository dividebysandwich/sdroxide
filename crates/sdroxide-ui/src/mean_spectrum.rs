//! S/N guide: every frame of the panadapter accumulated into one mean per
//! frequency, drawn as how far each frequency stands above the noise floor.
//! The mean only ever grows smoother, so the noise flattens out with time
//! while a faint steady carrier keeps its height and stands out of it.
// Why: HDZ 2026-11-07: "ein Minispektrum zu überlagern, welches ein weighted
//      Mittel akkumuliert … sodass man faint structures sehen kann"; "an der
//      Position, wo vorher im Korr-Spektrum die Null war, in der grünen Farbe
//      darübergelegt ein S/N-bereinigtes akkumulatives Signal … als Guide im
//      gleichen Grün, Höhe vielleicht 1/4 vom Original, akkumulierend als
//      weighted average immer weiter glättend".

use sdroxide_types::SpectrumFrame;

/// Where the noise floor is read among the cells' means, from the lowest:
/// the tenth. The median would do on a quiet band, but on a full broadcast
/// band the stations are half the cells and lift it, and the guide would sit
/// too high; the lowest tenth stays in the gaps between them.
// Why: HDZ 2026-11-07: "das ist ein guter Call" (lowest tenth instead of the
//      median, for full bands).
const FLOOR_QUANTILE: f64 = 0.1;

/// Share of the reached cells at each end of the window left out of the floor:
/// a narrow I/Q window (a KiwiSDR's 12 kHz) falls away at its edges under the
/// front end's filter, and those skirts are not the noise.
const FLOOR_EDGE_SHARE: f64 = 0.1;

/// The noise floor of a row of mean powers: the [`FLOOR_QUANTILE`] of the
/// reached cells, the outer [`FLOOR_EDGE_SHARE`] at each end left out.
// Why: HDZ 2026-11-07, a KiwiSDR on 5505 kHz: "dort ist das S/N weird … zu
//      hohe Werte" — the floor was the filter skirts, and the whole window
//      stood 10–16 dB over it.
fn floor_of(power: &[f64]) -> Option<f64> {
    let reached: Vec<f64> = power.iter().copied().filter(|p| p.is_finite() && *p > 0.0).collect();
    let cut = (reached.len() as f64 * FLOOR_EDGE_SHARE) as usize;
    let mut mid: Vec<f64> = reached.get(cut..reached.len() - cut)?.to_vec();
    if mid.len() < 8 {
        return None;
    }
    let q = ((mid.len() - 1) as f64 * FLOOR_QUANTILE) as usize;
    Some(*mid.select_nth_unstable_by(q, |a, b| a.total_cmp(b)).1)
}

/// Where the app leaves the guide for the spectrum widget (`Arc<Guide>`).
pub fn guide_id() -> eframe::egui::Id {
    eframe::egui::Id::new("snr-guide")
}

/// What the guide draws: S/N in dB per cell (NaN where nothing was seen).
#[derive(Debug, Clone, PartialEq)]
pub struct Guide {
    pub lo_hz: f64,
    pub cell_hz: f64,
    pub snr_db: Vec<f32>,
    /// Frames in each cell's mean: cells that came into view when the grid
    /// was laid anew (the dial carrying the panadapter along) have fewer.
    pub count: Vec<u32>,
    /// The most frames in any cell's mean, for the caption.
    pub frames: u32,
}

/// Values on cells of `src_cell` hertz from `src_lo`, laid onto `n` cells of
/// `dst_cell` hertz from `dst_lo`. A cell takes the mean of the source cells
/// whose middles fall in it (a finer source), else the source cell its own
/// middle lies in (a coarser one); NaN outside the source and where the
/// source has no value.
fn resample(
    src_lo: f64,
    src_cell: f64,
    src: &[f64],
    dst_lo: f64,
    dst_cell: f64,
    n: usize,
) -> Vec<f64> {
    let len = src.len();
    (0..n)
        .map(|c| {
            let a = dst_lo + c as f64 * dst_cell;
            let b = a + dst_cell;
            let first = ((a - src_lo) / src_cell - 0.5).ceil().max(0.0) as usize;
            let last = (((b - src_lo) / src_cell - 0.5).ceil().max(0.0) as usize).min(len);
            if first < last {
                let seen: Vec<f64> =
                    src[first..last].iter().copied().filter(|v| v.is_finite()).collect();
                if seen.is_empty() {
                    f64::NAN
                } else {
                    seen.iter().sum::<f64>() / seen.len() as f64
                }
            } else {
                let i = ((a + b) / 2.0 - src_lo) / src_cell;
                if i >= 0.0 && i < len as f64 { src[i as usize] } else { f64::NAN }
            }
        })
        .collect()
}

/// How long the mean remembers, in seconds: a frame this old weighs 1/e of
/// a new one, so what has left the band fades out of the guide.
// Why: HDZ 2026-11-07: "als weighted Mittel, wo aktuelle Spektrumseinträge
//      wichtiger sind als länger zurückliegende … alte Information faded hier
//      nicht weg".
pub const MEAN_TAU_S: f64 = 30.0;

/// How long the mean takes to let go of a cell whose level has fallen: a frame
/// below the mean pulls it down with this shorter time constant. Shown in dB,
/// a mean of power rises in seconds and falls in minutes — a carrier 12 dB up
/// is at +3 dB two seconds after it appears, but needs four time constants to
/// drop under 1 dB once it has gone. At 6 s that is under half a minute.
// Why: HDZ 2026-11-07: "ein etwas schnellerer Verfall wäre gut, dass das
//      akkumulierte Spektrum besser runtergeht … wir bauen nur auf, nehmen
//      aber relativ langsam Info weg".
pub const MEAN_FALL_TAU_S: f64 = 6.0;

/// The accumulated mean, on a grid fixed in hertz. A frame at the grid's
/// resolution that reaches past it (the dial carrying the panadapter along)
/// gets a grid moved along with the mean carried over. A frame at another
/// resolution (zoom, transform size) starts the mean afresh: a bin of
/// another width holds another share of the noise, so levels from before
/// stand on another floor and would leave a step that never smooths away.
// Why: HDZ 2026-11-07 (picture): "die Kante vom Zoom … zu stark im
//      S/N-Spektrum drin".
#[derive(Debug, Clone)]
pub struct MeanGrid {
    lo_hz: f64,
    cell_hz: f64,
    /// Mean linear power per cell; NaN where no frame reached yet.
    power: Vec<f64>,
    /// Frames in each cell's mean.
    count: Vec<f64>,
    /// When the last frame was added (Unix seconds).
    last_t: Option<f64>,
}

impl Guide {
    /// S/N at `hz`, from the cell it falls in; `None` off the grid or where
    /// nothing has been seen.
    pub fn snr_at(&self, hz: f64) -> Option<f32> {
        let i = ((hz - self.lo_hz) / self.cell_hz).floor();
        if i < 0.0 {
            return None;
        }
        self.snr_db.get(i as usize).copied().filter(|v| v.is_finite())
    }
}

impl MeanGrid {
    /// An empty grid with the span and resolution of `f`.
    pub fn new(f: &SpectrumFrame) -> Option<Self> {
        let n = f.bins.len();
        (n >= 8 && f.span_hz > 0.0).then(|| MeanGrid {
            lo_hz: f.center_hz - f.span_hz / 2.0,
            cell_hz: f.span_hz / n as f64,
            power: vec![f64::NAN; n],
            count: vec![0.0; n],
            last_t: None,
        })
    }

    /// Whether `f` can be laid onto this grid as it is: inside it (half a
    /// cell of slack at the edges) and at its resolution (5 %).
    pub fn fits(&self, f: &SpectrumFrame) -> bool {
        let n = f.bins.len();
        if n == 0 || f.span_hz <= 0.0 {
            return true;
        }
        let edge = f.center_hz - f.span_hz / 2.0;
        let slack = self.cell_hz / 2.0;
        edge >= self.lo_hz - slack
            && edge + f.span_hz <= self.lo_hz + self.power.len() as f64 * self.cell_hz + slack
            && self.same_resolution(f)
    }

    /// Whether `f`'s bins are as wide as the grid's cells, within 5 %.
    fn same_resolution(&self, f: &SpectrumFrame) -> bool {
        let bin = f.span_hz / f.bins.len().max(1) as f64;
        (bin / self.cell_hz - 1.0).abs() <= 0.05
    }

    /// A grid with the span and resolution of `f`: moved along, with this
    /// one's mean and frame counts carried over, when `f` has the grid's
    /// resolution; empty when it has another.
    pub fn relaid(&self, f: &SpectrumFrame) -> Option<Self> {
        let mut g = MeanGrid::new(f)?;
        if !self.same_resolution(f) {
            return Some(g);
        }
        g.last_t = self.last_t;
        let n = g.power.len();
        g.power = resample(self.lo_hz, self.cell_hz, &self.power, g.lo_hz, g.cell_hz, n);
        g.count = resample(self.lo_hz, self.cell_hz, &self.count, g.lo_hz, g.cell_hz, n)
            .into_iter()
            .zip(&g.power)
            .map(|(c, p)| if p.is_finite() && c.is_finite() { c.round().max(1.0) } else { 0.0 })
            .collect();
        Some(g)
    }

    /// Add `f`, received at `now`, to the mean: weighted by time, a frame
    /// [`MEAN_TAU_S`] old counting 1/e of a new one; at the start, while a
    /// cell has few frames, each weighs at least 1/(frames so far), so the
    /// mean fills quickly. Averaged as power, not dB, so the noise floor is
    /// not pulled down by the log.
    pub fn push(&mut self, f: &SpectrumFrame, now: f64) {
        let n = f.bins.len();
        if n == 0 || f.span_hz <= 0.0 {
            return;
        }
        let edge = f.center_hz - f.span_hz / 2.0;
        let lin: Vec<f64> = f
            .bins
            .iter()
            .map(|v| {
                let db = f.db_floor + (f.db_ceil - f.db_floor) * *v as f32 / 255.0;
                10f64.powf(db as f64 / 10.0)
            })
            .collect();
        let row =
            resample(edge, f.span_hz / n as f64, &lin, self.lo_hz, self.cell_hz, self.power.len());
        // Over a gap longer than a second (the app in the background) the
        // mean is taken as having waited a second, not as having aged.
        let dt = self.last_t.map_or(0.0, |t| (now - t).clamp(0.0, 1.0));
        self.last_t = Some(now);
        let fade = 1.0 - (-dt / MEAN_TAU_S).exp();
        let fall = 1.0 - (-dt / MEAN_FALL_TAU_S).exp();
        for ((p, c), x) in self.power.iter_mut().zip(self.count.iter_mut()).zip(row) {
            if !x.is_finite() {
                continue;
            }
            *c += 1.0;
            let w = (1.0 / *c).max(if x < *p { fall } else { fade });
            *p = if p.is_finite() { *p + w * (x - *p) } else { x };
        }
    }

    /// S/N per cell: the mean over the noise floor in dB, the floor being the
    /// lowest tenth of the cells' means ([`FLOOR_QUANTILE`]).
    pub fn guide(&self) -> Option<Guide> {
        let floor = floor_of(&self.power)?;
        let snr_db = self
            .power
            .iter()
            .map(|p| {
                if p.is_finite() && *p > 0.0 {
                    (10.0 * (p / floor).log10()) as f32
                } else {
                    f32::NAN
                }
            })
            .collect();
        let frames = self.count.iter().fold(0.0f64, |m, c| m.max(*c)) as u32;
        let count = self.count.iter().map(|c| *c as u32).collect();
        Some(Guide { lo_hz: self.lo_hz, cell_hz: self.cell_hz, snr_db, count, frames })
    }
}

/// Fixed-frequency services that are no broadcast schedule's business but are
/// worth a label where they come in: NAVTEX and the VOLMET weather broadcasts
/// for aircraft, (name, kHz, mode).
const UTILITY_KHZ: &[(&str, f64, &str)] = &[
    ("NAVTEX", 518.0, "FSK"),
    ("NAVTEX (national)", 490.0, "FSK"),
    ("NAVTEX HF", 4209.5, "FSK"),
    ("Shannon VOLMET", 3413.0, "USB"),
    ("Shannon VOLMET", 5505.0, "USB"),
    ("Shannon VOLMET", 8957.0, "USB"),
    ("Shannon VOLMET", 13264.0, "USB"),
    ("RAF VOLMET", 5450.0, "USB"),
    ("RAF VOLMET", 11253.0, "USB"),
];

/// Label candidates beyond the broadcast schedule, for LABELS BY S/N: the
/// weather-fax stations (`sdroxide_types::WEFAX_STATIONS`), the five NCDXF/IARU beacon
/// frequencies (`sdroxide_types::IBP_BANDS`) and [`UTILITY_KHZ`]. As broadcast-kind
/// spots, so they are drawn and listed like the schedule's own; the S/N guide
/// decides which of them get a label.
// Why: HDZ 2026-11-07: "dann können auch mehr Kandidaten aus der Datenbank
//      befragt werden … oder auch Sender aus anderen Quellen, was passt, das
//      passt".
pub fn extra_label_candidates(now_utc: i64) -> Vec<sdroxide_types::Spot> {
    use sdroxide_types::{Spot, SpotKind};
    let spot =
        |call: String, freq_hz: f64, mode: &str, comment: &str, loc: Option<(f64, f64)>| Spot {
            id: Spot::make_id(SpotKind::Broadcast, &call, freq_hz),
            kind: SpotKind::Broadcast,
            freq_hz,
            call,
            spotter: String::new(),
            mode: mode.to_string(),
            comment: comment.to_string(),
            reference: None,
            grid: None,
            loc,
            when_utc: now_utc,
            snr_db: None,
        };
    let mut out = Vec::new();
    for st in sdroxide_types::WEFAX_STATIONS {
        for &khz in st.carriers_khz {
            out.push(spot(
                st.name.to_string(),
                khz * 1000.0,
                "FAX",
                "WEFAX",
                Some((st.lat, st.lon)),
            ));
        }
    }
    for b in sdroxide_types::IBP_BANDS {
        out.push(spot(format!("IBP {}", b.label), b.freq_hz, "CW", "NCDXF/IARU beacons", None));
    }
    // The digital modes' published dials: the signals sit in the audio
    // passband above the dial, so the label goes 1.5 kHz up, the middle of
    // FT8's and WSPR's windows.
    // Why: HDZ 2026-11-07: "bekommen wir die digitalen Quellen auch noch ran?"
    let digital: [(&str, &[f64]); 5] = [
        ("FT8", sdroxide_types::FT8_DIALS),
        ("FT4", sdroxide_types::FT4_DIALS),
        ("JS8", sdroxide_types::JS8_DIALS),
        ("WSPR", sdroxide_types::WSPR_DIALS),
        ("FSQ", sdroxide_types::FSQ_DIALS),
    ];
    for (mode, dials) in digital {
        for &dial in dials {
            out.push(spot(mode.to_string(), dial + 1_500.0, mode, "", None));
        }
    }
    for &(dial, _, _) in sdroxide_types::PSK_DIALS {
        out.push(spot("PSK31".to_string(), dial + 1_000.0, "PSK", "", None));
    }
    for &(name, khz, mode) in UTILITY_KHZ {
        out.push(spot(name.to_string(), khz * 1000.0, mode, "", None));
    }
    out
}

#[cfg(test)]
mod tests {
    /// A narrow window whose edges fall away under the front end's filter (a
    /// KiwiSDR's 12 kHz I/Q): the floor is the noise in the middle, not the
    /// skirts, so the noise reads about 0 dB and only the carrier stands out.
    #[test]
    fn the_filter_skirts_are_not_the_floor() {
        let mut p = vec![1.0f64; 120];
        for i in 0..10 {
            p[i] = 0.01 * (i + 1) as f64;
            p[119 - i] = 0.01 * (i + 1) as f64;
        }
        p[60] = 16.0;
        let floor = super::floor_of(&p).expect("a floor");
        assert!((floor - 1.0).abs() < 1e-9, "floor {floor}");
    }

    /// The extra label candidates carry the services they are meant to:
    /// DWD Pinneberg's 7880 kHz fax, the 20 m beacon, Shannon VOLMET.
    #[test]
    fn extra_candidates_name_fax_beacons_and_volmet() {
        let c = super::extra_label_candidates(0);
        let has = |name: &str, hz: f64| {
            c.iter().any(|s| s.call.contains(name) && (s.freq_hz - hz).abs() < 1.0)
        };
        assert!(has("Pinneberg", 7_880_000.0));
        assert!(has("IBP 20m", 14_100_000.0));
        assert!(has("Shannon VOLMET", 5_505_000.0));
        assert!(has("FT8", 14_075_500.0));
        assert!(has("WSPR", 10_138_700.0 + 1_500.0));
    }

    use super::*;

    /// A frame of 200 bins over 2 kHz around `center`: noise that changes
    /// from frame to frame (a fixed pseudo-random walk), and a carrier at
    /// `carrier_hz` only 3 dB over the noise's mean.
    fn frame(r: usize, center: f64, carrier_hz: f64) -> SpectrumFrame {
        let (span, n) = (2_000.0, 200);
        let edge = center - span / 2.0;
        let bin = span / n as f64;
        let bins = (0..n)
            .map(|i| {
                let hz = edge + (i as f64 + 0.5) * bin;
                // Noise: −100 dB ± 6 dB, different in every bin and frame.
                let h =
                    ((i as u64 * 2_654_435_761) ^ (r as u64 * 40_503)).wrapping_mul(2_246_822_519);
                let noise = -100.0 + ((h >> 16) % 13) as f32 - 6.0;
                let db = if (hz - carrier_hz).abs() < 6.0 { noise + 3.0 } else { noise };
                ((db + 120.0) / 100.0 * 255.0).round().clamp(0.0, 255.0) as u8
            })
            .collect();
        SpectrumFrame {
            seq: r as u32,
            center_hz: center,
            span_hz: span,
            db_floor: -120.0,
            db_ceil: -20.0,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    /// Proves: a carrier 3 dB over noise that swings ±6 dB frame to frame
    /// stands out once enough frames are in the mean — noise flattens near
    /// 0 dB S/N, the carrier keeps about its 3 dB.
    #[test]
    fn a_faint_carrier_stands_out_of_the_accumulated_noise() {
        let mut g = MeanGrid::new(&frame(0, 1_000_000.0, 1_000_305.0)).expect("grid");
        for r in 0..400 {
            g.push(&frame(r, 1_000_000.0, 1_000_305.0), r as f64 * 0.05);
        }
        let s = g.guide().expect("guide");
        let at = |hz: f64| s.snr_db[((hz - s.lo_hz) / s.cell_hz) as usize];
        let noise_max = (0..s.snr_db.len())
            .map(|i| s.lo_hz + (i as f64 + 0.5) * s.cell_hz)
            .filter(|hz| (hz - 1_000_305.0).abs() > 20.0)
            .map(at)
            .fold(f32::MIN, f32::max);
        assert_eq!(s.frames, 400);
        assert!(at(1_000_305.0) > 2.0, "carrier {}", at(1_000_305.0));
        assert!(noise_max < at(1_000_305.0) - 1.0, "noise up to {noise_max}");
    }

    /// Proves: frames that move with the dial (maximum zoom out, where the
    /// panadapter follows the tuning) get a grid reaching the new edge, and
    /// the cells seen before the move keep their frames.
    #[test]
    fn a_panadapter_carried_by_the_dial_keeps_its_mean_and_its_new_edge() {
        let mut g = MeanGrid::new(&frame(0, 1_000_000.0, 1_000_305.0)).expect("grid");
        for r in 0..40 {
            let f = if r < 20 {
                frame(r, 1_000_000.0, 1_000_305.0)
            } else {
                frame(r, 1_000_500.0, 1_000_305.0)
            };
            if !g.fits(&f) {
                g = g.relaid(&f).expect("grid");
            }
            g.push(&f, r as f64 * 0.05);
        }
        let s = g.guide().expect("guide");
        let hi = s.lo_hz + s.snr_db.len() as f64 * s.cell_hz;
        assert!(hi >= 1_001_499.0, "grid reaches the new edge, ends at {hi}");
        assert_eq!(s.frames, 40, "the move kept the frames before it");
        assert!(s.snr_db.last().is_some_and(|v| v.is_finite()), "new edge drawn");
    }

    /// Proves: after the dial carried the panadapter along, the cells that
    /// came into view count only the frames since, the others all of them —
    /// so the caption can say how much of the picture is young.
    #[test]
    fn cells_new_after_a_move_count_only_their_own_frames() {
        let mut g = MeanGrid::new(&frame(0, 1_000_000.0, 1_000_305.0)).expect("grid");
        for r in 0..40 {
            let f = if r < 30 {
                frame(r, 1_000_000.0, 1_000_305.0)
            } else {
                frame(r, 1_000_500.0, 1_000_305.0)
            };
            if !g.fits(&f) {
                g = g.relaid(&f).expect("grid");
            }
            g.push(&f, r as f64 * 0.05);
        }
        let s = g.guide().expect("guide");
        let count_at = |hz: f64| s.count[((hz - s.lo_hz) / s.cell_hz) as usize];
        assert_eq!(count_at(1_000_305.0), 40, "seen all along");
        assert_eq!(count_at(1_001_305.0), 10, "in view since the move");
    }

    /// Proves: on a full band, stations on 60 % of the frequencies, the noise
    /// floor stays at the noise between them (S/N 0 dB there, +10 dB on a
    /// station) — the median would have put it on the stations.
    #[test]
    fn a_full_band_keeps_its_floor_in_the_gaps() {
        let full = |r: usize| {
            let mut f = frame(r, 1_000_000.0, 0.0);
            for (i, b) in f.bins.iter_mut().enumerate() {
                // −100 dB noise, stations at −90 dB in 6 of every 10 bins.
                let db = if i % 10 < 6 { -90.0 } else { -100.0 };
                *b = ((db + 120.0) / 100.0 * 255.0_f32).round() as u8;
            }
            f
        };
        let mut g = MeanGrid::new(&full(0)).expect("grid");
        for r in 0..20 {
            g.push(&full(r), r as f64 * 0.05);
        }
        let s = g.guide().expect("guide");
        let gap = s.snr_db[8];
        let station = s.snr_db[2];
        assert!(gap.abs() < 0.5, "noise between stations at {gap} dB");
        assert!((station - 10.0).abs() < 0.5, "station at {station} dB");
    }

    /// Proves: a carrier that goes off air fades out of the guide — within
    /// half a minute (five fall constants) it is under 1 dB, where a plain mean
    /// of all frames would still show most of it.
    #[test]
    fn a_carrier_gone_off_air_fades_out() {
        let mut g = MeanGrid::new(&frame(0, 1_000_000.0, 1_000_305.0)).expect("grid");
        let step = 0.05;
        let on = (MEAN_TAU_S / step) as usize;
        for r in 0..on {
            g.push(&frame(r, 1_000_000.0, 1_000_305.0), r as f64 * step);
        }
        let off = (5.0 * MEAN_FALL_TAU_S / step) as usize;
        for r in on..on + off {
            // The carrier off: placed outside the frame.
            g.push(&frame(r, 1_000_000.0, 0.0), r as f64 * step);
        }
        let s = g.guide().expect("guide");
        let at = s.snr_db[((1_000_305.0 - s.lo_hz) / s.cell_hz) as usize];
        assert!(at < 1.0, "carrier still at {at} dB");
    }

    /// Proves: a zoom (frames of another bin width) starts the mean afresh,
    /// so levels from the old width — another share of the noise in each
    /// bin — leave no step in the guide.
    #[test]
    fn a_zoom_starts_afresh_and_leaves_no_step() {
        // Fine frames over 2 kHz, then coarse ones over 4 kHz whose noise is
        // 3 dB higher per bin (twice the width).
        let coarse = |r: usize| {
            let mut f = frame(r, 1_000_000.0, 0.0);
            f.span_hz = 4_000.0;
            for b in f.bins.iter_mut() {
                *b = b.saturating_add(8); // +3.1 dB at 100 dB over 255 steps
            }
            f
        };
        let mut g = MeanGrid::new(&frame(0, 1_000_000.0, 0.0)).expect("grid");
        for r in 0..200 {
            g.push(&frame(r, 1_000_000.0, 0.0), r as f64 * 0.05);
        }
        for r in 200..400 {
            let f = coarse(r);
            if !g.fits(&f) {
                g = g.relaid(&f).expect("grid");
            }
            g.push(&f, r as f64 * 0.05);
        }
        let s = g.guide().expect("guide");
        assert_eq!(s.frames, 200, "counted from the zoom on");
        let spread = s.snr_db.iter().fold(f32::MIN, |m, v| m.max(*v));
        assert!(spread < 2.0, "noise only, no step: highest {spread} dB");
    }

    /// Proves: the S/N under the pointer is the value of the cell the
    /// frequency falls in, and nothing off the grid.
    #[test]
    fn the_value_under_the_pointer_is_its_cells() {
        let g = Guide {
            lo_hz: 1_000.0,
            cell_hz: 10.0,
            snr_db: vec![0.0, 7.5, f32::NAN],
            count: vec![1, 1, 0],
            frames: 1,
        };
        assert_eq!(g.snr_at(1_015.0), Some(7.5));
        assert_eq!(g.snr_at(1_025.0), None, "nothing seen there");
        assert_eq!(g.snr_at(995.0), None, "below the grid");
        assert_eq!(g.snr_at(1_030.0), None, "above the grid");
    }
}
