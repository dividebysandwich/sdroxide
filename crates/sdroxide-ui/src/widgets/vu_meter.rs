//! A stereo VU meter: two columns of segments, left and right, that move with
//! the programme. Drawn for the DAB panel, where the audio is decoded on the
//! station and only its level reaches the screen.
//!
//! Two halves, kept apart so the motion can be tested without drawing:
//! [`VuBallistics`] turns the level the engine reports into what the needle —
//! here the lit column — should show, and [`stereo`] paints it in the theme's
//! own colours.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::theme;

/// The scale, top to bottom, in VU (dB relative to the alignment level).
/// One segment each; the two at the top are the red overload zone.
pub const SCALE_VU: [i32; 16] = [8, 6, 4, 2, 0, -2, -4, -6, -8, -12, -16, -20, -28, -40, -48, -60];

/// The red zone starts here: a segment at or above it is drawn in the alert
/// colour.
pub const RED_FROM_VU: i32 = 6;

/// 0 VU sits at −12 dBFS RMS. The EBU alignment level is −18, but DAB+
/// stations are compressed loud: on the air (8D, Nancy) programme measured
/// around −12 dBFS RMS and sat at +4…+6 against −18, so the column lived in
/// the red. At −12 a station at full volume moves round 0 VU and only loud
/// passages reach the red.
pub const ALIGN_DBFS: f32 = -12.0;

/// One volume step for the wheel and the arrow keys.
pub const VOLUME_STEP: f32 = 0.05;

/// Below the bottom of the scale: the column is dark.
const FLOOR_VU: f32 = -70.0;

/// The ballistics of a hi-fi LED meter: the column jumps up with each beat
/// (99 % of a rise in [`ATTACK_S`]) and falls back more slowly (99 % of a fall
/// in [`RELEASE_S`]), so it pumps with the programme instead of drifting with
/// its average. The peak marker keeps its slower hold above it.
const ATTACK_S: f32 = 0.03;
const RELEASE_S: f32 = 0.35;

/// The swing. A DAB+ station is compressed so hard that its level moves only
/// three or four dB with the music, and read straight the column sat between
/// 0 and +4 VU and barely moved. A hi-fi meter is there to dance, so the
/// meter reads each moment *against the programme's own running level*: the
/// running level sits at [`SWING_CENTRE_VU`], and every dB above or below it
/// moves the column [`SWING_GAIN`] dB. Left and right share the running level,
/// so a louder ear still reads louder. The running level follows over about
/// [`SWING_MEAN_S`].
const SWING_CENTRE_VU: f32 = -6.0;
const SWING_GAIN: f32 = 3.0;
const SWING_MEAN_S: f32 = 3.0;

/// How long the peak marker holds before it starts to fall, and how fast it
/// falls then.
const PEAK_HOLD_S: f32 = 1.5;
const PEAK_FALL_VU_PER_S: f32 = 20.0;

/// The meter's motion, left and right: the integrated level and the held peak,
/// both in VU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VuBallistics {
    pub level: [f32; 2],
    pub peak: [f32; 2],
    peak_age: [f32; 2],
    /// The programme's running level, in VU, both ears together; `None`
    /// until something plays. See [`SWING_GAIN`].
    mean: Option<f32>,
}

impl Default for VuBallistics {
    fn default() -> Self {
        VuBallistics { level: [FLOOR_VU; 2], peak: [FLOOR_VU; 2], peak_age: [0.0; 2], mean: None }
    }
}

impl VuBallistics {
    /// The dBFS RMS the engine reports, as VU.
    pub fn vu_of_dbfs(dbfs: f32) -> f32 {
        if dbfs.is_finite() { (dbfs - ALIGN_DBFS).max(FLOOR_VU) } else { FLOOR_VU }
    }

