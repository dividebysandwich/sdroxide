//! The QO-100 page of the SAT window: calibrate the station's frequency chain
//! against Es'hail-2's narrowband beacon.
//!
//! A page rather than a window of its own because QO-100 is a satellite. It is
//! the one this program can work without Doppler correction, being
//! geostationary — so what it needs instead is its LNB measured, which is all
//! of this. The window is [`crate::app::sat`]'s; only the body is here.
//!
//! The beacon this tracks — [`QO100_BEACON_HZ`] — is not a plain carrier: it
//! is a 400 baud differential+Manchester BPSK telemetry signal (AO-40
//! "uncoded" framing), which is why a magnitude peak search has no purpose
//! here — Manchester encoding leaves a *null* at the carrier frequency, not a
//! peak. The actual DSP lives engine-side, in `sdroxide_qo100` (raw IQ and
//! real phase information, neither of which the UI has); this page is a thin
//! front end onto [`sdroxide_types::Qo100Settings`] and
//! [`sdroxide_types::Qo100Status`] — the same split the ISM window keeps.
//!
//! Two jobs, split apart:
//! * **ON** runs the fast spectral tracker. Every second it looks in the
//!   operator's parking window (`+park_lo..+park_hi kHz`, shaded on the
//!   strip) for the beacon's two symmetric lobes with the null between them,
//!   and reports where the carrier sits — `null`, `sym` and `snr` say how
//!   convincing the shape was. It reads the beacon's *shape*, never its bits,
//!   so it works where the decoder cannot lock. **auto-correct** then closes
//!   the loop engine-side: on a clean, steady estimate it nudges
//!   `converter_offset_hz` so the beacon lands back on 10489.750 MHz and
//!   holds it there as the LNB drifts, deadbanded and rate-limited so a bad
//!   reading never yanks the receiver.
//! * **decode AO-40 telemetry** additionally runs the sync-word + CRC frame
//!   decoder, with a step-by-step readout (`carrier` → `sync` → `CRC`) of how
//!   far each pass got and how full the frame buffer is.
//!
//! When the decoder locks, the frequency it had to assume for the sync word
//! and CRC to check out *is* the station's whole frequency error — the LNB's
//! real LO plus whatever the SDR's own clock is off by, lumped together the
//! way an operator would correct them by hand. APPLY writes that correction
//! into [`sdroxide_types::RadioConfig::converter_offset_hz`] and reopens the
//! front end — the same round trip Settings ▸ Radio ▸ Apply makes — rather
//! than doing it silently on every lock, so a bad reading never yanks a
//! running receiver.
//!
//! The mini waterfall is visual context, not an automatic measurement: it
//! reads the same [`sdroxide_types::SpectrumFrame`] everything else on screen
//! already gets, cropped to a window around the beacon, so the operator can
//! see the Manchester null (and the search width buttons' effect) even
//! though nothing here hunts a peak in it. It does take one deliberate read
//! back, though — a double-click on the strip plants a "the beacon is here"
//! mark ([`Qo100WinState::manual_hz`]) that drives DRIFT and APPLY exactly as
//! a decoder lock would, for the case the null is plainly visible but the
//! demodulator will not lock.

use crate::i18n::tr;
use std::collections::VecDeque;

use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2,
};
use sdroxide_types::{Command, QO100_BEACON_HZ, Qo100Status, SpectrumFrame, Vfo};

use crate::app::SdroxideApp;
use crate::{colormap, theme};

/// Columns the mini waterfall's history is resampled to — independent of the
/// widget's pixel width, like [`crate::widgets::wide_spectrum`]'s `COLS`.
const COLS: usize = 240;
/// Rows of history kept.
const ROWS: usize = 90;
/// Height of the strip, in points.
const STRIP_H: f32 = 130.0;

/// The search-width step the width buttons move by, and the ends of the
/// range they're clamped to — "5 kHz and its multiples", wide enough at the
/// top for a station that has never been calibrated and narrow enough at the
/// bottom that the search is not paying to cover more band than any real
/// LNB drifts.
const WIDTH_STEP_HZ: f64 = 5_000.0;
const MIN_HALF_WIDTH_HZ: f64 = WIDTH_STEP_HZ;
const MAX_HALF_WIDTH_HZ: f64 = 50_000.0;

/// The parking-window steppers' step and the ends they clamp to. The spectral
/// tracker searches only `+park_lo_hz..+park_hi_hz` for the beacon's twin-lobe
/// shape, so the operator parks the beacon in a positive lane clear of the DC
/// spike before switching the tracker on.
const PARK_STEP_HZ: f64 = 1_000.0;
const PARK_MIN_HZ: f64 = 1_000.0;
const PARK_MAX_HZ: f64 = 40_000.0;
/// Smallest span the parking window is allowed to close to — the tracker needs
/// room for both lobes plus a stretch of floor either side.
const PARK_MIN_SPAN_HZ: f64 = 3_000.0;

/// Everything the window remembers between frames that is not a setting the
/// engine already tracks — purely the mini waterfall's own drawing state,
/// and the last correction actually applied.
#[derive(Default)]
pub(in crate::app) struct Qo100WinState {
    /// Newest row last, each [`COLS`] bytes of 0..=255 magnitude, resampled
    /// from whatever of the live [`SpectrumFrame`] falls in the current
    /// window.
    rows: VecDeque<Vec<u8>>,
    tex: Option<TextureHandle>,
    last_seq: u32,
    /// The (lo_hz, hi_hz) the stored rows were drawn against — a width change
    /// invalidates them outright rather than trying to rescale history drawn
    /// at a different span.
    window: Option<(f64, f64)>,
    /// The last correction actually written: old offset, new offset, and the
    /// wall-clock second it was applied, so the operator sees what happened
    /// even after the numbers above have moved on.
    applied: Option<(f64, f64, i64)>,
    /// A dial-domain frequency the operator double-clicked on the strip to
    /// say "the beacon is *here*" — for when its Manchester null is plainly
    /// visible but the decoder will not lock or decode. Drives DRIFT and
    /// APPLY exactly as a real lock would (see [`effective_measurement`]);
    /// cleared by the button that appears while it is set, by a real decoder
    /// lock superseding it, or once APPLY has written it.
    manual_hz: Option<f64>,
}

