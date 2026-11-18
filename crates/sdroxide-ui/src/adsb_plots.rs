//! The sighting log as a bar chart: the lines the log shows, counted by hour
//! of day, altitude, distance, airline or type — the axis picked from a list.
//!
//! One line of the log is one aircraft on one day, so every count here is of
//! aircraft-days. By hour, an aircraft counts in each hour it was heard in,
//! from its first frame to its last: that is how busy the sky was then. By
//! altitude it counts at its highest, by distance at its farthest — the
//! latter the diagnostic one: where the antenna stops hearing, the bars stop.
//! Airline and type show the commonest, the rest summed as one bar.
// Why: HDZ 2026-11-18: „rechts neben der Tabelle noch ein Diagramm … x =
//      Uhrzeit in Stundenbins, y count Flugbewegungen. Dann noch ein Diagramm
//      x = Höhe in Bins … x = Airline … x = Typ (mach das in einem Diagramm und
//      lass die x-Achse einfach per Popdown auswählen) … mach lieber im
//      Diagramm eine x-Achse Kilometer gegen counts als diagnostischer Plot“.

use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke};

use crate::adsb_log::Sighting;

/// What the bars are counted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Daytime,
    Height,
    Distance,
    Airline,
    Type,
}

impl Axis {
    pub const ALL: [Axis; 5] =
        [Axis::Daytime, Axis::Height, Axis::Distance, Axis::Airline, Axis::Type];

    pub fn from_u8(v: u8) -> Axis {
        Axis::ALL.get(usize::from(v)).copied().unwrap_or(Axis::Daytime)
    }

    pub fn to_u8(self) -> u8 {
        Axis::ALL.iter().position(|a| *a == self).unwrap_or(0) as u8
    }
}

/// Altitude bins, feet: 2,000 each, the last one open above.
pub const HEIGHT_STEP_FT: i32 = 2_000;
pub const HEIGHT_BINS: usize = 23;
/// Distance bins, km: the coverage diagram's rings.
pub const DISTANCE_STEP_KM: f32 = crate::adsb_coverage::RING_KM as f32;
pub const DISTANCE_BINS: usize = crate::adsb_coverage::RINGS;

/// The chart, ready to draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bars {
    pub labels: Vec<String>,
    pub counts: Vec<u32>,
    /// Lines that had nothing to count by: no altitude, no position, no
    /// airline code, no type.
    pub without: usize,
}

/// Count `rows` by `axis`. `offset` gives the local clock's offset from UTC
/// at a time, seconds; `top` is how many categories airline and type keep
/// before the rest becomes one bar.
pub fn bars(rows: &[&Sighting], axis: Axis, offset: impl Fn(i64) -> i64, top: usize) -> Bars {
    let mut b = Bars::default();
    match axis {
        Axis::Daytime => {
            b.counts = vec![0; 24];
            for s in rows {
                let (f, l) = (s.first + offset(s.first), s.last + offset(s.last));
                let (h0, h1) = (f.div_euclid(3600), l.div_euclid(3600).max(f.div_euclid(3600)));
                // A line is one day: at most 24 hours, each counted once.
                for h in h0..=h1.min(h0 + 23) {
                    b.counts[h.rem_euclid(24) as usize] += 1;
                }
            }
            b.labels = (0..24).map(|h| format!("{h:02}")).collect();
        }
        Axis::Height => {
            b.counts = vec![0; HEIGHT_BINS];
            for s in rows {
                match s.max_alt_ft {
                    Some(a) => {
                        let i = (a.max(0) / HEIGHT_STEP_FT) as usize;
                        b.counts[i.min(HEIGHT_BINS - 1)] += 1;
                    }
                    None => b.without += 1,
                }
            }
            // Flight levels: FL0, FL20 … FL440+.
            b.labels = (0..HEIGHT_BINS)
                .map(|i| {
                    let fl = i as i32 * HEIGHT_STEP_FT / 100;
                    if i == HEIGHT_BINS - 1 { format!("{fl}+") } else { format!("{fl}") }
                })
                .collect();
        }
        Axis::Distance => {
            b.counts = vec![0; DISTANCE_BINS];
            for s in rows {
                if s.max_km.is_nan() {
                    b.without += 1;
                    continue;
                }
                let i = (s.max_km.max(0.0) / DISTANCE_STEP_KM) as usize;
                b.counts[i.min(DISTANCE_BINS - 1)] += 1;
            }
            // Out to the farthest bin with anything, five at least: an empty
            // tail would squeeze the bars that say where reception ends.
            let last = b.counts.iter().rposition(|&n| n > 0).map_or(4, |i| i.max(4));
            b.counts.truncate(last + 1);
            b.labels = (0..b.counts.len())
                .map(|i| format!("{:.0}", i as f32 * DISTANCE_STEP_KM))
                .collect();
        }
        Axis::Airline | Axis::Type => {
            let mut n: std::collections::HashMap<String, u32> = Default::default();
            for s in rows {
                let key = if axis == Axis::Airline {
                    crate::adsb_log::airline_of(&s.call).map(str::to_string)
                } else {
                    // A guessed type counts as the type it guesses.
                    let k = s.kind.trim_end_matches('?').trim();
                    (!k.is_empty()).then(|| k.to_string())
                };
                match key {
                    Some(k) => *n.entry(k).or_default() += 1,
                    None => b.without += 1,
                }
            }
            let mut v: Vec<(String, u32)> = n.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let top = top.max(2);
            if v.len() > top {
                let rest: u32 = v[top - 1..].iter().map(|x| x.1).sum();
                v.truncate(top - 1);
                v.push(("…".to_string(), rest));
            }
            (b.labels, b.counts) = v.into_iter().unzip();
        }
    }
    b
}