    /// Advance by `dt` seconds towards `input` (dBFS RMS, left and right).
    /// `None` means nothing is playing: the column falls away to dark.
    pub fn step(&mut self, input: Option<[f32; 2]>, dt: f32) {
        let dt = dt.clamp(0.0, 0.5);
        // A first-order integrator whose 99 % time is the attack going up and
        // the release coming down: ln(100) ≈ 4.6.
        let k = |time: f32| 1.0 - (-dt * 4.6 / time).exp();
        let raw = input.map(|l| [Self::vu_of_dbfs(l[0]), Self::vu_of_dbfs(l[1])]);
        match raw {
            Some([l, r]) => {
                let now = (l + r) / 2.0;
                let mean = self.mean.get_or_insert(now);
                *mean += (now - *mean) * k(SWING_MEAN_S);
            }
            None => self.mean = None,
        }
        for ch in 0..2 {
            let target = match (raw, self.mean) {
                (Some(raw), Some(mean)) => (SWING_CENTRE_VU + SWING_GAIN * (raw[ch] - mean))
                    .clamp(FLOOR_VU, SCALE_VU[0] as f32 + 2.0),
                _ => FLOOR_VU,
            };
            let time = if target > self.level[ch] { ATTACK_S } else { RELEASE_S };
            self.level[ch] += (target - self.level[ch]) * k(time);
            if self.level[ch] >= self.peak[ch] {
                self.peak[ch] = self.level[ch];
                self.peak_age[ch] = 0.0;
            } else {
                self.peak_age[ch] += dt;
                if self.peak_age[ch] > PEAK_HOLD_S {
                    self.peak[ch] = (self.peak[ch] - PEAK_FALL_VU_PER_S * dt).max(self.level[ch]);
                }
            }
        }
    }

    /// Whether anything is still moving, so the caller knows to keep asking
    /// for frames.
    pub fn is_live(&self) -> bool {
        self.level.iter().chain(self.peak.iter()).any(|v| *v > FLOOR_VU + 0.5)
    }
}

/// How many segments, counted from the bottom, a level lights.
pub fn lit_segments(vu: f32) -> usize {
    SCALE_VU.iter().filter(|s| vu >= **s as f32).count()
}

/// The segment the peak marker sits on, counted from the bottom (1-based), or
/// 0 when the peak is below the scale.
fn peak_segment(vu: f32) -> usize {
    lit_segments(vu)
}

/// The column width the meter asks for: two bars, the scale between and beside
/// them, and a frame.
pub const WIDTH: f32 = 112.0;

/// The narrow form, for a panel too tight for [`WIDTH`]: the scale on the left
/// only, and no plate.
pub const WIDTH_COMPACT: f32 = 64.0;

/// The engraving on the plate under the bars.
pub const PLATE: &str = "Oxide Brown";

/// What a drawn meter answers: its response — dragged, clicked, hovered —
/// and where its bars are, which is the travel of the volume it sets.
pub struct VuResponse {
    pub response: egui::Response,
    pub body: Rect,
    pub horizontal: bool,
}

impl VuResponse {
    /// The volume a pointer at `pos` asks for: the bottom of the column (the
    /// left of the bar) is none, the top (the right) is full.
    pub fn volume_at(&self, pos: Pos2) -> f32 {
        let f = if self.horizontal {
            (pos.x - self.body.left()) / self.body.width().max(1.0)
        } else {
            (self.body.bottom() - pos.y) / self.body.height().max(1.0)
        };
        f.clamp(0.0, 1.0)
    }
}