fn fmt_hz_signed(hz: f64) -> String {
    if hz.abs() >= 1000.0 {
        format!("{hz:+.2} kHz", hz = hz / 1000.0)
    } else {
        format!("{hz:+.0} Hz")
    }
}

/// A small round status light plus its label — lit green when `on`, a dim
/// grey dot otherwise. Used for the AO-40 decoder's stage readout.
fn led(ui: &mut egui::Ui, label: &str, on: bool) {
    let (dot, text) =
        if on { (theme::GREEN(), theme::TEXT()) } else { (theme::gray(70), theme::CYAN_DIM()) };
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, dot);
    ui.label(RichText::new(label).size(9.5).color(text));
    ui.add_space(4.0);
}

/// New [`sdroxide_types::RadioConfig::converter_offset_hz`] that puts the
/// beacon — currently read at `measured_hz` where it should read `target_hz`
/// — exactly on `target_hz`, for the same physical LNB and receiver.
///
/// Derived from the converter's own convention
/// (`sdroxide_radio::converter_open_hz`: `hardware_hz = dial_hz + offset`, so
/// `dial_hz = hardware_hz - offset`). The same physical signal read under two
/// offsets gives `measured_hz - target_hz = new_offset - old_offset`.
pub(in crate::app) fn corrected_offset_hz(
    old_offset_hz: f64,
    measured_hz: f64,
    target_hz: f64,
) -> f64 {
    old_offset_hz + (measured_hz - target_hz)
}

/// The bin range of `frame` overlapping `[lo_hz, hi_hz)`, clamped to what the
/// frame actually covers. `None` when the frame has no bins, no span, or does
/// not reach the requested window at all.
fn bin_range(frame: &SpectrumFrame, lo_hz: f64, hi_hz: f64) -> Option<std::ops::Range<usize>> {
    let n = frame.bins.len();
    if n == 0 || frame.span_hz <= 0.0 {
        return None;
    }
    let flo = frame.center_hz - frame.span_hz / 2.0;
    let fhi = frame.center_hz + frame.span_hz / 2.0;
    if hi_hz <= flo || lo_hz >= fhi {
        return None;
    }
    let bin_hz = frame.span_hz / n as f64;
    let b0 = (((lo_hz.max(flo) - flo) / bin_hz).floor() as isize).clamp(0, n as isize - 1) as usize;
    let b1 = (((hi_hz.min(fhi) - flo) / bin_hz).ceil() as isize).clamp(1, n as isize) as usize;
    (b1 > b0).then_some(b0..b1)
}

/// Fold a new frame into the history, resampled onto [`COLS`] against the
/// fixed `(lo, hi)` window. A width change clears the history outright rather
/// than rescaling it — cheap, and correct, since nothing here promises a
/// scrolling record of *other* widths.
fn push_row(win: &mut Qo100WinState, frame: &SpectrumFrame, lo: f64, hi: f64) {
    if frame.seq == win.last_seq && win.window == Some((lo, hi)) {
        return;
    }
    if win.window != Some((lo, hi)) {
        win.rows.clear();
        win.window = Some((lo, hi));
    }
    win.last_seq = frame.seq;

    let flo = frame.center_hz - frame.span_hz / 2.0;
    let fhi = frame.center_hz + frame.span_hz / 2.0;
    let n = frame.bins.len();
    let mut row = vec![0u8; COLS];
    if n > 0 && frame.span_hz > 0.0 && hi > flo && lo < fhi {
        let bin_hz = frame.span_hz / n as f64;
        for (c, slot) in row.iter_mut().enumerate() {
            let x0 = lo + (hi - lo) * c as f64 / COLS as f64;
            let x1 = lo + (hi - lo) * (c + 1) as f64 / COLS as f64;
            let (ox0, ox1) = (x0.max(flo), x1.min(fhi));
            if ox1 <= ox0 {
                continue; // this column falls outside what the receiver currently covers
            }
            let b0 = (((ox0 - flo) / bin_hz).floor() as isize).clamp(0, n as isize - 1) as usize;
            let b1 = (((ox1 - flo) / bin_hz).ceil() as isize).clamp(1, n as isize) as usize;
            if b1 > b0 {
                *slot = frame.bins[b0..b1].iter().copied().max().unwrap_or(0);
            }
        }
    }
    win.rows.push_back(row);
    while win.rows.len() > ROWS {
        win.rows.pop_front();
    }
    win.tex = None; // rebuilt on the next draw
}

fn texture<'a>(
    win: &'a mut Qo100WinState,
    ctx: &egui::Context,
    palette: usize,
) -> Option<&'a TextureHandle> {
    if win.tex.is_none() && !win.rows.is_empty() {
        let lut = colormap::lut(palette);
        // Newest row first, so the strip scrolls downward like every other
        // waterfall in the app.
        let mut pixels = Vec::with_capacity(COLS * win.rows.len());
        for row in win.rows.iter().rev() {
            pixels.extend(row.iter().map(|v| {
                let i = *v as usize * 4;
                Color32::from_rgb(lut[i], lut[i + 1], lut[i + 2])
            }));
        }
        let img = ColorImage::new([COLS, win.rows.len()], pixels);
        win.tex = Some(ctx.load_texture("qo100-waterfall", img, egui::TextureOptions::LINEAR));
    }
    win.tex.as_ref()
}