/// A step for the count axis that gives about four grid lines: 1, 2 or 5
/// times a power of ten.
fn nice_step(max: u32) -> u32 {
    let raw = (max as f64 / 4.0).max(1.0);
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    let k = if m <= 1.0 {
        1.0
    } else if m <= 2.0 {
        2.0
    } else if m <= 5.0 {
        5.0
    } else {
        10.0
    };
    (k * p) as u32
}

/// Draw the bars into a `size` area. Hovering a bar names it and its count.
pub fn draw(ui: &mut egui::Ui, b: &Bars, size: egui::Vec2, bar: Color32, ink: Color32) {
    let (resp, painter) = ui.allocate_painter(size, Sense::hover());
    let r = resp.rect;
    let font = FontId::monospace(9.5);
    let (left, bottom, top) = (34.0, 18.0, 14.0);
    let plot = egui::Rect::from_min_max(
        egui::pos2(r.left() + left, r.top() + top),
        egui::pos2(r.right() - 6.0, r.bottom() - bottom),
    );
    let max = b.counts.iter().copied().max().unwrap_or(0).max(1);
    let step = nice_step(max);
    let ymax = max.div_ceil(step) * step;
    let y_of = |n: u32| plot.bottom() - plot.height() * n as f32 / ymax as f32;
    let grid = Stroke::new(1.0, ink.gamma_multiply(0.15));
    let mut g = 0;
    while g <= ymax {
        let y = y_of(g);
        painter.hline(plot.left()..=plot.right(), y, grid);
        painter.text(
            egui::pos2(plot.left() - 4.0, y),
            Align2::RIGHT_CENTER,
            g.to_string(),
            font.clone(),
            ink.gamma_multiply(0.7),
        );
        g += step;
    }
    let n = b.counts.len().max(1);
    let slot = plot.width() / n as f32;
    let gap = (slot * 0.2).clamp(1.0, 6.0);
    // Labels as many as fit: every k-th when they would overlap.
    let widest = b.labels.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32 * 6.0 + 4.0;
    let every = (widest / slot).ceil().max(1.0) as usize;
    let pointer = resp.hover_pos();
    for (i, &c) in b.counts.iter().enumerate() {
        let x0 = plot.left() + i as f32 * slot + gap / 2.0;
        let rect = egui::Rect::from_min_max(
            egui::pos2(x0, y_of(c)),
            egui::pos2(x0 + slot - gap, plot.bottom()),
        );
        let column = egui::Rect::from_x_y_ranges(rect.x_range(), plot.y_range());
        let hot = pointer.is_some_and(|p| column.contains(p));
        painter.rect_filled(rect, 1.0, if hot { bar } else { bar.gamma_multiply(0.7) });
        if c > 0 && slot >= 18.0 {
            painter.text(
                egui::pos2(rect.center().x, rect.top() - 1.0),
                Align2::CENTER_BOTTOM,
                c.to_string(),
                font.clone(),
                ink,
            );
        }
        if i % every == 0
            && let Some(l) = b.labels.get(i)
        {
            painter.text(
                egui::pos2(x0 + (slot - gap) / 2.0, plot.bottom() + 3.0),
                Align2::CENTER_TOP,
                l,
                font.clone(),
                ink.gamma_multiply(0.8),
            );
        }
        if hot {
            let label = b.labels.get(i).cloned().unwrap_or_default();
            resp.clone().on_hover_text(format!("{label}: {c}"));
        }
    }
    painter.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        Stroke::new(1.0, ink.gamma_multiply(0.5)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(call: &str, kind: &str, first: i64, last: i64, km: f32, alt: Option<i32>) -> Sighting {
        Sighting {
            icao: 1,
            call: call.into(),
            kind: kind.into(),
            route: String::new(),
            country: String::new(),
            first,
            last,
            min_km: km,
            max_km: km,
            max_alt_ft: alt,
        }
    }

    /// Heard 09:50 to 11:10 UTC, an hour ahead locally: hours 10, 11 and 12.
    #[test]
    fn an_aircraft_counts_in_every_hour_it_was_heard() {
        let day = 1_795_003_200 - 12 * 3600; // 2026-11-18 00:00 UTC
        let s = line("KLM1", "E295", day + 9 * 3600 + 50 * 60, day + 11 * 3600 + 600, 50.0, None);
        let b = bars(&[&s], Axis::Daytime, |_| 3600, 10);
        assert_eq!(b.counts.len(), 24);
        let hit: Vec<usize> = (0..24).filter(|&h| b.counts[h] > 0).collect();
        assert_eq!(hit, vec![10, 11, 12]);
        assert_eq!(b.labels[10], "10");
    }

    #[test]
    fn heights_go_by_two_thousand_feet_and_the_top_is_open() {
        let rows = [
            line("A", "", 0, 0, 1.0, Some(37_000)),
            line("B", "", 0, 0, 1.0, Some(37_999)),
            line("C", "", 0, 0, 1.0, Some(51_000)),
            line("D", "", 0, 0, 1.0, None),
        ];
        let r: Vec<&Sighting> = rows.iter().collect();
        let b = bars(&r, Axis::Height, |_| 0, 10);
        assert_eq!(b.counts[18], 2, "FL360–380");
        assert_eq!(b.labels[18], "360");
        assert_eq!(b.counts[HEIGHT_BINS - 1], 1);
        assert_eq!(b.labels[HEIGHT_BINS - 1], "440+");
        assert_eq!(b.without, 1);
    }

    /// The distance bars end where reception ends; a line without a
    /// position is counted apart.
    #[test]
    fn distance_bars_end_at_the_farthest() {
        let rows = [
            line("A", "", 0, 0, 12.0, None),
            line("B", "", 0, 0, 148.0, None),
            line("C", "", 0, 0, f32::NAN, None),
        ];
        let r: Vec<&Sighting> = rows.iter().collect();
        let b = bars(&r, Axis::Distance, |_| 0, 10);
        assert_eq!(b.counts.len(), 15, "0 … 140 km");
        assert_eq!((b.counts[1], b.counts[14]), (1, 1));
        assert_eq!(b.labels[14], "140");
        assert_eq!(b.without, 1);
    }

    /// The commonest airlines and types, the rest as one bar; guessed types
    /// count as their type, private callsigns as without.
    #[test]
    fn categories_keep_the_commonest_and_sum_the_rest() {
        let rows = [
            line("KLM1", "E295", 0, 0, 1.0, None),
            line("KLM2", "E295?", 0, 0, 1.0, None),
            line("KLM3", "B738", 0, 0, 1.0, None),
            line("DLH4", "A320", 0, 0, 1.0, None),
            line("EWG5", "A320", 0, 0, 1.0, None),
            line("RYR6", "B738", 0, 0, 1.0, None),
            line("DIGRK", "", 0, 0, 1.0, None),
        ];
        let r: Vec<&Sighting> = rows.iter().collect();
        let a = bars(&r, Axis::Airline, |_| 0, 3);
        assert_eq!(a.labels, vec!["KLM", "DLH", "…"]);
        assert_eq!(a.counts, vec![3, 1, 2]);
        assert_eq!(a.without, 1);
        let t = bars(&r, Axis::Type, |_| 0, 10);
        assert_eq!(t.labels, vec!["A320", "B738", "E295"]);
        assert_eq!(t.counts, vec![2, 2, 2]);
        assert_eq!(t.without, 1);
    }

    #[test]
    fn grid_steps_are_round() {
        assert_eq!(nice_step(1), 1);
        assert_eq!(nice_step(7), 2);
        assert_eq!(nice_step(19), 5);
        assert_eq!(nice_step(130), 50);
        for a in Axis::ALL {
            assert_eq!(Axis::from_u8(a.to_u8()), a);
        }
    }
}