/// The volume mark: a thin yellow line across the bars at the volume, with a
/// small notch at its start, so the meter shows the knob it is.
fn paint_volume_mark(p: &egui::Painter, body: Rect, volume: f32, horizontal: bool) {
    let ink = theme::YELLOW();
    let v = volume.clamp(0.0, 1.0);
    if horizontal {
        let x = body.left() + body.width() * v;
        p.line_segment(
            [Pos2::new(x, body.top() - 2.0), Pos2::new(x, body.bottom() + 2.0)],
            Stroke::new(1.5, ink.gamma_multiply(0.8)),
        );
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(x - 4.0, body.top() - 6.0),
                Pos2::new(x + 4.0, body.top() - 6.0),
                Pos2::new(x, body.top() - 1.0),
            ],
            ink,
            Stroke::NONE,
        ));
    } else {
        let y = body.bottom() - body.height() * v;
        p.line_segment(
            [Pos2::new(body.left() - 2.0, y), Pos2::new(body.right() + 2.0, y)],
            Stroke::new(1.5, ink.gamma_multiply(0.8)),
        );
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(body.right() + 1.0, y),
                Pos2::new(body.right() + 6.0, y - 4.0),
                Pos2::new(body.right() + 6.0, y + 4.0),
            ],
            ink,
            Stroke::NONE,
        ));
    }
}

/// The sense a meter takes: a volume control when it has one to set.
fn meter_sense(volume: Option<f32>) -> Sense {
    if volume.is_some() { Sense::click_and_drag() } else { Sense::hover() }
}