/// Draw the strip: the target line, where the decoder locked (if it has),
/// and any frequency the operator double-clicked to mark the beacon by hand.
/// The magnitude picture is visual only — nothing is read back off it (see
/// the module doc for why a peak search never worked on this signal) — but a
/// double-click *is* read back, in [`Qo100WinState::manual_hz`], as a
/// deliberate "the beacon is here" for DRIFT and APPLY to act on.
#[allow(clippy::too_many_arguments)]
fn paint_strip(
    ui: &mut egui::Ui,
    win: &mut Qo100WinState,
    frame: Option<&SpectrumFrame>,
    palette: usize,
    lo: f64,
    hi: f64,
    measured_hz: Option<f64>,
    // `park`: the parking window in dial Hz, shaded so the operator can see
    // where to put the beacon before switching the tracker on.
    // `est_hz`: the spectral tracker's current estimate, in dial Hz.
    park: Option<(f64, f64)>,
    est_hz: Option<f64>,
) {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), STRIP_H), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    // A double-click plants the hand-placed beacon mark at the frequency
    // under the pointer — the strip's x axis is a straight `lo..hi` ramp.
    if resp.double_clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let t = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
        win.manual_hz = Some(lo + t * (hi - lo));
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::BLACK);

    if let Some(f) = frame {
        push_row(win, f, lo, hi);
    }
    let ctx = ui.ctx().clone();
    if let Some(tex) = texture(win, &ctx, palette) {
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    let x_of = |hz: f64| -> f32 {
        rect.left() + ((hz - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 * rect.width()
    };
    // The parking window: shade it so "put the beacon in here" is literal.
    if let Some((plo, phi)) = park {
        let (xa, xb) = (x_of(plo), x_of(phi));
        if xb > xa {
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(xa, rect.top()), Pos2::new(xb, rect.bottom())),
                0.0,
                Color32::from_rgba_unmultiplied(0, 200, 255, 22),
            );
            for x in [xa, xb] {
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_white_alpha(40)),
                );
            }
        }
    }
    // The target: where the beacon belongs.
    let tx = x_of(QO100_BEACON_HZ);
    painter.line_segment(
        [Pos2::new(tx, rect.top()), Pos2::new(tx, rect.bottom())],
        Stroke::new(1.0, theme::CYAN()),
    );
    // Where the decoder actually locked, while that lock is still fresh.
    if let Some(m) = measured_hz
        && (lo..=hi).contains(&m)
    {
        let mx = x_of(m);
        painter.line_segment(
            [Pos2::new(mx, rect.top()), Pos2::new(mx, rect.bottom())],
            Stroke::new(1.6, theme::GREEN()),
        );
    }
    // The spectral tracker's live estimate.
    if let Some(m) = est_hz
        && (lo..=hi).contains(&m)
    {
        let mx = x_of(m);
        painter.line_segment(
            [Pos2::new(mx, rect.top()), Pos2::new(mx, rect.bottom())],
            Stroke::new(1.4, theme::YELLOW()),
        );
        painter.text(
            Pos2::new((mx + 3.0).min(rect.right() - 2.0), rect.top() + 12.0),
            egui::Align2::LEFT_TOP,
            "tracker",
            egui::FontId::monospace(8.0),
            theme::YELLOW(),
        );
    }
    // The hand-placed mark, if the operator has double-clicked one in.
    if let Some(m) = win.manual_hz
        && (lo..=hi).contains(&m)
    {
        let mx = x_of(m);
        painter.line_segment(
            [Pos2::new(mx, rect.top()), Pos2::new(mx, rect.bottom())],
            Stroke::new(1.4, theme::PINK()),
        );
        painter.text(
            Pos2::new((mx + 3.0).min(rect.right() - 2.0), rect.bottom() - 11.0),
            egui::Align2::LEFT_BOTTOM,
            "clicked",
            egui::FontId::monospace(8.0),
            theme::PINK(),
        );
    }

    let font = egui::FontId::monospace(9.0);
    let dim = Color32::from_white_alpha(200);
    painter.text(
        Pos2::new(rect.left() + 3.0, rect.top() + 1.0),
        egui::Align2::LEFT_TOP,
        format!("{e6:.3} MHz", e6 = lo / 1e6),
        font.clone(),
        dim,
    );
    painter.text(
        Pos2::new(rect.right() - 3.0, rect.top() + 1.0),
        egui::Align2::RIGHT_TOP,
        format!("{e6:.3} MHz", e6 = hi / 1e6),
        font,
        dim,
    );
    crate::chrome::paint_cut_border(&painter, rect, theme::LINE_LIT(), theme::PANEL());
}

impl SdroxideApp {
    pub(in crate::app) fn qo100_body(
        &mut self,
        ui: &mut egui::Ui,
        win: &mut Qo100WinState,
        cmds: &mut Vec<Command>,
    ) {
        // `may_rx_hz`, not `can_rx_hz`: a driver that publishes no tuning
        // ranges (SoapySDR makes `getFrequencyRange` optional, and plenty of
        // backends skip it) has not said it *cannot* reach the beacon. Gating
        // on `can_rx_hz` there greys ON out on a correctly-set-up LNB station
        // and tells it to configure a converter offset it already has.
        let reachable = self.caps.as_ref().is_none_or(|c| c.may_rx_hz(QO100_BEACON_HZ));
        let frame = self.frame.clone();
        // Edited in place and sent whole on any change, the same convention
        // the ISM window follows for `IsmSettings` — the engine persists this
        // and echoes it back, so there is no separate apply step.
        let mut cfg = self.state.qo100;
        // The strip spans the telemetry search width either side of the
        // beacon, and — while the tracker is on — out far enough on the high
        // side to show the whole parking window too.
        let hw = cfg.search_half_width_hz;
        let (lo, hi) = (
            QO100_BEACON_HZ - hw,
            QO100_BEACON_HZ + if cfg.enabled { hw.max(cfg.park_hi_hz + 2_000.0) } else { hw },
        );
        // Whether whatever the receiver is *actually* capturing right now
        // reaches anywhere near the beacon at all — as against `reachable`,
        // which asks whether it ever could. A capable receiver still shows a
        // blank strip while it is parked on 144 MHz, and that is the second
        // most likely reason (after no converter at all) this window opens on
        // an empty waterfall. This only judges the *mini waterfall*'s own
        // picture — the decoder itself reads the raw IQ straight from the
        // hardware and does not depend on the main dial being anywhere near
        // the beacon at all, only on the beacon being inside what the
        // hardware actually captures.
        let in_view = frame.as_deref().is_some_and(|f| bin_range(f, lo, hi).is_some());
        let dial_hz = self.state.active_freq_hz();
        let status = self.qo100_status.clone();

        ui.horizontal(|ui| {
            let run = crate::chrome::chip_enabled(
                ui,
                reachable,
                cfg.enabled,
                if cfg.enabled { "ON" } else { "OFF" },
            );
            if run.clicked() {
                cfg.enabled = !cfg.enabled;
                // The main dial is left exactly where it is: the tracker mixes
                // its own downconversion onto the beacon out of the raw IQ, so
                // as long as 10489.750 MHz is inside what the hardware is
                // already capturing there is nothing to retune. An operator
                // whose capture does not reach the beacon still has the
                // explicit "Tune to 10489.750 MHz" button below.
            }
            if !reachable {
                run.on_hover_text(tr!("qo100-receiver-cannot-reach-10489-750"));
            } else {
                run.on_hover_text(tr!("qo100-run-fast-spectral-tracker-every"));
            }

            ui.add_space(8.0);
            let tel = crate::chrome::chip_enabled(
                ui,
                reachable,
                cfg.decode_telemetry,
                &tr!("qo100-telemetry"),
            );
            if tel.clicked() {
                cfg.decode_telemetry = !cfg.decode_telemetry;
            }
            tel.on_hover_text(tr!("qo100-also-run-ao-40-uncoded"));

            ui.add_space(4.0);
            let auto = crate::chrome::chip_enabled(
                ui,
                reachable && cfg.enabled,
                cfg.auto_apply,
                &tr!("qo100-auto"),
            );
            if auto.clicked() {
                cfg.auto_apply = !cfg.auto_apply;
            }
            auto.on_hover_text(tr!("qo100-auto-correct-let-tracker-adjust"));

            ui.add_space(8.0);
            ui.label(RichText::new("width").size(10.0).color(theme::CYAN_DIM()));
            if ui
                .small_button("−")
                .on_hover_text(tr!("qo100-narrower-search-smaller-slice"))
                .clicked()
            {
                cfg.search_half_width_hz =
                    (cfg.search_half_width_hz - WIDTH_STEP_HZ).max(MIN_HALF_WIDTH_HZ);
            }
            ui.label(
                RichText::new(format!(
                    "±{search_half_width_hz:.0} kHz",
                    search_half_width_hz = cfg.search_half_width_hz / 1000.0
                ))
                .size(11.0)
                .monospace(),
            );
            if ui.small_button("+").on_hover_text(tr!("qo100-wider-when-beacon-isn-t")).clicked() {
                cfg.search_half_width_hz =
                    (cfg.search_half_width_hz + WIDTH_STEP_HZ).min(MAX_HALF_WIDTH_HZ);
            }

            if win.manual_hz.is_some() {
                ui.add_space(8.0);
                if ui
                    .small_button(tr!("qo100-clear-mark"))
                    .on_hover_text(tr!("qo100-drop-hand-placed-beacon-mark"))
                    .clicked()
                {
                    win.manual_hz = None;
                }
            }
        });

        // The parking window the spectral tracker searches. Two steppers in
        // 1 kHz clicks; the low edge cannot cross within PARK_MIN_SPAN of the
        // high one, or the tracker loses the room it needs for both lobes.
        ui.horizontal(|ui| {
            ui.label(RichText::new("park").size(10.0).color(theme::CYAN_DIM()));
            if ui.small_button(tr!("qo100-lo")).clicked() {
                cfg.park_lo_hz = (cfg.park_lo_hz - PARK_STEP_HZ).max(PARK_MIN_HZ);
            }
            if ui.small_button(tr!("qo100-lo-2")).clicked() {
                cfg.park_lo_hz =
                    (cfg.park_lo_hz + PARK_STEP_HZ).min(cfg.park_hi_hz - PARK_MIN_SPAN_HZ);
            }
            ui.label(
                RichText::new(tr!(
                    "qo100-khz-3",
                    park_lo_hz = format!("{:.0}", cfg.park_lo_hz / 1000.0),
                    park_hi_hz = format!("{:.0}", cfg.park_hi_hz / 1000.0)
                ))
                .size(11.0)
                .monospace(),
            );
            if ui.small_button(tr!("qo100-hi")).clicked() {
                cfg.park_hi_hz =
                    (cfg.park_hi_hz - PARK_STEP_HZ).max(cfg.park_lo_hz + PARK_MIN_SPAN_HZ);
            }
            if ui.small_button(tr!("qo100-hi-2")).clicked() {
                cfg.park_hi_hz = (cfg.park_hi_hz + PARK_STEP_HZ).min(PARK_MAX_HZ);
            }
        });
        if !cfg.enabled {
            ui.label(
                RichText::new(tr!(
                    "qo100-before-switching-tune-so-beacon",
                    park_lo_hz = format!("{:.0}", cfg.park_lo_hz / 1000.0),
                    park_hi_hz = format!("{:.0}", cfg.park_hi_hz / 1000.0)
                ))
                .size(9.0)
                .color(theme::CYAN_DIM()),
            );
        }

        // Unmissable, not just a hover: a fresh station has no converter set
        // up yet, so ON stays disabled and the strip would otherwise sit
        // there blank with no clue why — the single most likely reason
        // anyone ever opens this window and sees nothing.
        if !reachable {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr!("qo100-radio-s-own-tuning-range"))
                        .size(10.5)
                        .color(theme::YELLOW()),
                );
                if ui.small_button(tr!("qo100-open-settings-radio")).clicked() {
                    self.open_radio_settings();
                }
            });
        }

        // The receiver *can* reach the beacon but currently is not — parked
        // on some other band, most often. Only the *mini waterfall* has
        // nothing to show either way; the decoder itself is unaffected (see
        // `in_view`'s own doc), so this is about the picture, not the search.
        if reachable && !in_view {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(tr!(
                        "qo100-receiver-currently-listening-around-mhz",
                        mhz = format!("{:.6}", dial_hz / 1e6)
                    ))
                    .size(10.5)
                    .color(theme::YELLOW()),
                );
                if ui.small_button(tr!("qo100-tune-10489-750-mhz")).clicked() {
                    cmds.push(Command::SetVfo { vfo: Vfo::A, hz: QO100_BEACON_HZ });
                }
            });
        }

        if let Some(f) = self.frame.as_ref() {
            let capped = f.span_hz / 2.0 < cfg.search_half_width_hz;
            if capped && f.span_hz > 0.0 {
                ui.label(
                    RichText::new(tr!(
                        "qo100-receiver-currently-covers-only-khz",
                        khz = format!("{:.0}", f.span_hz / 2e3)
                    ))
                    .size(9.5)
                    .color(theme::YELLOW()),
                );
            }
        }

        ui.add_space(4.0);
        let measured_hz = locked_freq(status.as_ref());
        let tracker_dial_hz =
            status.as_ref().and_then(|s| s.est_offset_hz).map(|o| QO100_BEACON_HZ + o);
        paint_strip(
            ui,
            win,
            frame.as_deref(),
            self.ui_settings.waterfall_palette,
            lo,
            hi,
            measured_hz,
            Some((QO100_BEACON_HZ + cfg.park_lo_hz, QO100_BEACON_HZ + cfg.park_hi_hz)),
            tracker_dial_hz,
        );

        // `paint_strip` may have just planted or cleared the hand-placed
        // mark; from here on DRIFT and APPLY act on whichever reading
        // [`effective_measurement`] picks.
        let confirmed = apply_is_confirmed(status.as_ref());
        let effective =
            effective_measurement(measured_hz, confirmed, win.manual_hz, tracker_dial_hz);

        ui.add_space(6.0);
        ui.label(
            RichText::new(status_line(cfg.enabled, status.as_ref(), self.ctrl.engine_is_remote()))
                .size(9.5)
                .color(theme::CYAN_DIM()),
        );
        // Discoverability: a stalled search or a lock that never decodes is
        // exactly when double-click matters, so say so — but only while it
        // would actually do something new (no confirmed decode, no mark yet).
        if cfg.enabled && !confirmed && win.manual_hz.is_none() {
            ui.label(
                RichText::new(tr!("qo100-tip-if-you-can-see")).size(9.0).color(theme::CYAN_DIM()),
            );
        }

        // The spectral tracker's live readout — one line per cycle, so a
        // parked beacon that the tracker cannot see is obvious immediately.
        if let Some(s) = status.as_ref().filter(|s| s.tracking) {
            ui.add_space(4.0);
            egui::Grid::new("qo100-tracker").num_columns(2).spacing([16.0, 2.0]).show(ui, |ui| {
                let dim = |t: &str| RichText::new(t).size(9.5).color(theme::CYAN_DIM());
                ui.label(dim(&tr!("qo100-tracker")));
                match s.est_offset_hz {
                    Some(o) => {
                        let good = s.est_null_depth_db >= 5.0 && s.est_symmetry >= 0.7;
                        ui.label(
                            RichText::new(format!(
                                "{o} → {e6:.6} MHz",
                                o = fmt_hz_signed(o),
                                e6 = (QO100_BEACON_HZ + o) / 1e6
                            ))
                            .size(11.0)
                            .monospace()
                            .color(if good {
                                theme::GREEN()
                            } else {
                                theme::YELLOW()
                            }),
                        );
                    }
                    None => {
                        ui.label(
                            RichText::new(tr!("qo100-no-twin-lobe-shape-parking"))
                                .size(10.0)
                                .color(theme::YELLOW()),
                        );
                    }
                }
                ui.end_row();

                ui.label(dim(&tr!("qo100-shape")));
                ui.label(
                    RichText::new(tr!(
                        "qo100-null-db-sym-snr-db",
                        est_null_depth_db = format!("{:.1}", s.est_null_depth_db),
                        est_symmetry = format!("{:.2}", s.est_symmetry),
                        est_snr_db = format!("{:.1}", s.est_snr_db)
                    ))
                    .size(10.0)
                    .monospace(),
                );
                ui.end_row();

                ui.label(dim(&tr!("qo100-drift")));
                ui.label(
                    RichText::new(if s.est_drift_hz_s.abs() >= 0.05 {
                        if s.est_drift_accel_hz_s2.abs() >= 0.05 {
                            tr!(
                                "qo100-hz-s-hz-s-de",
                                est_drift_hz_s = format!("{:+.1}", s.est_drift_hz_s),
                                est_drift_accel_hz_s2 = format!("{:+.2}", s.est_drift_accel_hz_s2)
                            )
                        } else {
                            tr!(
                                "qo100-hz-s-de-rotated-before",
                                est_drift_hz_s = format!("{:+.1}", s.est_drift_hz_s)
                            )
                        }
                    } else {
                        tr!("qo100-need-longer-run-estimates").to_string()
                    })
                    .size(10.0)
                    .monospace()
                    .color(
                        if s.est_drift_hz_s.abs() > 15.0 || s.est_drift_accel_hz_s2.abs() > 3.0 {
                            theme::YELLOW()
                        } else {
                            theme::TEXT()
                        },
                    ),
                );
                ui.end_row();

                ui.label(dim(&tr!("qo100-cycles")));
                ui.label(
                    RichText::new(tr!(
                        "qo100-found-empty",
                        est_updates = s.est_updates,
                        est_misses = s.est_misses
                    ))
                    .size(10.0)
                    .monospace(),
                );
                ui.end_row();

                if s.auto_applying {
                    ui.label(dim(&tr!("qo100-auto")));
                    let last = if s.auto_last_unix > 0 {
                        tr!(
                            "qo100-last-s-ago",
                            auto_last_hz = fmt_hz_signed(s.auto_last_hz),
                            auto_last_unix = (crate::time::now_unix() - s.auto_last_unix).max(0)
                        )
                    } else {
                        String::new()
                    };
                    ui.label(
                        RichText::new(tr!(
                            "qo100-over-correction",
                            auto_total_hz = fmt_hz_signed(s.auto_total_hz),
                            auto_applies = s.auto_applies,
                            last = last
                        ))
                        .size(10.0)
                        .monospace()
                        .color(theme::GREEN()),
                    );
                    ui.end_row();
                }
            });
        }

        let radio_cfg = self.ctrl.radio_config();
        // The offset in force *at the beacon*, which on a station with a
        // transverter row covering 10489.750 MHz is that row's and not the
        // single converter offset — correcting the wrong one leaves the beacon
        // exactly where it was (`RadioConfig::converter_offset_at`).
        let old_offset =
            radio_cfg.as_ref().map(|c| c.converter_offset_at(QO100_BEACON_HZ)).unwrap_or(0.0);

        egui::Grid::new("qo100-grid").num_columns(2).spacing([16.0, 3.0]).show(ui, |ui| {
            let dim = |s: &str| RichText::new(s).size(9.5).color(theme::CYAN_DIM());
            // What the receiver is actually listening to right now — the
            // direct answer to "why is the strip empty", always on screen
            // rather than only when it explains a problem.
            ui.label(dim(&tr!("qo100-receiver")));
            ui.label(
                RichText::new(format!("{e6:.6} MHz", e6 = dial_hz / 1e6))
                    .size(11.0)
                    .monospace()
                    .color(if in_view { theme::TEXT() } else { theme::YELLOW() }),
            );
            ui.end_row();

            ui.label(dim(&tr!("qo100-target")));
            ui.label(
                RichText::new(format!("{e6:.6} MHz", e6 = QO100_BEACON_HZ / 1e6))
                    .size(12.0)
                    .monospace(),
            );
            ui.end_row();

            ui.label(dim(&tr!("qo100-measured")));
            match effective {
                Some((hz, src)) => {
                    let (tag, colour) = match src {
                        MeasSource::Lock => (String::new(), None),
                        MeasSource::Manual => (tr!("qo100-clicked"), Some(theme::PINK())),
                        MeasSource::Tracker => (tr!("qo100-tracker-2"), Some(theme::YELLOW())),
                    };
                    let mut t =
                        RichText::new(format!("{e6:.6} MHz{tag}", e6 = hz / 1e6, tag = tag))
                            .size(12.0)
                            .monospace();
                    t = match colour {
                        Some(c) => t.color(c),
                        None => t.strong(),
                    };
                    ui.label(t)
                }
                None => ui.label(
                    RichText::new(if cfg.enabled {
                        tr!("qo100-not-locked-yet")
                    } else {
                        String::from("—")
                    })
                    .size(11.0)
                    .color(theme::CYAN_DIM()),
                ),
            };
            ui.end_row();

            ui.label(dim(&tr!("qo100-drift")));
            match effective {
                Some((hz, _)) => {
                    let err = hz - QO100_BEACON_HZ;
                    let colour = if err.abs() < 200.0 {
                        theme::GREEN()
                    } else if err.abs() < 3000.0 {
                        theme::YELLOW()
                    } else {
                        theme::TEXT()
                    };
                    ui.label(RichText::new(fmt_hz_signed(err)).size(12.0).monospace().color(colour))
                }
                None => ui.label(RichText::new("—").size(11.0).color(theme::CYAN_DIM())),
            };
            ui.end_row();

            ui.label(dim(&tr!("qo100-converter-offset")));
            ui.label(RichText::new(format!("{old_offset:.0} Hz")).size(11.0).monospace());
            ui.end_row();
        });

        // The AO-40 decoder's step-by-step readout: which stage the last pass
        // reached, and how full the buffer is toward the whole frame it needs.
        if let Some(s) = status.as_ref().filter(|s| s.decoding) {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr!("qo100-decode")).size(9.5).color(theme::CYAN_DIM()));
                led(ui, "carrier", s.carrier_seen);
                led(ui, "sync", s.sync_seen);
                led(ui, "CRC", s.crc_ok);
                if s.sync_bit_errors != u8::MAX {
                    // Always shown: sync passing (≤3) but CRC never lighting
                    // means the demod is marginal — the closer this is to 0,
                    // the more of the payload is decoding right. The ×N is how
                    // many distinct sync alignments the best bitstream held: a
                    // real frame shows ×1–2 at very few errors, a lone chance
                    // hit shows ×1 near 3 errors.
                    let real = s.sync_bit_errors <= 1 && s.sync_matches >= 1;
                    ui.label(
                        RichText::new(tr!(
                            "qo100-sync-32-err",
                            sync_bit_errors = s.sync_bit_errors,
                            sync_matches = s.sync_matches
                        ))
                        .size(9.0)
                        .color(if real {
                            theme::GREEN()
                        } else if s.sync_bit_errors <= 3 {
                            theme::CYAN_DIM()
                        } else {
                            theme::YELLOW()
                        }),
                    );
                }
            });
            ui.add(
                egui::ProgressBar::new(s.frame_fill).desired_height(6.0).text(
                    RichText::new(tr!(
                        "qo100-frame-buffer",
                        frame_fill = format!("{:.0}", s.frame_fill * 100.0)
                    ))
                    .size(8.0),
                ),
            );
            ui.label(
                RichText::new(tr!(
                    "qo100-blocks-tried-decoded",
                    blocks_tried = s.blocks_tried,
                    blocks_locked = s.blocks_locked
                ))
                .size(9.0)
                .color(theme::CYAN_DIM()),
            );
        } else if cfg.decode_telemetry {
            // Toggled on but no status back yet — the first ~24 s window.
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr!("qo100-decode-filling-first-frame-buffer"))
                    .size(9.5)
                    .color(theme::CYAN_DIM()),
            );
        } else {
            ui.add_space(4.0);
            ui.label(
                RichText::new(tr!("qo100-decode-off-turn-telemetry-chip"))
                    .size(9.0)
                    .color(theme::CYAN_DIM()),
            );
        }

        // The decoded telemetry text — the beacon's own status report, shown
        // for its own sake and as independent confirmation the decode is
        // real: garbage here despite a "locked" CRC would be a red flag no
        // number above could catch.
        if let Some(s) = status.as_ref().filter(|s| !s.text.is_empty()) {
            ui.add_space(4.0);
            ui.label(RichText::new(tr!("qo100-telemetry")).size(9.5).color(theme::CYAN_DIM()));
            egui::Frame::new().fill(theme::INPUT_BG()).inner_margin(6.0).show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(&s.text).monospace().size(10.5).color(theme::GREEN()),
                    )
                    .wrap(),
                );
            });
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            // A hand-placed mark or the tracker's estimate is a deliberate
            // operator action, so it needs no second-frame confirmation; a
            // bare decoder lock still does (see `apply_is_confirmed`).
            let can_apply = radio_cfg.is_some()
                && match effective {
                    Some((_, MeasSource::Lock)) => confirmed,
                    Some((_, MeasSource::Manual | MeasSource::Tracker)) => true,
                    None => false,
                };
            let apply = ui.add_enabled(
                can_apply,
                egui::Button::new(RichText::new(tr!("qo100-apply-correction")).strong()),
            );
            let apply = apply.on_hover_text(match effective {
                Some((_, MeasSource::Manual)) => {
                    tr!("qo100-write-converter-lnb-offset-puts")
                }
                Some((_, MeasSource::Tracker)) => {
                    tr!("qo100-write-converter-lnb-offset-puts-2")
                }
                Some((_, MeasSource::Lock)) if !confirmed => {
                    tr!("qo100-waiting-second-crc-valid-frame")
                }
                _ => {
                    tr!("qo100-write-corrected-converter-lnb-offset")
                }
            });
            if apply.clicked()
                && let (Some(mut c), Some((measured, _))) = (radio_cfg.clone(), effective)
            {
                let slot = c.converter_offset_at_mut(QO100_BEACON_HZ);
                let new_offset = corrected_offset_hz(*slot, measured, QO100_BEACON_HZ);
                *slot = new_offset;
                self.ctrl.set_radio_config(c.clone());
                self.ctrl.reopen_source();
                self.radio_cfg = Some(c);
                win.applied = Some((old_offset, new_offset, crate::time::now_unix()));
                // The correction now lives in the offset and the beacon should
                // land near centre next sweep; a stale mark would only mislead.
                win.manual_hz = None;
            }
            if let Some((old, new, at)) = win.applied {
                let ago = crate::time::now_unix() - at;
                ui.label(
                    RichText::new(tr!(
                        "qo100-last-applied-s-ago-hz",
                        ago = ago,
                        old = format!("{:.0}", old),
                        new = format!("{:.0}", new),
                        old2 = format!("{:+.0}", new - old)
                    ))
                    .size(9.5)
                    .color(theme::CYAN_DIM()),
                );
            }
        });

        if cfg != self.state.qo100 {
            cmds.push(Command::SetQo100Config(cfg));
        }
    }
}