/// Paint a stereo meter into a box `size` big, in the theme's colours.
/// `compact` is the narrow form ([`WIDTH_COMPACT`]): one scale, no plate.
/// With a `volume`, the meter is the volume control too: its mark is drawn and
/// it takes a drag, a click, the wheel and the arrow keys.
pub fn stereo(
    ui: &mut egui::Ui,
    m: &VuBallistics,
    size: Vec2,
    compact: bool,
    volume: Option<f32>,
) -> VuResponse {
    let (rect, resp) = ui.allocate_exact_size(size, meter_sense(volume));
    let case = rect.shrink(1.0);
    let title_h = 18.0;
    let plate_h = if compact { 0.0 } else { 20.0 };
    let pad = 6.0;
    let body = Rect::from_min_max(
        Pos2::new(case.left() + pad, case.top() + pad + title_h),
        Pos2::new(
            case.right() - pad,
            case.bottom() - pad - plate_h - if compact { 12.0 } else { 18.0 },
        ),
    );
    let out = VuResponse { response: resp, body, horizontal: false };
    if !ui.is_rect_visible(rect) {
        return out;
    }
    let p = ui.painter_at(rect);

    // The case: the theme's deep ground, framed like every other box.
    p.rect_filled(case, 6.0, theme::BG_DEEP());
    p.rect_stroke(case, 6.0, Stroke::new(1.0, theme::LINE_LIT()), egui::StrokeKind::Inside);

    p.text(
        Pos2::new(case.center().x, case.top() + pad + title_h / 2.0 - 2.0),
        Align2::CENTER_CENTER,
        "VU",
        FontId::proportional(13.0),
        theme::TEXT_STRONG(),
    );

    // The bars and their scale.
    let n = SCALE_VU.len();
    let pitch = body.height() / n as f32;
    let seg_h = (pitch * 0.72).max(2.0);
    let label_w = 22.0;
    let gap = 4.0;
    let labels = if compact { 1.0 } else { 2.0 };
    let bar_w = ((body.width() - labels * label_w - gap) / 2.0).max(6.0);
    let left_x = body.left() + label_w;
    let right_x = left_x + bar_w + gap;

    let lit = theme::CYAN();
    let red = theme::ALERT();
    let unlit = |c: Color32| c.gamma_multiply(0.14);
    let label_ink = theme::gray(150);

    let lit_n = [lit_segments(m.level[0]), lit_segments(m.level[1])];
    let peak_n = [peak_segment(m.peak[0]), peak_segment(m.peak[1])];

    for (i, vu) in SCALE_VU.iter().enumerate() {
        // `i` counts from the top; a segment is lit when the level reaches it,
        // which counted from the bottom is `n - i`.
        let from_bottom = n - i;
        let y = body.top() + pitch * i as f32 + (pitch - seg_h) / 2.0;
        let base = if *vu >= RED_FROM_VU { red } else { lit };
        for (ch, x) in [(0usize, left_x), (1usize, right_x)] {
            let seg = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bar_w, seg_h));
            let on = from_bottom <= lit_n[ch];
            let is_peak = from_bottom == peak_n[ch] && peak_n[ch] > 0;
            if on || is_peak {
                // A soft glow behind the lit segment, then the segment itself.
                p.rect_filled(seg.expand(2.0), 3.0, base.gamma_multiply(0.25));
                p.rect_filled(seg, 2.0, base);
                p.rect_filled(
                    Rect::from_min_size(seg.min, Vec2::new(seg.width(), seg.height() * 0.35)),
                    2.0,
                    Color32::from_white_alpha(40),
                );
            } else {
                p.rect_filled(seg, 2.0, unlit(base));
            }
        }
        let text = if *vu > 0 { format!("+{vu}") } else { vu.to_string() };
        let cy = y + seg_h / 2.0;
        let font = FontId::monospace(9.0);
        p.text(Pos2::new(left_x - 3.0, cy), Align2::RIGHT_CENTER, &text, font.clone(), label_ink);
        if !compact {
            p.text(
                Pos2::new(right_x + bar_w + 3.0, cy),
                Align2::LEFT_CENTER,
                &text,
                font,
                label_ink,
            );
        }
    }
    // L and R under their columns.
    for (x, ch) in [(left_x, "L"), (right_x, "R")] {
        p.text(
            Pos2::new(x + bar_w / 2.0, body.bottom() + 3.0),
            Align2::CENTER_TOP,
            ch,
            FontId::proportional(9.0),
            label_ink,
        );
    }

    if let Some(v) = volume {
        paint_volume_mark(
            &p,
            Rect::from_x_y_ranges(left_x..=right_x + bar_w, body.y_range()),
            v,
            false,
        );
    }
    if compact {
        return out;
    }
    // The plate: a brushed-grey strip with the name engraved on it.
    let plate = Rect::from_min_max(
        Pos2::new(case.left() + pad + 8.0, case.bottom() - pad - plate_h),
        Pos2::new(case.right() - pad - 8.0, case.bottom() - pad),
    );
    p.rect_filled(plate, 3.0, theme::gray(175));
    p.rect_stroke(plate, 3.0, Stroke::new(1.0, theme::gray(110)), egui::StrokeKind::Inside);
    for (x, y) in [(plate.left() + 4.0, plate.center().y), (plate.right() - 4.0, plate.center().y)]
    {
        p.circle_filled(Pos2::new(x, y), 1.6, theme::gray(110));
    }
    let engrave = FontId::proportional(10.5);
    p.text(
        plate.center() + Vec2::new(0.0, 0.6),
        Align2::CENTER_CENTER,
        PLATE,
        engrave.clone(),
        Color32::from_white_alpha(90),
    );
    p.text(plate.center(), Align2::CENTER_CENTER, PLATE, engrave, theme::gray(55));

    out
}

/// The height a horizontal meter wants: two bars and the scale under them.
pub const HEIGHT_HORIZONTAL: f32 = 46.0;