/// The beacon's dial-domain frequency while the decoder's lock is still
/// fresh — `None` once it has gone stale (see
/// [`sdroxide_types::Qo100Status::locked`]'s own doc for how long that grace
/// period is) or if the decoder has never locked at all.
fn locked_freq(status: Option<&Qo100Status>) -> Option<f64> {
    status.filter(|s| s.locked).map(|s| QO100_BEACON_HZ + s.offset_hz)
}

/// Where a DRIFT / APPLY figure came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum MeasSource {
    /// A CRC-valid AO-40 frame decode.
    Lock,
    /// The frequency the operator double-clicked on the strip.
    Manual,
    /// The spectral tracker's twin-lobe estimate.
    Tracker,
}

/// The frequency DRIFT and APPLY act on, and where it came from.
///
/// A *confirmed* decoder lock always wins — it is a phase measurement off the
/// raw IQ. Then the operator's hand-placed mark, then the spectral tracker's
/// estimate, then — last, and never applied — an unconfirmed lock, which
/// still shows in DRIFT but cannot be written (see `apply_is_confirmed`).
fn effective_measurement(
    locked_hz: Option<f64>,
    confirmed: bool,
    manual_hz: Option<f64>,
    tracker_hz: Option<f64>,
) -> Option<(f64, MeasSource)> {
    match (locked_hz, manual_hz, tracker_hz) {
        (Some(hz), _, _) if confirmed => Some((hz, MeasSource::Lock)),
        (_, Some(hz), _) => Some((hz, MeasSource::Manual)),
        (_, _, Some(hz)) => Some((hz, MeasSource::Tracker)),
        (Some(hz), _, _) => Some((hz, MeasSource::Lock)),
        (None, None, None) => None,
    }
}

/// Whether a measured offset is safe to offer for `APPLY CORRECTION`, which
/// writes it straight into the converter/LNB setting.
///
/// A 32-bit sync word matched within three bit errors, times a 16-bit CRC,
/// turns up by chance roughly once every couple of hours of searching, so a
/// single lock is not enough to act on. The button stays disabled until a
/// second CRC-valid frame lands (`blocks_locked >= 2`) carrying a non-empty
/// decoded payload — a false positive that clears both is vanishingly
/// unlikely, and the operator can still eyeball the TELEMETRY panel.
fn apply_is_confirmed(status: Option<&Qo100Status>) -> bool {
    status.is_some_and(|s| s.blocks_locked >= 2 && !s.text.is_empty())
}

/// The one-line summary under the strip: off, searching (with how many
/// blocks it has tried), or locked — the same "attempted vs. succeeded"
/// distinction `IsmStatus`'s bursts/decodes line exists for, so a search
/// that is running but has not found the beacon yet reads differently from
/// one that never started.
///
/// `remote` is the client-does-not-own-the-engine case: the decoder runs on
/// the station and its status has no path to a remote client yet (see
/// `RadioEvent::Qo100Status`), so rather than sit on "starting…" forever the
/// line says plainly that this readout is local to the receiving station.
fn status_line(enabled: bool, status: Option<&Qo100Status>, remote: bool) -> String {
    if !enabled {
        return String::new();
    }
    if remote {
        return tr!("qo100-decoder-runs-receiving-station-readout").to_string();
    }
    match status {
        None => tr!("qo100-starting").to_string(),
        Some(s) if s.locked => {
            tr!(
                "qo100-locked-block-tried-locked",
                blocks_tried = s.blocks_tried,
                blocks_locked = s.blocks_locked
            )
        }
        Some(s) if s.blocks_tried == 0 => {
            tr!("qo100-searching-first-window-fills-after").to_string()
        }
        Some(s) => tr!("qo100-searching-block-tried-none-locked", blocks_tried = s.blocks_tried),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beacon_read_high_needs_the_offset_raised() {
        // Beacon really sits on QO100_BEACON_HZ but reads 5 kHz high — the
        // LNB's LO is 5 kHz low, which the software currently under-subtracts.
        let old = -9_750_000_000.0;
        let measured = QO100_BEACON_HZ + 5_000.0;
        assert_eq!(corrected_offset_hz(old, measured, QO100_BEACON_HZ), old + 5_000.0);
    }

    #[test]
    fn a_beacon_read_low_needs_the_offset_lowered() {
        let old = -9_750_000_000.0;
        let measured = QO100_BEACON_HZ - 1_200.0;
        assert_eq!(corrected_offset_hz(old, measured, QO100_BEACON_HZ), old - 1_200.0);
    }

    #[test]
    fn a_correctly_calibrated_station_gets_the_same_offset_back() {
        let old = -9_750_000_000.0;
        assert_eq!(corrected_offset_hz(old, QO100_BEACON_HZ, QO100_BEACON_HZ), old);
    }

    fn frame_with(center_hz: f64, span_hz: f64, bins: Vec<u8>) -> SpectrumFrame {
        SpectrumFrame {
            seq: 1,
            center_hz,
            span_hz,
            db_floor: -120.0,
            db_ceil: -20.0,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    #[test]
    fn bin_range_refuses_a_window_the_frame_never_reaches() {
        let frame = frame_with(14_000_000.0, 100_000.0, vec![0u8; 64]);
        assert!(bin_range(&frame, QO100_BEACON_HZ - 5_000.0, QO100_BEACON_HZ + 5_000.0).is_none());
    }

    #[test]
    fn bin_range_clamps_to_what_the_frame_actually_covers() {
        // Requested window straddles the frame's edge; the range returned
        // must stay inside 0..bins.len().
        let frame = frame_with(QO100_BEACON_HZ + 3_000.0, 10_000.0, vec![0u8; 100]);
        let r = bin_range(&frame, QO100_BEACON_HZ - 5_000.0, QO100_BEACON_HZ + 5_000.0)
            .expect("overlaps");
        assert!(r.end <= 100);
    }

    fn status(locked: bool, offset_hz: f64) -> Qo100Status {
        Qo100Status { running: true, locked, offset_hz, ..Default::default() }
    }

    #[test]
    fn locked_freq_reads_off_the_target_plus_the_measured_offset() {
        let s = status(true, 1_234.0);
        assert_eq!(locked_freq(Some(&s)), Some(QO100_BEACON_HZ + 1_234.0));
    }

    #[test]
    fn locked_freq_is_none_when_not_locked_or_absent() {
        assert_eq!(locked_freq(None), None);
        assert_eq!(locked_freq(Some(&status(false, 0.0))), None);
    }

    #[test]
    fn effective_measurement_prefers_a_confirmed_lock_over_everything_else() {
        let m = effective_measurement(
            Some(QO100_BEACON_HZ + 100.0),
            true,
            Some(QO100_BEACON_HZ + 9_000.0),
            Some(QO100_BEACON_HZ + 14_000.0),
        );
        assert_eq!(m, Some((QO100_BEACON_HZ + 100.0, MeasSource::Lock)));
    }

    #[test]
    fn effective_measurement_lets_the_mark_win_over_a_lock_that_will_not_decode() {
        let m = effective_measurement(
            Some(QO100_BEACON_HZ + 100.0),
            false,
            Some(QO100_BEACON_HZ + 9_000.0),
            Some(QO100_BEACON_HZ + 14_000.0),
        );
        assert_eq!(m, Some((QO100_BEACON_HZ + 9_000.0, MeasSource::Manual)));
    }

    #[test]
    fn effective_measurement_uses_the_tracker_when_there_is_no_lock_or_mark() {
        let m = effective_measurement(None, false, None, Some(QO100_BEACON_HZ + 14_000.0));
        assert_eq!(m, Some((QO100_BEACON_HZ + 14_000.0, MeasSource::Tracker)));
    }

    #[test]
    fn effective_measurement_shows_an_unconfirmed_lock_as_a_last_resort() {
        let m = effective_measurement(Some(QO100_BEACON_HZ + 100.0), false, None, None);
        assert_eq!(m, Some((QO100_BEACON_HZ + 100.0, MeasSource::Lock)));
    }

    #[test]
    fn effective_measurement_is_none_with_no_source_at_all() {
        assert_eq!(effective_measurement(None, false, None, None), None);
    }

    #[test]
    fn a_hand_placed_mark_feeds_the_same_offset_maths_as_a_lock() {
        let old = -9_750_000_000.0;
        let (clicked, src) =
            effective_measurement(None, false, Some(QO100_BEACON_HZ + 9_000.0), None).unwrap();
        assert_eq!(src, MeasSource::Manual);
        assert_eq!(corrected_offset_hz(old, clicked, QO100_BEACON_HZ), old + 9_000.0);
    }

    #[test]
    fn status_line_is_blank_while_switched_off() {
        assert_eq!(status_line(false, Some(&status(true, 0.0)), false), "");
        assert_eq!(status_line(false, None, true), "");
    }

    #[test]
    fn status_line_distinguishes_locked_from_still_searching() {
        assert!(status_line(true, Some(&status(true, 0.0)), false).starts_with("locked"));
        let mut searching = status(false, 0.0);
        searching.blocks_tried = 3;
        assert!(status_line(true, Some(&searching), false).starts_with("searching"));
    }

    #[test]
    fn status_line_tells_a_remote_client_the_readout_is_local() {
        // No status will ever arrive on a remote client (the server drops
        // the event), so the line must not sit on "starting…".
        let line = status_line(true, None, true);
        assert!(line.contains("receiving station"), "{line:?}");
        assert_ne!(line, "starting…");
    }

    #[test]
    fn status_line_before_the_first_window_says_how_long_it_takes() {
        // A search that is running but has not filled a window yet — distinct
        // from one that never started (`None` → "starting…").
        let mut s = status(false, 0.0);
        s.blocks_tried = 0;
        let line = status_line(true, Some(&s), false);
        assert!(line.starts_with("searching"), "{line:?}");
        assert!(line.contains("24 s"), "{line:?}");
    }

    #[test]
    fn apply_stays_disabled_until_a_second_crc_valid_frame_with_text() {
        assert!(!apply_is_confirmed(None), "nothing decoded yet");

        let mut s = status(true, 1_000.0);
        s.blocks_locked = 1;
        s.text = "QO-100 XX".into();
        assert!(!apply_is_confirmed(Some(&s)), "one lock could be a chance match");

        s.blocks_locked = 2;
        s.text = String::new();
        assert!(!apply_is_confirmed(Some(&s)), "a blank payload is not a confirmation");

        s.text = "QO-100 XX".into();
        assert!(apply_is_confirmed(Some(&s)), "two locks and real text: safe to offer");
    }
}