/// The same stereo meter laid on its side, for a phone: L over R, quiet on the
/// left and loud on the right, the scale underneath. A column would be a
/// finger wide there; a bar across the screen is the width of the screen.
pub fn stereo_horizontal(
    ui: &mut egui::Ui,
    m: &VuBallistics,
    size: Vec2,
    volume: Option<f32>,
) -> VuResponse {
    let (rect, resp) = ui.allocate_exact_size(size, meter_sense(volume));
    let case = rect.shrink(1.0);
    let body = Rect::from_min_max(
        Pos2::new(case.left() + 6.0 + 26.0 + 10.0, case.top() + 6.0),
        Pos2::new(case.right() - 6.0, case.bottom() - 6.0 - 11.0),
    );
    let out = VuResponse { response: resp, body, horizontal: true };
    if !ui.is_rect_visible(rect) {
        return out;
    }
    let p = ui.painter_at(rect);
    p.rect_filled(case, 6.0, theme::BG_DEEP());
    p.rect_stroke(case, 6.0, Stroke::new(1.0, theme::LINE_LIT()), egui::StrokeKind::Inside);

    let pad = 6.0;
    let title_w = 26.0;
    let label_ink = theme::gray(150);
    p.text(
        Pos2::new(case.left() + pad + title_w / 2.0, case.center().y),
        Align2::CENTER_CENTER,
        "VU",
        FontId::proportional(12.0),
        theme::TEXT_STRONG(),
    );
    let ch_w = 10.0;
    let n = SCALE_VU.len();
    let pitch = body.width() / n as f32;
    let seg_w = (pitch * 0.78).max(2.0);
    let gap = 3.0;
    let bar_h = ((body.height() - gap) / 2.0).max(4.0);

    let lit = theme::CYAN();
    let red = theme::ALERT();
    let lit_n = [lit_segments(m.level[0]), lit_segments(m.level[1])];
    let peak_n = [peak_segment(m.peak[0]), peak_segment(m.peak[1])];

    for (ch, label) in [(0usize, "L"), (1usize, "R")] {
        let y = body.top() + ch as f32 * (bar_h + gap);
        p.text(
            Pos2::new(body.left() - ch_w / 2.0 - 1.0, y + bar_h / 2.0),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(9.0),
            label_ink,
        );
        // Counted from the quiet end, which is the bottom of the column and
        // the left of the bar.
        for k in 0..n {
            let vu = SCALE_VU[n - 1 - k];
            let from_bottom = k + 1;
            let x = body.left() + pitch * k as f32;
            let seg = Rect::from_min_size(Pos2::new(x, y), Vec2::new(seg_w, bar_h));
            let base = if vu >= RED_FROM_VU { red } else { lit };
            let on = from_bottom <= lit_n[ch];
            let is_peak = from_bottom == peak_n[ch] && peak_n[ch] > 0;
            if on || is_peak {
                p.rect_filled(seg.expand(1.5), 3.0, base.gamma_multiply(0.25));
                p.rect_filled(seg, 2.0, base);
            } else {
                p.rect_filled(seg, 2.0, base.gamma_multiply(0.14));
            }
        }
    }
    // A thinned scale: every label would run into its neighbours.
    for k in 0..n {
        let vu = SCALE_VU[n - 1 - k];
        if ![-60, -40, -20, -8, 0, 4, 8].contains(&vu) {
            continue;
        }
        let text = if vu > 0 { format!("+{vu}") } else { vu.to_string() };
        p.text(
            Pos2::new(body.left() + pitch * k as f32 + seg_w / 2.0, body.bottom() + 1.0),
            Align2::CENTER_TOP,
            text,
            FontId::monospace(8.5),
            label_ink,
        );
    }
    if let Some(v) = volume {
        paint_volume_mark(&p, body, v, true);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A loud DAB+ station at full volume moves round 0 VU, not in the red.
    #[test]
    fn a_dab_station_at_full_volume_is_not_pinned_in_the_red() {
        let vu = VuBallistics::vu_of_dbfs(-12.0);
        assert!(vu < RED_FROM_VU as f32, "{vu} VU is in the red");
        assert!(vu > -4.0, "{vu} VU is too low for a loud station");
    }

    #[test]
    fn the_alignment_level_reads_zero_vu() {
        assert_eq!(VuBallistics::vu_of_dbfs(ALIGN_DBFS), 0.0);
        assert_eq!(VuBallistics::vu_of_dbfs(f32::NEG_INFINITY), FLOOR_VU);
    }

    #[test]
    fn a_level_lights_the_segments_up_to_it_and_no_further() {
        assert_eq!(lit_segments(FLOOR_VU), 0, "silence is dark");
        assert_eq!(lit_segments(-60.0), 1, "the bottom segment");
        assert_eq!(lit_segments(0.0), 12, "0 VU: everything from -60 to 0");
        assert_eq!(lit_segments(8.0), SCALE_VU.len(), "the top segment is +8");
        assert_eq!(lit_segments(5.9), 11 + 3, "+6 is red and not reached at +5.9");
    }

    /// Like a hi-fi LED meter: a beat lifts the column within a couple of
    /// frames, and it falls back over a few hundred milliseconds.
    #[test]
    fn the_column_jumps_with_a_beat_and_falls_back_slowly() {
        let mut m = VuBallistics::default();
        let (quiet, beat) = (Some([ALIGN_DBFS - 4.0; 2]), Some([ALIGN_DBFS; 2]));
        for _ in 0..90 {
            m.step(quiet, 1.0 / 30.0);
        }
        let rest = m.level[0];
        m.step(beat, 1.0 / 30.0);
        assert!(m.level[0] > rest + 8.0, "one frame of a beat lifts it: {rest} -> {}", m.level[0]);
        let top = m.level[0];
        for _ in 0..3 {
            m.step(quiet, 1.0 / 30.0);
        }
        assert!(m.level[0] > rest && m.level[0] < top, "it falls back slowly: {}", m.level[0]);
        for _ in 0..12 {
            m.step(quiet, 1.0 / 30.0);
        }
        assert!((m.level[0] - rest).abs() < 2.0, "then it is back down: {}", m.level[0]);
    }

    /// A DAB+ station moves only three or four dB with the music; the meter
    /// has to swing across most of its scale with it, not sit between 0 and
    /// +4 (a report: "it hardly ever leaves 0 to 4").
    #[test]
    fn a_compressed_station_swings_across_the_scale() {
        let mut m = VuBallistics::default();
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for f in 0..240 {
            // A beat four times a second: 3.5 dB louder for its first 60 ms.
            let t = f as f32 / 60.0;
            let level = if (t % 0.25) < 0.06 { -8.5 } else { -12.0 };
            m.step(Some([level, level - 0.5]), 1.0 / 60.0);
            if f >= 120 {
                lo = lo.min(m.level[0]);
                hi = hi.max(m.level[0]);
            }
        }
        assert!(hi - lo >= 9.0, "the column swings only {:.1} VU ({lo:.1}..{hi:.1})", hi - lo);
        assert!(lit_segments(hi) - lit_segments(lo) >= 4, "it lights {lo:.1}..{hi:.1}");
        assert!(hi < RED_FROM_VU as f32 + 2.0, "and is not pinned at the top: {hi:.1}");
    }

    /// The peak marker stays where the loudest moment was for a while, then
    /// falls; and silence takes the whole meter dark.
    #[test]
    fn the_peak_holds_then_falls_and_silence_goes_dark() {
        let mut m = VuBallistics::default();
        for _ in 0..60 {
            m.step(Some([-10.0, -30.0]), 1.0 / 60.0);
        }
        let held = m.peak[0];
        for _ in 0..30 {
            m.step(Some([-40.0, -40.0]), 1.0 / 60.0);
        }
        assert_eq!(m.peak[0], held, "half a second in, the peak still holds");
        assert!(m.level[0] < held, "the level has fallen under it");
        for _ in 0..600 {
            m.step(None, 1.0 / 60.0);
        }
        assert!(!m.is_live(), "ten seconds of nothing is a dark meter: {m:?}");
        assert_eq!(lit_segments(m.level[0]), 0);
    }

    #[test]
    fn left_and_right_move_on_their_own() {
        let mut m = VuBallistics::default();
        for _ in 0..60 {
            m.step(Some([-10.0, -40.0]), 1.0 / 60.0);
        }
        assert!(lit_segments(m.level[0]) > lit_segments(m.level[1]));
    }
}
