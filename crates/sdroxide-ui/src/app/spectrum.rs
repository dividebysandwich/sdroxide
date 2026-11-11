//! The panadapter and waterfall: what the engine is asked for, and what gets
//! drawn on top of what comes back.
//!
//! Viewport and FFT changes are debounced before they go out (dragging the
//! span would otherwise reconfigure the engine every frame), and the overlays
//! — CW skimmer boxes, FT8 callsign labels, network spot flags — are rebuilt
//! here each frame from state the rest of the app maintains.

use crate::i18n::tr;
use eframe::egui::Color32;
use sdroxide_types::{Mode, SkimmerKind, SkimmerSpot, SpectrumConfig, Spot, SpotKind};

use crate::time::{now_unix, now_unix_f64};
use crate::waterfall_gpu;
use crate::widgets::spectrum_view;

use crate::app::SdroxideApp;

/// What this machine will actually draw, so the SPEC popup's detail row can
/// show the operator what `Auto` decided and grey what the renderer cannot
/// hold. Built by [`SdroxideApp::detail_report`].
///
/// Worked out here rather than in `sdroxide-types`, where
/// [`sdroxide_types::UiSettings`] lives: that crate must never learn about
/// wgpu.
pub(in crate::app) struct DetailReport {
    /// Columns in force right now, whatever the setting says.
    pub chosen: u32,
    /// The widest the operator may pick on this renderer.
    pub ceiling: u32,
    /// One sentence naming what bound it, for the hover on a greyed chip.
    pub reason: String,
}

/// Viewport/FFT config updates are sent once the view has been stable this
/// long (seconds of egui time — `std::time::Instant` panics on wasm).
pub(in crate::app) const CFG_DEBOUNCE_S: f64 = 0.25;

/// A skimmer box fades to nothing over this many seconds after its signal
/// stops keying, instead of vanishing.
pub(in crate::app) const SKIMMER_FADE_SECS: f64 = 5.0;

/// Shortest gap between the starts of two automatic fits (seconds of egui
/// time). A fit is a glide rather than a jump (see [`FIT_STEP_FRAC`]), so this
/// is also about as long as one takes to arrive.
const FIT_MIN_GAP_S: f64 = 5.0;

/// How long the view has to hold still before an automatic fit lands on it.
/// Longer than [`CFG_DEBOUNCE_S`] on purpose: a pan or zoom re-cuts the engine's
/// viewport, and fitting to the frame that was in flight while the drag was
/// still moving would measure the band the operator has just left. The gap
/// beyond the debounce is what the engine gets to deliver a frame of the new
/// window in.
const FIT_SETTLE_S: f64 = 0.75;

/// How often the levels are measured, and how often a glide steps.
///
/// Longer than [`CFG_DEBOUNCE_S`], and that is the whole reason for the number:
/// the levels only take effect once the *engine* has them — the waterfall is
/// painted from bins it has already mapped — and a value that moved every frame
/// would reset the debounce every frame and never be sent at all. Stepping at
/// this cadence leaves the debounce a gap to fire in, so each step of the glide
/// reaches the screen.
const FIT_STEP_S: f64 = 0.5;

/// What fraction of the distance still to go one step of a glide covers.
/// A quarter each, ten steps to the [`FIT_MIN_GAP_S`]: 94% of the way there
/// over five seconds, with no step large enough to read as a jump.
const FIT_STEP_FRAC: f32 = 0.25;

/// Close enough to the target to stop gliding and sit on it, in dB — below the
/// ~0.4 dB the frame's u8 bins are quantised to anyway.
const FIT_ARRIVED_DB: f32 = 0.2;

/// Weight of one measurement in the rolling average a glide aims at. A
/// twentieth each, at [`FIT_STEP_S`] apart: a burst of QRM or a station coming
/// up for a few seconds moves the target by a fraction of a dB, while a band
/// that has genuinely changed carries it over the following half-minute.
const FIT_AVG_ALPHA: f32 = 0.05;

/// How far the levels may drift from that average before auto-fit steps in, in
/// dB. Well above frame-to-frame percentile jitter and the ~0.4 dB quantisation
/// of the frame's u8 bins, so a steady band is fitted once and then left alone.
const FIT_DRIFT_DB: f32 = 3.0;

/// FT8/FT4 callsign boxes stop being drawn once the newest decode is this old,
/// so a stalled decoder (dead band, band change) doesn't leave labels pinned to
/// the waterfall for good.
const FT8_LABEL_MAX_AGE_SECS: i64 = 45;

/// Stable per-callsign id for the FT8 overlay boxes (keeps a station's box in
/// place across slots).
fn hash_call(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Pick `(floor, ceil)` dB for best waterfall contrast from a frame's u8
/// `bins` (mapped over `[db_floor, db_ceil]`). Percentile-based so a single
/// strong carrier doesn't over-blow the scale and weak signals stay visible.
/// Returns `None` for an empty or degenerate frame.
fn pick_levels(bins: &[u8], db_floor: f32, db_ceil: f32) -> Option<(f32, f32)> {
    let range = db_ceil - db_floor;
    if bins.is_empty() || range <= 0.0 {
        return None;
    }
    // Reconstruct approximate dB per bin from the u8 mapping and sort.
    let mut db: Vec<f32> = bins.iter().map(|&b| db_floor + (b as f32 / 255.0) * range).collect();
    db.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: f32| -> f32 {
        let i = ((p * (db.len() - 1) as f32).round() as usize).min(db.len() - 1);
        db[i]
    };
    let noise = pct(0.25); // typical noise floor
    let peak = pct(0.99); // strong signals, ignoring the hottest outliers
    // A bin outside the frame's own mapping is clamped to its edge, so all it
    // says is "beyond this" — the level it stands for is not in the frame at
    // all. Reach further in that direction rather than the usual few dB, or a
    // display that has gone flat black (or solid white) would crawl back a
    // margin at a time. One step may overshoot; the next fit reels it in.
    let step = range / 255.0;
    let reach = |clipped: bool, margin: f32| if clipped { 15.0 } else { margin };
    let mut floor = noise - reach(noise <= db_floor + step, 5.0); // noise just above the floor (dark)
    let mut ceil = peak + reach(peak >= db_ceil - step, 6.0); // headroom so strong signals don't clip
    // Keep a usable dynamic range even on an empty/flat band.
    let min_range = 24.0;
    if ceil - floor < min_range {
        let mid = 0.5 * (ceil + floor);
        floor = mid - 0.5 * min_range;
        ceil = mid + 0.5 * min_range;
    }
    // Clamp to the same bounds as the manual controls.
    let floor = floor.clamp(-160.0, -40.0);
    let mut ceil = ceil.clamp(-100.0, 20.0);
    if ceil - floor < 10.0 {
        ceil = (floor + 10.0).min(20.0);
    }
    Some((floor, ceil))
}

/// Auto-fit bookkeeping (see [`SdroxideApp::auto_fit_tick`]).
#[derive(Default)]
pub(in crate::app) struct AutoFit {
    /// Rolling average of the levels a fit would pick, and when it was last
    /// added to. What a glide aims at — an average rather than the latest
    /// measurement so that a signal up for a second or two moves the waterfall's
    /// contrast hardly at all. `None` until the window it describes has been
    /// measured once.
    avg: Option<(f32, f32)>,
    stepped_at: f64,
    /// When the glide now running started, and whether one is running. Also
    /// what [`FIT_MIN_GAP_S`] spaces out — nothing dates from the *end* of a
    /// fit, so a glide that is overtaken by events still counts as one.
    started_at: Option<f64>,
    gliding: bool,
    /// What the last tick was looking at, and when that last changed. Any change
    /// is both a reason to refit and the start of the settle wait.
    key: Option<FitKey>,
    changed_at: f64,
    /// A refit asked for by a change above, still waiting for the interval to
    /// elapse or the view to settle. Kept rather than dropped, so a band change
    /// that lands inside the interval is fitted late instead of never.
    pending: bool,
}

/// What an automatic fit is a fit *of*: the visible window, the two numbers the
/// resolution is made of — the transform size and the columns it is pooled into
/// — the width the front end delivers, and the band the dial sits in.
///
/// The column count belongs here for the same reason the FFT size does: pooling
/// into more columns takes the maximum of fewer bins apiece, so every level in
/// the picture moves and the contrast the last fit chose is no longer the right
/// one. The band is carried separately because a wideband front end can be
/// retuned clear across a band edge without the visible span moving.
///
/// Deliberately *not* the tuned frequency: on a front end whose centre follows
/// the dial, every nudge of the knob would then be a refit, and tuning across a
/// band would keep re-contrasting the waterfall underneath the operator. What
/// such a retune really changes is the levels, and drift already answers that.
type FitKey = (f64, f64, u32, u32, f64, sdroxide_types::Band);

/// Whether a new automatic fit may start: the interval since the last one began
/// has elapsed, and the view has held still long enough that the frames in hand
/// are of the window being fitted rather than the one being left.
///
/// Deliberately says nothing about *why* a fit is wanted — a queued trigger and
/// drifted levels both wait on exactly these two.
fn fit_due(fit: &AutoFit, now: f64) -> bool {
    fit.started_at.is_none_or(|t| now - t >= FIT_MIN_GAP_S) && now - fit.changed_at >= FIT_SETTLE_S
}

/// Whether `have` is far enough from `want` to be worth a fit — i.e. the
/// spectrum now sits too high or too low in the window it is drawn over.
fn levels_drifted(want: (f32, f32), have: (f32, f32)) -> bool {
    (want.0 - have.0).abs() > FIT_DRIFT_DB || (want.1 - have.1).abs() > FIT_DRIFT_DB
}

/// One step of the glide from `have` towards `target`, or `None` once there is
/// nothing left worth covering.
fn glide_step(have: (f32, f32), target: (f32, f32)) -> Option<(f32, f32)> {
    let arrived = |a: f32, b: f32| (a - b).abs() <= FIT_ARRIVED_DB;
    if arrived(have.0, target.0) && arrived(have.1, target.1) {
        return None;
    }
    let step = |from: f32, to: f32| from + (to - from) * FIT_STEP_FRAC;
    Some((step(have.0, target.0), step(have.1, target.1)))
}

/// Fold a fresh measurement into the rolling average, or start one.
fn average_in(avg: Option<(f32, f32)>, want: (f32, f32)) -> (f32, f32) {
    // The first measurement of a window *is* the average: there is nothing yet
    // to smooth it against, and a band change would otherwise be dragged
    // towards the levels of the band that was left.
    let Some(avg) = avg else { return want };
    let mix = |a: f32, w: f32| a + (w - a) * FIT_AVG_ALPHA;
    (mix(avg.0, want.0), mix(avg.1, want.1))
}

/// The window to ask the engine for: the visible span with 2× slack, centred on
/// the view and kept inside the device window, so panning inside it needs no
/// reconfiguration.
///
/// The upper bound on where it may start is floored at `dev_lo` because the two
/// are not the same expression: once the slack covers the whole device window,
/// `dev_hi - slack` *is* `dev_lo` in exact arithmetic, but `dev_lo` is
/// `centre − span/2` while this is `(centre + span/2) − span`, and the two round
/// differently. A few nanohertz of disagreement is nothing to a viewport and
/// everything to [`f64::clamp`], which panics when its min exceeds its max —
/// which is how scrolling an RX-888 to the top of its band crashed the app.
fn slack_viewport(center_hz: f64, full_span: f64, (view_lo, view_hi): (f64, f64)) -> (f64, f64) {
    let dev_lo = center_hz - full_span / 2.0;
    let dev_hi = center_hz + full_span / 2.0;
    let slack = ((view_hi - view_lo) * 2.0).min(full_span);
    let center = (view_lo + view_hi) / 2.0;
    let lo = (center - slack / 2.0).clamp(dev_lo, (dev_hi - slack).max(dev_lo));
    (lo, lo + slack)
}

/// How far either side of a spot's frequency the S/N filter looks: a
/// broadcast carrier's sidebands and a dial a little off the listed frequency.
const SNR_LABEL_REACH_HZ: f64 = 2_000.0;
/// The S/N a spot needs for its label is the S/N rail's
/// ([`crate::view::ViewState::snr_label_min_db`]); a shown label goes only
/// under that less this margin, and between the two it stays as it was. Half
/// the threshold under 3 dB, so a threshold of 0 dB has no margin below zero.
// Why: HDZ 2026-11-07: "S/N mit 6 ist viel zu streng, Shannon hat hier 2 …
//      du kannst unter SPEC einen Schieberegler machen für S/N, Default 2,
//      Range 0..6".
fn snr_label_off_db(on: f32) -> f32 {
    on - (on / 2.0).clamp(0.0, 1.5)
}
/// How often the labels are decided anew, seconds.
const SNR_LABEL_RECHECK_S: f32 = 2.0;
/// How long a label takes to fade in or out, seconds.
const SNR_LABEL_FADE_S: f32 = 1.0;
/// Frames a cell's mean needs before the filter judges it: at the guide's
/// frame rate a few seconds of band.
const SNR_LABEL_MIN_FRAMES: u32 = 60;

/// The highest S/N `g` has within [`SNR_LABEL_REACH_HZ`] of `hz`, or `None`
/// where it cannot say yet: no cell there, or the cells there have fewer than
/// [`SNR_LABEL_MIN_FRAMES`] frames in their mean. A cell not reached counts as
/// no signal once its neighbours are judged.
fn guide_max(g: &crate::mean_spectrum::Guide, hz: f64) -> Option<f32> {
    guide_peak(g, hz - SNR_LABEL_REACH_HZ, hz + SNR_LABEL_REACH_HZ).map(|(v, _)| v)
}

/// The strongest cell of `g` between `lo_hz` and `hi_hz`: its S/N and the
/// frequency of its middle. `None` as for [`guide_max`].
fn guide_peak(g: &crate::mean_spectrum::Guide, lo_hz: f64, hi_hz: f64) -> Option<(f32, f64)> {
    if g.cell_hz <= 0.0 || g.snr_db.is_empty() {
        return None;
    }
    let first = ((lo_hz - g.lo_hz) / g.cell_hz).floor().max(0.0) as usize;
    let last = (((hi_hz - g.lo_hz) / g.cell_hz).ceil().max(0.0) as usize).min(g.snr_db.len() - 1);
    if first > last {
        return None;
    }
    let frames = g.count.get(first..=last)?.iter().copied().max().unwrap_or(0);
    if frames < SNR_LABEL_MIN_FRAMES {
        return None;
    }
    let (i, v) = g.snr_db[first..=last]
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .fold((0, f32::NEG_INFINITY), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    Some((v, g.lo_hz + (first + i) as f64 * g.cell_hz + g.cell_hz / 2.0))
}

/// PEAK: how far a label's S/N must stand over its surroundings, dB.
const PEAK_PROMINENCE_DB: f32 = 3.0;
/// The ring a label's peak is measured against, Hz from its frequency: past a
/// carrier's own cells, inside its sidebands.
// Why: HDZ 2026-11-07, picture 7.305 MHz: catalogue guesses on the sideband of
//      the strong carrier 5 kHz away — a modulated sideband stands S/N MIN over
//      the noise as a broad hump; a station of its own stands out of its
//      surroundings as a peak. 2026-11-11 the same for interference.
const PEAK_RING_HZ: (f64, f64) = (600.0, 2_000.0);

/// Whether a label at `hz` has a peak of its own: its S/N within
/// [`SNR_LABEL_REACH_HZ`] at least [`PEAK_PROMINENCE_DB`] over the median of
/// the cells [`PEAK_RING_HZ`] from it. A broad hump — interference over part of
/// the band, a strong station's sideband — lifts the ring with it and so has
/// none. `None` where the guide cannot tell, as [`guide_max`].
fn has_a_peak(g: &crate::mean_spectrum::Guide, hz: f64) -> Option<bool> {
    let top = guide_max(g, hz)?;
    let (near, far) = PEAK_RING_HZ;
    let mut ring: Vec<f32> = g
        .snr_db
        .iter()
        .enumerate()
        .filter(|(i, v)| {
            let d = (g.lo_hz + (*i as f64 + 0.5) * g.cell_hz - hz).abs();
            v.is_finite() && (near..=far).contains(&d)
        })
        .map(|(_, v)| *v)
        .collect();
    if ring.is_empty() {
        return None;
    }
    ring.sort_by(f32::total_cmp);
    Some(top - ring[ring.len() / 2] >= PEAK_PROMINENCE_DB)
}

/// How far either side of a strong station FLANKS keeps other labels away: its
/// sidebands and splatter, about a cursor's width in all.
// Why: HDZ 2026-11-11: "es werden noch zu viele Sender insbesondere in den
//      Seitenbändern erlaubt … wenn ein Sender einen großen Wert (say >10 S/N)
//      oder 15 hat, die Umgebung in typischer Cursorbreite, vielleicht 20 kHz,
//      blacklisten … einen Regler wie 'keine Labels an Sendergrenzen'".
const SNR_FLANK_REACH_HZ: f64 = 10_000.0;
/// How far under its strong neighbour a label's own S/N must stay for it to
/// count as that neighbour's flank rather than a station of its own.
const SNR_FLANK_MARGIN_DB: f32 = 6.0;

/// Whether a label at `hz` stands on a strong station's flank: within
/// [`SNR_FLANK_REACH_HZ`] a cell reaches `flank_db`, it lies outside the label's
/// own [`SNR_LABEL_REACH_HZ`], and the label's own S/N there is
/// [`SNR_FLANK_MARGIN_DB`] or more under it. The strong station keeps its own
/// label; `flank_db` at [`crate::view::SNR_FLANK_OFF`] or below blocks nothing.
fn on_a_flank(g: &crate::mean_spectrum::Guide, hz: f64, flank_db: f32) -> bool {
    if flank_db <= crate::view::SNR_FLANK_OFF {
        return false;
    }
    let (Some(own), Some((strong, at))) =
        (guide_max(g, hz), guide_peak(g, hz - SNR_FLANK_REACH_HZ, hz + SNR_FLANK_REACH_HZ))
    else {
        return false;
    };
    strong >= flank_db
        && (at - hz).abs() > SNR_LABEL_REACH_HZ
        && own <= strong - SNR_FLANK_MARGIN_DB
}

/// LABELS BY S/N: per spot id, whether its label is wanted and its present
/// opacity; when the last decision and the last frame were.
#[derive(Default)]
pub(in crate::app) struct SnrLabels {
    shown: std::collections::HashMap<u64, (bool, f32)>,
    checked: Option<std::time::Instant>,
    last: Option<std::time::Instant>,
}

impl SdroxideApp {
    /// The full-band window this front end publishes beside its I/Q, `(centre,
    /// span)` — what the main panadapter may be zoomed out into.
    ///
    /// `None` where there is no such lane, or where it has stopped arriving:
    /// a window built on a spectrum that is no longer being sent would let the
    /// operator zoom out into a frozen picture. The engine applies the same
    /// staleness rule on its side before it will draw one.
    pub(in crate::app) fn wide_window(&self) -> Option<(f64, f64)> {
        let f = self.wide_frame.as_ref()?;
        (f.span_hz > 0.0).then_some((f.center_hz, f.span_hz))
    }

    /// The widest window the view may cover: the full-band lane where there is
    /// one and it beats the passband, else the passband itself.
    ///
    /// The *width* comes from the capabilities rather than from the last frame,
    /// so it is known from the moment the front end opens. Waiting for a
    /// picture would spend the first frames of every session believing the
    /// passband was the limit — long enough to shrink a restored window to it
    /// and lose the operator's zoom on every restart.
    ///
    /// Where it sits still comes from the frame, because that moves with the
    /// receiver; until one arrives the passband's centre is the best guess and
    /// is right whenever the lane is centred on the receiver, which is the
    /// usual case.
    pub(in crate::app) fn zoom_out_window(&self) -> (f64, f64) {
        let dev_span = self.state.sample_rate;
        let stated = self.caps.as_ref().map_or(0.0, |c| c.wide_span_hz);
        let span = match self.wide_window() {
            Some((_, seen)) => seen.max(stated),
            None => stated,
        };
        if span <= dev_span {
            return (self.state.center_hz, dev_span);
        }
        let center = self.wide_window().map_or(self.state.center_hz, |(c, _)| c);
        (center, span)
    }

    /// Desired engine-side spectrum config. The requested viewport gets 2×
    /// slack around the visible span so panning inside it needs no
    /// reconfiguration (which would clear the waterfall history); the FFT
    /// grows with zoom for real resolution.
    /// The panadapter width this client wants: the operator's setting, or what
    /// `Auto` makes of this machine and this screen.
    ///
    /// Two things have to agree on this number — the config sent to the engine
    /// ([`Self::desired_spectrum_cfg`]) and the history texture the waterfall
    /// draws into (`WfTuning::tex_w`, filled in [`Self::wf_tick`]) — and they
    /// agree by both calling here. Cheap enough per frame: a handful of
    /// comparisons over numbers gathered once when the window opened.
    ///
    /// A manual choice is still held to what the renderer can hold, so a
    /// `config.toml` carried from a desktop to a Raspberry Pi does not ask the
    /// Pi for a texture it cannot make. The stored preference is left alone —
    /// carrying it back gets the detail back.
    /// Record how wide the panadapter is about to be drawn, in device pixels.
    ///
    /// Taken from the `Ui` rather than from the window, so a split view gives
    /// each pane its own answer — two radios side by side are each half the
    /// screen and should each ask for what they can show. Multiplied by the
    /// zoom factor because egui measures in points and a texture is measured in
    /// pixels: a 4K laptop at 2× scaling reports about 1900 points for a
    /// 3840-pixel panadapter, and believing it would leave the operator with
    /// exactly the picture issue #172 is about.
    pub(in crate::app) fn note_panadapter_width(&mut self, ui: &eframe::egui::Ui) {
        let px = (ui.available_width() * ui.ctx().pixels_per_point()).max(0.0) as u32;
        self.panadapter_px = self.panadapter_px.max(px);
    }

    pub(in crate::app) fn panadapter_bins(&self) -> u32 {
        self.bins_for_detail(self.ui_settings.spectrum_detail)
    }

    /// [`Self::panadapter_bins`] for a setting that is not (yet) the one in
    /// force — what the settings window reads back while the operator is still
    /// choosing.
    pub(in crate::app) fn bins_for_detail(&self, detail: sdroxide_types::SpectrumDetail) -> u32 {
        let Some(class) = self.display_class else {
            return waterfall_gpu::DEFAULT_TEX_W;
        };
        match detail.columns() {
            Some(want) => {
                want.clamp(waterfall_gpu::DEFAULT_TEX_W, waterfall_gpu::manual_ceiling(class))
            }
            None => waterfall_gpu::auto_display_bins(class, self.panadapter_px),
        }
    }

    /// What the detail row of the SPEC popup shows beside its chips: the width
    /// in force, the widest that may be picked, and one sentence naming
    /// whatever stopped it going higher.
    ///
    /// The sentence is the point. A greyed chip with no explanation is a bug
    /// report; a greyed chip that says *why* is an answer.
    pub(in crate::app) fn detail_report(
        &self,
        detail: sdroxide_types::SpectrumDetail,
    ) -> DetailReport {
        let ceiling =
            self.display_class.map_or(waterfall_gpu::DEFAULT_TEX_W, waterfall_gpu::manual_ceiling);
        let reason = match self.display_class {
            None => tr!("spectrum-window-has-no-gpu-renderer").to_string(),
            Some(c) if c.device_type == crate::egui_wgpu::wgpu::DeviceType::Cpu => {
                tr!("spectrum-machine-drawing-without-gpu-every").to_string()
            }
            Some(c) if c.backend == crate::egui_wgpu::wgpu::Backend::Gl => {
                tr!("spectrum-window-renders-through-opengl-which").to_string()
            }
            Some(c) if c.max_texture_dim < waterfall_gpu::MAX_TEX_W => {
                tr!("spectrum-renderer-will-not-hold-texture", max_texture_dim = c.max_texture_dim)
            }
            Some(_) => tr!("spectrum-wider-than-renderer-can-draw").to_string(),
        };
        DetailReport { chosen: self.bins_for_detail(detail), ceiling, reason }
    }

    /// Whether the panadapter shows a hole: see [`view_shows_a_hole`].
    pub(in crate::app) fn view_uncovered(&self) -> bool {
        !self.view.is_unset()
            && view_shows_a_hole(
                (self.view.view_lo_hz, self.view.view_hi_hz),
                self.state.center_hz,
                self.state.sample_rate,
            )
    }

    pub(in crate::app) fn desired_spectrum_cfg(&self) -> SpectrumConfig {
        let full_span = self.state.sample_rate;
        // The window the *view* lives in, which past the passband is the
        // full-band lane's — slack clamped to the I/Q there would ask the
        // engine for a window the operator is not looking at, and it is the
        // wide bins that answer once the view leaves the passband.
        let (out_center, out_span) = self.zoom_out_window();
        let (viewport, zoom) = if !self.view.is_unset() && full_span > 0.0 {
            let ratio = (full_span / self.view.span()).max(1.0);
            // Zoomed out past the passband: the whole view is the request, and
            // there is no zoom factor to grow the transform by — the wide lane
            // has whatever resolution it has.
            if self.view.span() > full_span * WIDER_THAN_PASSBAND {
                (
                    Some(slack_viewport(
                        out_center,
                        out_span,
                        (self.view.view_lo_hz, self.view.view_hi_hz),
                    )),
                    1.0,
                )
            } else if ratio > 1.05 {
                let vp = slack_viewport(
                    self.state.center_hz,
                    full_span,
                    (self.view.view_lo_hz, self.view.view_hi_hz),
                );
                (Some(vp), ratio)
            } else {
                (None, 1.0)
            }
        } else {
            (None, 1.0)
        };
        // The operator's chip, capped at what this front end's *rate* can
        // afford. A transform covers `fft_size / rate` seconds of signal, and
        // that time is the panadapter's update period and its smear both — so
        // the same 32768 that is free on a 2 Msps SDR (16 ms) is 1.4 seconds
        // on the Icom LAN backend's 24 kHz IF, which is one new picture every
        // 1.4 s, each of them an average of the whole of it. It buys nothing
        // to pay for: a frame carries `DISPLAY_BINS` bins, so of 32768 across
        // 24 kHz, sixteen are max-pooled into every one drawn.
        //
        // Only the *base* is capped. Zoom still multiplies it below, because
        // resolution finer than the span is exactly what somebody zoomed in is
        // asking for, and the seconds it costs are then a price they chose.
        let base = base_fft_for_rate(self.view.fft_size, full_span);
        // …and never past the point where the engine's own zoom lane would
        // switch off. Growing this analyser is a transform over everything the
        // front end streams; the lane resolves the same window off a decimated
        // copy for a fraction of it, and asking for more than this stops it
        // being built at all — which is what made zooming in on a 2 Msps
        // HackRF start dropping samples (issue #195).
        let ceiling = viewport
            .and_then(|(lo, hi)| {
                sdroxide_types::panadapter_fft_ceiling(full_span, hi - lo, self.panadapter_bins())
            })
            .unwrap_or(MAX_FFT);
        let mut fft = base;
        while (fft as f64) < base as f64 * zoom.min(8.0) && fft < MAX_FFT && fft * 2 <= ceiling {
            fft *= 2;
        }
        SpectrumConfig {
            fft_size: fft,
            // The same number the waterfall texture is built at — see
            // `panadapter_bins`. The engine holds it to its own ceiling and to
            // what its FFT can actually fill, so the frame that comes back may
            // be narrower; the texture upload resamples where it is.
            display_bins: self.panadapter_bins(),
            // The waterfall's own clock. Not `fps`: the engine appends lines to
            // a texture, which is cheap, where a frame is a repaint, which is
            // not — see `SpectrumConfig::rows_per_sec`. Scaled by the display's
            // zoom factor for the same reason the rows on screen are, so a line
            // is one device pixel tall on a HiDPI panel too.
            rows_per_sec: (self.ui_settings.waterfall_rows_per_sec() * self.wf_row_scale)
                .round()
                .clamp(1.0, f32::from(sdroxide_types::MAX_ROWS_PER_SEC))
                as u16,
            db_floor: self.view.db_floor,
            db_ceil: self.view.db_ceil,
            viewport,
            // Frame rate comes from the UI settings and also drives the repaint
            // cadence (see the end of `ui`). Engine averaging is disabled so the
            // waterfall gets full detail; the spectrum *line* is smoothed UI-side
            // per the spectrum-speed setting (decoupled from the waterfall).
            fps: self.ui_settings.fps().min(255) as u8,
            avg_tc: 0.0,
        }
    }

    /// Advance the waterfall time-scroll one frame: convert the wall-clock
    /// elapsed since the last tick into a whole number of rows to append (at the
    /// configured rows/second), carrying the fraction. Returns the tuning the
    /// widget needs; the same rows/second also spaces the time gridlines, so the
    /// line and the waterfall move together. `live` gates scrolling — false
    /// once the stream has stalled (a radio switched off, a device gone quiet),
    /// so the last frame is not duplicated down the screen as time that never
    /// happened. The callers judge staleness; this only stops the rows.
    ///
    /// `row_scale` is the display's pixels per egui point. The waterfall stores
    /// one history row per point by default, which on a HiDPI panel means every
    /// row is drawn two pixels tall — the vertical half of the coarse picture
    /// issue #172 is about. Scaling *both* the row rate and the rows on screen
    /// by it puts one row on one pixel and changes nothing else: the seconds in
    /// view, the scroll speed and the gridline spacing are all ratios of the
    /// two and come out identical. What it costs is scrollback, since the ring
    /// is a fixed number of rows — 18 s instead of 36 at the medium rate on a
    /// 2× display.
    pub(in crate::app) fn wf_tick(
        &mut self,
        live: bool,
        row_scale: f32,
    ) -> spectrum_view::WfTuning {
        let now = now_unix_f64();
        self.wf_row_scale = row_scale.max(1.0);
        let rows_per_sec = self.ui_settings.waterfall_rows_per_sec() * self.wf_row_scale;
        // Clamp dt so a hitch/tab-away can't dump a huge run of rows at once.
        let dt =
            if self.wf_last_now > 0.0 { (now - self.wf_last_now).clamp(0.0, 0.3) } else { 0.0 };
        self.wf_last_now = now;
        let rows_to_write = if live {
            self.wf_row_accum += dt as f32 * rows_per_sec;
            let n = self.wf_row_accum.floor();
            self.wf_row_accum -= n;
            (n as u32).min(32)
        } else {
            0
        };
        // The time axis belongs to the rows. While they scroll, the newest row
        // is "now" and the gridlines ride the wall clock; frozen, the clock is
        // pinned where the rows stopped, or the timestamps would slide over
        // history that is not moving.
        if live || self.wf_now_pin == 0.0 {
            self.wf_now_pin = now;
        }
        // Spectrum-line smoothing: convert the time constant to a per-frame EMA
        // coefficient using the frame rate, so the reaction time is the same at
        // any fps (0 tc = no smoothing = raw frames).
        let tc = self.ui_settings.spectrum_avg_tc();
        let fps = self.ui_settings.fps().max(1) as f32;
        let spectrum_alpha = if tc <= 0.0 { 1.0 } else { 1.0 - (-(1.0 / fps) / tc).exp() };
        let s = &self.ui_settings;
        let gradient = s.spectrum_gradient.then(|| {
            let [tr, tg, tb] = s.gradient_top;
            let [br, bg, bb] = s.gradient_bottom;
            (Color32::from_rgb(tr, tg, tb), Color32::from_rgb(br, bg, bb))
        });
        spectrum_view::WfTuning {
            rows_to_write,
            // The operator's own rate, unscaled: the gridlines are spaced in
            // points and derive the rest from `row_scale` themselves.
            rows_per_sec: self.ui_settings.waterfall_rows_per_sec(),
            // Gated on `live` for the reason the waterfall's rows are: a
            // stalled stream must freeze the picture, not fill it with copies
            // of the last spectrum it managed to send.
            surface_rows_per_sec: if live {
                self.ui_settings.spectrum_3d_rows_per_sec()
            } else {
                0.0
            },
            row_scale: row_scale.max(1.0),
            tex_w: self.panadapter_bins(),
            now_unix: self.wf_now_pin,
            spectrum_alpha,
            palette: s.waterfall_palette,
            smooth: s.waterfall_smooth,
            gradient,
            wf_id: u64::from(self.radio_id),
        }
    }

    /// Hysteresis: is the config the engine already has still fine for the
    /// current view? (Avoids waterfall-clearing resends while panning.)
    pub(in crate::app) fn cfg_still_good(&self) -> bool {
        let Some(sent) = self.sent_cfg else { return false };
        let ideal = self.desired_spectrum_cfg();
        if sent.fft_size != ideal.fft_size
            || sent.display_bins != ideal.display_bins
            || sent.rows_per_sec != ideal.rows_per_sec
            || sent.db_floor != ideal.db_floor
            || sent.db_ceil != ideal.db_ceil
            || sent.fps != ideal.fps
            || sent.avg_tc != ideal.avg_tc
        {
            return false;
        }
        match (sent.viewport, ideal.viewport) {
            (None, None) => true,
            (Some((slo, shi)), Some(_)) => {
                let full_span = self.state.sample_rate;
                let dev_lo = self.state.center_hz - full_span / 2.0;
                let dev_hi = self.state.center_hz + full_span / 2.0;
                let sspan = shi - slo;
                let margin = sspan * 0.05;
                // Inside with margin, unless the sent window is pinned to a
                // device edge on that side.
                let lo_ok = self.view.view_lo_hz >= slo + margin || slo <= dev_lo + 1.0;
                let hi_ok = self.view.view_hi_hz <= shi - margin || shi >= dev_hi - 1.0;
                let res = sspan / self.view.span().max(1.0);
                lo_ok && hi_ok && (1.15..=3.5).contains(&res)
            }
            _ => false,
        }
    }

    /// The CW-skimmer overlay: the current spots plus a parallel per-spot
    /// opacity that fades a box out over `SKIMMER_FADE_SECS` once it stops
    /// keying. Fully-faded spots are dropped so they free their lane.
    pub(in crate::app) fn cw_overlay(&self, now: f64) -> (Vec<SkimmerSpot>, Vec<f32>) {
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        for s in &self.skimmer_spots {
            let a = if s.active {
                1.0
            } else {
                let last = self.skimmer_active_at.get(&s.id).copied().unwrap_or(now);
                (1.0 - (now - last) / SKIMMER_FADE_SECS).clamp(0.0, 1.0) as f32
            };
            if a <= 0.02 {
                continue;
            }
            spots.push(s.clone());
            alpha.push(a);
        }
        (spots, alpha)
    }

    /// Reuse the skimmer overlay to mark FT8/FT4 stations: one box per decoded
    /// callsign at its audio frequency (`dial + audio_hz`). The newest slot is
    /// solid; the previous slot is dimmed. Clicking a box sets the audio offset.
    ///
    /// Nothing at all with the labels switched off in the SPEC popup: the
    /// decode list beside the waterfall says the same callsigns, so an operator
    /// who finds a busy band unreadable through thirty boxes loses nothing by
    /// clearing them (issue #248).
    pub(in crate::app) fn ft8_overlay(&self) -> (Vec<SkimmerSpot>, Vec<f32>) {
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        if !self.view.decode_labels {
            return (spots, alpha);
        }
        let Some(latest) = self.digi_decodes.first().map(|d| d.slot_utc) else {
            return (spots, alpha);
        };
        // Age the whole overlay against the wall clock, not just against its own
        // newest entry: once decoding stops the boxes expire instead of staying
        // on the waterfall indefinitely.
        if now_unix() - latest > FT8_LABEL_MAX_AGE_SECS {
            return (spots, alpha);
        }
        let dial = self.state.rx_freq_hz();
        let mut seen = std::collections::HashSet::new();
        for d in &self.digi_decodes {
            // Decodes are newest-first; show only the last couple of slots.
            if latest - d.slot_utc > 30 {
                break;
            }
            let Some(call) = &d.from else { continue };
            if !seen.insert(call.clone()) {
                continue; // keep the most recent decode per callsign
            }
            let newest = d.slot_utc == latest;
            spots.push(SkimmerSpot {
                id: hash_call(call),
                kind: SkimmerKind::Cw,
                freq_hz: dial + d.audio_hz as f64,
                callsign: Some(call.clone()),
                text: d.message.clone(),
                snr_db: d.snr_db,
                wpm: 0,
                active: newest,
            });
            alpha.push(if newest { 1.0 } else { 0.5 });
        }
        (spots, alpha)
    }

    /// The network-spot overlay: the currently-shown spots (filtered by kind and,
    /// optionally, to the panadapter view span) plus a parallel age-fade alpha.
    /// Newest spots are solid; they dim over the last quarter of their lifetime.
    ///
    /// Runs every frame, so it clones only what survives the filters rather than
    /// building a merged list first — the layout pass sorts by screen position
    /// itself, so the output need not be in frequency order.
    /// The ISM devices to label on the waterfall.
    ///
    /// Every device currently in the table, formatted the same way the ISM
    /// window's rows are so the two agree. There is deliberately no fade and no
    /// age cut: these transmitters sleep for a minute at a time between frames,
    /// so anything that faded on silence would spend most of its life invisible,
    /// and a label saying where a meter is stays true while it is asleep.
    pub(in crate::app) fn ism_overlay(&self) -> Vec<crate::widgets::spectrum_view::IsmLabel> {
        self.ism_reports
            .iter()
            .map(|r| crate::widgets::spectrum_view::IsmLabel {
                freq_hz: r.freq_hz,
                // The id gets a `#` in front of it. The window can put it in a
                // column of its own; here it is run together with the model
                // name, and a model ending in a digit — "7-in-1" — reads as one
                // number with the id when only a space separates them.
                text: format!("{} #{}  {}", r.fmt_kind(), r.device, r.fmt_readings()),
                encrypted: r.encrypted,
            })
            .collect()
    }

    /// The stored memories to mark along the bottom of the waterfall.
    ///
    /// Only the ones on the visible span are formatted. A station list runs to
    /// hundreds of channels and this is rebuilt every frame, so the span the
    /// last frame was drawn with does the filtering — a pan moves the picture
    /// and its marks together on the frame after, which is one frame and
    /// invisible.
    pub(in crate::app) fn memory_overlay(&self) -> Vec<crate::widgets::memories::MemMark> {
        let (lo, hi) = (self.view.view_lo_hz, self.view.view_hi_hz);
        self.memories
            .iter()
            .filter(|m| (lo..=hi).contains(&m.freq_hz))
            .map(|m| {
                // A memory whose folder has gone from under it reads as
                // unfiled, exactly as the memory window lists it.
                let folder = m
                    .folder
                    .and_then(|id| self.mem_folders.iter().find(|f| f.id == id))
                    .map(|f| f.name.as_str());
                crate::widgets::memories::MemMark {
                    id: m.id,
                    freq_hz: m.freq_hz,
                    text: match folder {
                        Some(f) => tr!("spectrum-mem", f = f, value = m.name),
                        None => tr!("spectrum-mem-2", value = m.name),
                    },
                }
            })
            .collect()
    }

    /// Whether a spot at `hz` gets its label under the S/N filter: always
    /// while the filter or the guide is off; otherwise only where the guide
    /// has a cell within [`SNR_LABEL_REACH_HZ`] of it at least the S/N rail's
    /// threshold over the noise floor. A cell the guide has not reached counts
    /// as nothing there.
    /// The label's opacity under LABELS BY S/N, 0 to 1; 1 while the filter or
    /// the guide is off. Decided every [`SNR_LABEL_RECHECK_S`] rather than every
    /// frame, with hysteresis (on at the rail's threshold, off under
    /// [`snr_label_off_db`] of it), and faded over [`SNR_LABEL_FADE_S`], so labels
    /// come and go like the world in a game drawn as it is reached instead of
    /// flickering at the threshold. Where the guide has fewer than
    /// [`SNR_LABEL_MIN_FRAMES`] frames the label stays: too early to judge.
    // Why: HDZ 2026-11-07: "das kann dynamisch ab und an nachfragen, der Toggle
    //      sollte warten, bis S/N genug Spektren akkumuliert hat, um nicht
    //      durchzudrehen … wie bei einem MMO, wo die Welt dynamisch gezeichnet
    //      würde".
    /// Whether the S/N guide sees a signal at `s`: at least the S/N rail's
    /// threshold within [`SNR_LABEL_REACH_HZ`] of its frequency. `false` where
    /// the guide cannot tell (off, outside the panadapter, too few frames): the
    /// S/N chip in SPOTS keeps what is shown to be there, not what might be.
    pub(in crate::app) fn snr_matches(&self, s: &sdroxide_types::Spot) -> bool {
        let Some(g) = self.snr_last.as_deref().filter(|_| self.view.snr_guide) else {
            return false;
        };
        guide_max(g, s.freq_hz).is_some_and(|v| v >= self.view.snr_label_min_db)
    }

    fn snr_label_alpha(&self, id: u64, hz: f64, recheck: bool, step: f32) -> f32 {
        let Some(g) =
            self.snr_last.as_deref().filter(|_| self.view.snr_guide && self.view.snr_label_filter)
        else {
            return 1.0;
        };
        let on = self.view.snr_label_min_db;
        let off = snr_label_off_db(on);
        let mut st = self.snr_labels.borrow_mut();
        let entry = st.shown.entry(id).or_insert((true, 0.0));
        if recheck {
            entry.0 = if on_a_flank(g, hz, self.view.snr_label_flank_db)
                || (self.view.snr_label_peak && has_a_peak(g, hz) == Some(false))
            {
                false
            } else {
                match guide_max(g, hz) {
                    None => true,
                    Some(v) if v >= on => true,
                    Some(v) if v < off => false,
                    Some(_) => entry.0,
                }
            };
        }
        let target = if entry.0 { 1.0 } else { 0.0 };
        let d = step / SNR_LABEL_FADE_S;
        entry.1 =
            if entry.1 < target { (entry.1 + d).min(target) } else { (entry.1 - d).max(target) };
        entry.1
    }

    /// Once a frame before the labels are decided: the time since the last
    /// frame, for the fades, and whether a new decision is due. Clears the
    /// decisions while the S/N guide is off, so switching it on starts fresh.
    fn snr_label_tick(&self) -> (bool, f32) {
        let mut st = self.snr_labels.borrow_mut();
        // Kept while the guide runs, so a label's fade and decision carry
        // over a moment with the filter off.
        if !self.view.snr_guide {
            *st = SnrLabels::default();
            return (false, 0.0);
        }
        let now = std::time::Instant::now();
        let step = st.last.map_or(0.0, |t| now.duration_since(t).as_secs_f32().min(0.5));
        st.last = Some(now);
        let due =
            st.checked.is_none_or(|t| now.duration_since(t).as_secs_f32() >= SNR_LABEL_RECHECK_S);
        if due {
            st.checked = Some(now);
            // Spots gone from the feeds take their decisions with them.
            let ids: std::collections::HashSet<u64> = self.all_spots().map(|s| s.id).collect();
            st.shown.retain(|id, _| ids.contains(id));
        }
        (due, step)
    }

    pub(in crate::app) fn net_overlay(&self, now_utc: i64) -> (Vec<Spot>, Vec<f32>) {
        let (recheck, step) = self.snr_label_tick();
        let max_age = self.net_cfg_edit.spot_max_age_secs.max(60) as i64;
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        for s in self.all_spots() {
            if !self.spot_visible(s) {
                continue;
            }
            let snr_alpha = self.snr_label_alpha(s.id, s.freq_hz, recheck, step);
            if snr_alpha < 0.15 {
                continue;
            }
            // A scheduled broadcast station has no age: the fade and the
            // max-age cut are both about how stale a *report* is, and a
            // transmitter that is on the air now is not a stale report.
            let a = if s.kind == SpotKind::Broadcast {
                1.0
            } else {
                let age = (now_utc - s.when_utc).max(0);
                if age > max_age {
                    continue;
                } else if age as f64 > max_age as f64 * 0.75 {
                    (1.0 - (age as f64 - max_age as f64 * 0.75) / (max_age as f64 * 0.25)) as f32
                } else {
                    1.0
                }
            };
            // Faded by the S/N filter; under the 0.15 every label keeps, it
            // is not drawn at all (the cut above).
            let a = a * snr_alpha;
            spots.push(s.clone());
            alpha.push(a.clamp(0.15, 1.0));
        }
        (spots, alpha)
    }

    /// The floor/ceiling a fit would pick from the current frame for best
    /// waterfall contrast (noise dark, signals visible, no over-blow). Only the
    /// bins inside the visible viewport are considered, so signals scrolled or
    /// zoomed off-screen (e.g. a strong broadcaster) don't skew the levels —
    /// the emitted frame carries slack beyond the view. `None` when there is no
    /// frame to measure yet.
    fn wanted_levels(&self) -> Option<(f32, f32)> {
        let f = self.frame.as_ref()?;
        let n = f.bins.len();
        if n == 0 || f.span_hz <= 0.0 {
            return None;
        }
        let base = f.center_hz - f.span_hz / 2.0;
        let to_idx = |hz: f64| (hz - base) / f.span_hz * n as f64;
        let i_lo = (to_idx(self.view.view_lo_hz).floor().max(0.0) as usize).min(n);
        let i_hi = (to_idx(self.view.view_hi_hz).ceil().max(0.0) as usize).min(n);
        let slice = if i_hi > i_lo { &f.bins[i_lo..i_hi] } else { &f.bins[..] };
        pick_levels(slice, f.db_floor, f.db_ceil)
    }

    /// Fit the floor/ceiling now, on the operator's say-so (the FIT chip): it
    /// lands in one go, with none of the interval, settle wait or glide that
    /// pace a fit nobody asked for. With no frame to measure yet the fit is
    /// left queued, so switching FIT on before the first frame arrives still
    /// fits as soon as one does.
    pub(in crate::app) fn fit_levels_now(&mut self, now: f64) {
        self.fit.gliding = false;
        let Some(want) = self.wanted_levels() else {
            // Nothing to measure yet. Leave the fit queued and clear of the
            // interval, so the first frame to arrive is fitted to.
            self.fit.pending = true;
            self.fit.started_at = None;
            return;
        };
        (self.view.db_floor, self.view.db_ceil) = want;
        // Where the automatic side would have got to, so it has no correction
        // of its own to make on top of the one just clicked for.
        self.fit.avg = Some(want);
        self.fit.pending = false;
        self.fit.started_at = Some(now);
        self.fit.stepped_at = now;
    }

    /// Keep the levels fitted while [`ViewState::auto_fit`] is on: refit when
    /// the band changes or a pan/zoom settles, and when what the receiver is
    /// hearing has drifted far enough from the window it is drawn over that the
    /// waterfall has gone flat or blown out — starting no more than one fit
    /// every [`FIT_MIN_GAP_S`].
    ///
    /// An automatic fit is a glide, not a jump: the levels are stepped a
    /// quarter of the remaining distance every [`FIT_STEP_S`] towards a rolling
    /// average of what a fit would pick, so the contrast follows the band over
    /// a few seconds instead of snapping around it. The operator's own click
    /// ([`Self::fit_levels_now`]) is exempt — it is a request for *this*
    /// picture, now.
    ///
    /// Runs after the panadapter has drawn, so this frame's pan, zoom and
    /// retune are already in `view`, and before the debounced config update, so
    /// each step goes out to the engine in the frame it was taken.
    pub(in crate::app) fn auto_fit_tick(&mut self, now: f64) {
        // Tracked even while auto-fit is off, so switching it on doesn't count
        // a pan made while it was off as a change and refit a second time
        // straight after the click's own fit.
        let key: FitKey = (
            self.view.view_lo_hz,
            self.view.view_hi_hz,
            self.view.fft_size,
            self.panadapter_bins(),
            self.state.sample_rate,
            sdroxide_types::Band::containing(self.state.active_freq_hz()),
        );
        if self.fit.key != Some(key) {
            self.fit.key = Some(key);
            self.fit.changed_at = now;
            // The average describes a window that is no longer on screen, and
            // any glide was on its way somewhere that no longer exists.
            self.fit.avg = None;
            self.fit.gliding = false;
            // The first look counts too: with auto-fit on by default, the
            // display comes up fitted to the band rather than to whatever the
            // last session left in the settings file.
            self.fit.pending |= self.view.auto_fit;
        }
        if !self.view.auto_fit {
            self.fit.pending = false;
            self.fit.gliding = false;
            return;
        }
        // Measuring costs a sort of every visible bin, and a step that landed
        // sooner than the config debounce would never reach the engine — so
        // both happen on the one cadence.
        if now - self.fit.stepped_at < FIT_STEP_S || now - self.fit.changed_at < FIT_SETTLE_S {
            return;
        }
        let Some(want) = self.wanted_levels() else { return };
        self.fit.stepped_at = now;
        let target = average_in(self.fit.avg, want);
        self.fit.avg = Some(target);
        let have = (self.view.db_floor, self.view.db_ceil);
        if !self.fit.gliding {
            if !fit_due(&self.fit, now) || !(self.fit.pending || levels_drifted(target, have)) {
                return;
            }
            self.fit.pending = false;
            self.fit.gliding = true;
            self.fit.started_at = Some(now);
        }
        // The target keeps moving under the glide — it is the average of
        // everything measured since — so each step is taken against where it
        // has got to, not where it was when the glide began.
        match glide_step(have, target) {
            Some(next) => (self.view.db_floor, self.view.db_ceil) = next,
            None => {
                self.fit.gliding = false;
                (self.view.db_floor, self.view.db_ceil) = target;
            }
        }
    }

    /// The frequency the panadapter anchors on — see [`focus_hz`], which is
    /// where the rule lives. The dial in every mode but the handful that hold
    /// a tone pair off it.
    ///
    /// The controller's offset where it has reported one, because that is the
    /// figure the cursor and the tone markers are drawn from; the mode's own
    /// standard stands in until it has, so the first frame after a mode change
    /// anchors in the same place as the second rather than jumping.
    ///
    /// A status is only believed while it is *for* this mode. The engine's last
    /// report outlives a mode change by a frame or two, and this is called from
    /// the state event that carries the change — an FT8 transmit offset read as
    /// an RTTY tone pair would anchor the window 700 Hz off the tones on the one
    /// frame the operator is most likely to be watching it move.
    pub(in crate::app) fn panadapter_focus_hz(&self) -> f64 {
        let mode = self.state.rx[0].mode;
        let audio_hz = self.digi_status.as_ref().filter(|s| s.mode == mode).map_or_else(
            || match mode {
                Mode::Cw => self.digi_cfg_edit.cw_pitch_hz,
                _ => mode.standard_tone_offset_hz().unwrap_or_default(),
            },
            |s| s.audio_hz,
        );
        focus_hz(mode, self.state.active_freq_hz(), audio_hz, self.ui_settings.cw_qrg)
    }

    /// Write this radio's band-dock state out the moment it changes, rather
    /// than on a clean quit, so it survives a crash — the common way a program
    /// ends on a bench. The stored map is re-read first: every radio tab
    /// writes, and writing a copy read at start would erase the radio another
    /// tab docked since.
    pub(in crate::app) fn remember_band_dock(&mut self) {
        let mut stored = crate::app::persist::load_band_dock();
        if crate::app::persist::band_dock_entry(
            &mut stored,
            self.radio_id,
            self.band_docked,
            self.band_dock_visible,
        ) {
            crate::app::persist::save_band_dock(&stored);
        }
    }

    /// Center the view on the tuned frequency after big jumps (band change,
    /// memory recall, startup) — i.e. whenever the tuning changed AND left
    /// the visible span. Deliberate pans away from the VFO are never
    /// snapped back, and drag-tuning keeps the VFO in view by itself.
    pub(in crate::app) fn recenter_if_tuned_away(&mut self, prev_vfo: f64, prev_rate: f64) {
        let vfo = self.state.active_freq_hz();
        let first = !self.seen_first_state;
        self.seen_first_state = true;
        if self.view.is_unset() {
            return; // spectrum_view will fit and center on first draw
        }
        // The device window grew far past the view. On an audio-mode front end
        // (a CAT rig or an Icom over LAN) that is the radio's own scope taking
        // over the panadapter from the demodulated-audio band as it starts
        // sweeping, or a wider scope span being chosen — either way the whole
        // sweep is what the operator wants to see, not the sliver of it the
        // audio band occupied, which is why the R8600 came up "zoomed in" until
        // the wheel was rolled. Fit to the new window. Edge-triggered on the
        // rate *changing*, so a steady window never pulls a deliberate zoom back
        // out, and gated to audio-mode sources so an SDR that un-decimates keeps
        // its zoom.
        if refit_on_window_growth(
            self.caps.as_ref().is_some_and(|c| c.audio_mode),
            prev_rate,
            self.state.sample_rate,
            self.view.span(),
        ) {
            self.view.fit(self.state.center_hz, self.state.sample_rate);
            return;
        }
        // A zoom window only means anything against the span the front end is
        // delivering. Come up on a different interface — or on the same one at
        // another sample rate — and the restored window can be wider than the
        // whole passband, which draws the spectrum squeezed into part of the
        // panadapter with dead space either side. The same thing happens
        // mid-session the moment the operator decimates the front end, so this
        // is checked on every state rather than only on the first: it costs a
        // comparison, and the window is only ever *narrowed* to a span that is
        // really there. Only the width is settled here; where it sits is
        // settled just below.
        // Only once the front end has said what it offers. The first `State`
        // can arrive ahead of the first `Capabilities`, and until it has there
        // is no way to tell a front end with a full-band lane from one without
        // — so narrowing here would shrink a restored window to the passband on
        // the strength of an answer that had not come yet, which is exactly how
        // a zoomed-out KiwiSDR session came back up at 12 kHz.
        let (out_center, out_span) = self.zoom_out_window();
        if self.caps.is_some() && out_span > 0.0 && self.view.span() > out_span {
            self.view.fit(out_center, out_span);
        }
        // What has to stay on screen is what is being listened to, not the
        // dial — in RTTY the two are 2210 Hz apart, and zoomed in past that the
        // dial can be off the picture while the signal is dead centre. Anchored
        // on the dial, this fired on every click-tune and threw the tone pair
        // out of the window it had just been clicked in. `moved` stays on the
        // dial, because that is the question it asks: did the tuning change.
        let focus = self.panadapter_focus_hz();
        let moved = (vfo - prev_vfo).abs() > 0.5;
        let outside = !(self.view.view_lo_hz..=self.view.view_hi_hz).contains(&focus);
        if (moved || first) && outside {
            let span = self.view.span().min(self.state.sample_rate);
            self.view.view_lo_hz = focus - span / 2.0;
            self.view.view_hi_hz = focus + span / 2.0;
        }
    }
}

/// The longest stretch of signal one device-wide transform may cover.
///
/// Not a resolution choice — a resolution *floor* implied by a rate. The
/// waterfall scrolls on wall-clock, so a window longer than a row's worth of
/// time is drawn as a run of identical rows however fast frames are published,
/// and every event inside it is averaged flat. A tenth of a second keeps the
/// device-wide lane above ten new pictures a second on any front end.
const MAX_FFT_WINDOW_S: f64 = 0.1;

/// The largest device-wide transform the chips offer.
///
/// Not a throughput ceiling: an FFT's cost per *sample* grows only as the log
/// of its size, because the hop grows with it — 131072 on a 2 Msps front end
/// runs a quarter the transforms of 32768 at rather less than four times the
/// work each, so the totals are within a fifth of one another. What bounds it
/// is time. One transform covers `fft / rate` seconds, and that is both the
/// panadapter's update period and its smear: 131072 at 2 Msps is 65 ms and
/// 15 Hz a bin, and the same size on a 48 kHz audio lane would be 2.7 seconds.
///
/// [`MAX_FFT_WINDOW_S`] already holds the *base* to a tenth of a second on any
/// front end, so a narrow lane never reaches here at all. This is the ceiling
/// for the zoom multiplier stacked on top, where the seconds are a price the
/// operator chose by zooming in.
const MAX_FFT: u32 = 131_072;

/// The operator's FFT size, reduced to what `rate_hz` can deliver inside
/// [`MAX_FFT_WINDOW_S`] — the largest power of two that fits, never below the
/// 1024 floor the chip row starts at, and unchanged when the rate is unknown.
///
/// A wideband front end is untouched: 2 Msps affords 131072, well past the
/// 32768 ceiling the chips offer. It bites only on the narrow lanes, which are
/// the ones that cannot afford it — 2048 at 24 kHz, 4096 at 48 kHz.
fn base_fft_for_rate(chip: u32, rate_hz: f64) -> u32 {
    // The persisted `fft_size` is not trusted here: a stale or hand-edited
    // view can name a size the chips never offer, and an absurd one would spin
    // this loop (the `afford * 2` overflow below wraps to zero, after which the
    // condition stays true forever). Bound it to the ceiling the chips use.
    let chip = chip.clamp(1024, MAX_FFT);
    if !rate_hz.is_finite() || rate_hz <= 0.0 {
        return chip;
    }
    let mut afford = 1024u32;
    while afford < chip && f64::from(afford.saturating_mul(2)) / rate_hz <= MAX_FFT_WINDOW_S {
        afford = afford.saturating_mul(2);
    }
    chip.min(afford).max(1024)
}

/// How much wider than the passband a view has to be before it counts as
/// zoomed out past it.
///
/// Not a bare `>`. A front end restates its sample rate as it measures it, and
/// a GPS-disciplined one restates it *differently every session*: a KiwiSDR
/// reported 11998.876241, .876277, .876288, .876369, .876561 and .876576 across
/// six connections to the same receiver. A view fitted to one of those and
/// restored against another is a ten-thousandth of a hertz wider — enough for a
/// bare comparison to declare it zoomed out, whereupon the client asks for
/// twice the passband, the engine answers from the full-band lane, and a
/// 12 kHz window pooled from 29 kHz bins draws as a flat line across the
/// panadapter. A tenth of a percent is far below any zoom step and far above
/// that.
const WIDER_THAN_PASSBAND: f64 = 1.001;

/// Whether a jump in the device window should re-fit the view to it.
///
/// True only for an audio-mode front end whose window grew by a large factor
/// while the view is now a small fraction of it — the scope taking over an
/// Icom/CAT panadapter, or a wider scope span being chosen. The factor gate
/// makes it edge-triggered: a window that merely holds a wide span frame after
/// frame never matches, so a deliberate zoom into part of the sweep survives.
fn refit_on_window_growth(audio_mode: bool, prev_rate: f64, new_rate: f64, view_span: f64) -> bool {
    audio_mode && prev_rate > 0.0 && new_rate > prev_rate * 4.0 && view_span * 2.0 < new_rate
}

/// The frequency the panadapter is anchored on — what is being listened to,
/// which in the modes that hold a tone pair off the dial is not the dial.
///
/// RTTY is what this exists for. Its dial sits 2210 Hz below the mark tone by
/// convention, so clicking a signal tunes the dial *below* the picture rather
/// than onto it. Zoom in past that offset and the dial is no longer on screen
/// at all — and a window re-centred on it throws the picture two kilohertz
/// down and carries the signal that was just clicked off the right-hand edge.
/// The tone pair, the cyan cursor and the passband are all up at `dial + 2210`,
/// and that is where the middle of the window belongs.
///
/// Only where the offset is a standard rather than a slot inside a sub-band
/// ([`Mode::holds_standard_tones`] — RTTY and NAVTEX). FT8 and the rest move
/// their tone offset by themselves, and a view that chased it would slide out
/// from under the operator every time the engine picked a new transmit
/// frequency. CW joins on `cw_qrg`, which is already the setting that says the
/// signal is the frequency: it puts the readout and the tuning line on the
/// cursor, and leaving the window centred on the dial would be the one thing
/// still disagreeing.
///
/// `audio_hz` is unsigned — the distance from the dial — and
/// [`Mode::on_air_hz`] puts it on whichever sideband the mode rides there.
fn focus_hz(mode: Mode, dial_hz: f64, audio_hz: f32, cw_qrg: bool) -> f64 {
    let on_cursor = mode.holds_standard_tones() || (mode == Mode::Cw && cw_qrg);
    if on_cursor { mode.on_air_hz(dial_hz, audio_hz) } else { dial_hz }
}

#[cfg(test)]
mod tests {
    /// The S/N filter's reading of the guide: a carrier 6 dB up 1 kHz off the
    /// spot is found, plain noise reads 0 dB, and a guide with too few frames
    /// gives no verdict yet.
    #[test]
    fn the_guide_is_read_only_once_it_has_frames() {
        let mut snr = vec![0.0f32; 1000];
        snr[510] = 6.0;
        let g = |frames: u32| crate::mean_spectrum::Guide {
            lo_hz: 9_600_000.0,
            cell_hz: 100.0,
            snr_db: snr.clone(),
            count: vec![frames; 1000],
            frames,
        };
        assert_eq!(super::guide_max(&g(100), 9_650_000.0), Some(6.0));
        assert_eq!(super::guide_max(&g(100), 9_620_000.0), Some(0.0));
        assert_eq!(super::guide_max(&g(10), 9_650_000.0), None, "too early to judge");
    }

    /// PEAK (HDZ 2026-11-11, strong QRM): interference lifts 30 kHz of the band
    /// 8 dB over the floor. Every label in it passes S/N MIN by height alone;
    /// with PEAK none does, except a carrier standing 6 dB out of the hump. A
    /// broadcast carrier on quiet band keeps its peak, and the sideband hump of
    /// the 7.305 MHz picture has none.
    #[test]
    fn a_label_needs_a_peak_of_its_own() {
        let mut snr = vec![0.0f32; 1000];
        for v in snr.iter_mut().take(650).skip(350) {
            *v = 8.0;
        }
        snr[400] = 14.0; // a carrier standing out of the hump
        snr[800] = 12.0; // a carrier on quiet band
        for v in snr.iter_mut().take(880).skip(850) {
            *v = 6.0; // a sideband hump, 3 kHz wide
        }
        let g = crate::mean_spectrum::Guide {
            lo_hz: 7_000_000.0,
            cell_hz: 100.0,
            snr_db: snr,
            count: vec![100; 1000],
            frames: 100,
        };
        let at = |cell: f64| 7_000_000.0 + cell * 100.0 + 50.0;
        assert_eq!(super::guide_max(&g, at(500.0)), Some(8.0), "the hump passes by height");
        assert_eq!(super::has_a_peak(&g, at(500.0)), Some(false), "a label in the QRM kept");
        assert_eq!(super::has_a_peak(&g, at(400.0)), Some(true), "the carrier in the QRM lost");
        assert_eq!(super::has_a_peak(&g, at(800.0)), Some(true), "a carrier on quiet band lost");
        assert_eq!(super::has_a_peak(&g, at(865.0)), Some(false), "the sideband hump kept");
    }

    /// FLANKS (HDZ's picture on 7.305 MHz, 2026-11-11): a broadcast carrier
    /// 25 dB up with sidebands 10 dB up for 5 kHz either side. A label 6 kHz off
    /// it stands on the flank and goes; the strong station keeps its own; one
    /// 15 kHz off is beyond the flank; with the rail off nothing is blocked; a
    /// second station of its own 4 kHz off, 20 dB up, is no flank.
    #[test]
    fn a_strong_station_keeps_labels_off_its_flanks() {
        let mut snr = vec![0.0f32; 1000];
        for (i, v) in snr.iter_mut().enumerate().take(550).skip(450) {
            *v = if i == 500 { 25.0 } else { 10.0 };
        }
        let g = crate::mean_spectrum::Guide {
            lo_hz: 7_255_000.0,
            cell_hz: 100.0,
            snr_db: snr.clone(),
            count: vec![100; 1000],
            frames: 100,
        };
        let carrier = 7_255_000.0 + 500.0 * 100.0 + 50.0;
        assert!(super::on_a_flank(&g, carrier + 6_000.0, 15.0), "sideband label kept");
        assert!(!super::on_a_flank(&g, carrier, 15.0), "the station lost its own label");
        assert!(!super::on_a_flank(&g, carrier + 15_000.0, 15.0), "blocked past the flank");
        assert!(
            !super::on_a_flank(&g, carrier + 6_000.0, crate::view::SNR_FLANK_OFF),
            "off blocks"
        );
        // Counter-check on the threshold: a carrier under it blocks nothing.
        assert!(
            !super::on_a_flank(&g, carrier + 6_000.0, 26.0),
            "a carrier under the rail blocked"
        );
        let mut two = snr;
        two[540] = 20.0;
        let g2 = crate::mean_spectrum::Guide { snr_db: two, ..g };
        assert!(!super::on_a_flank(&g2, carrier + 4_000.0, 15.0), "a station of its own blocked");
    }

    use super::WIDER_THAN_PASSBAND;
    use super::{
        AutoFit, FIT_ARRIVED_DB, FIT_MIN_GAP_S, FIT_SETTLE_S, FIT_STEP_S, average_in,
        base_fft_for_rate, fit_due, glide_step, levels_drifted, pick_levels,
        refit_on_window_growth, slack_viewport,
    };

    /// The six sample rates one KiwiSDR reported across six connections. A view
    /// fitted to any of them must still read as "the whole passband" against
    /// any other, or the panadapter falls onto the full-band lane and draws a
    /// flat line — see [`WIDER_THAN_PASSBAND`].
    #[test]
    fn a_view_fitted_to_one_session_is_not_zoomed_out_in_the_next() {
        const SEEN: [f64; 6] =
            [11998.876241, 11998.876277, 11998.876288, 11998.876369, 11998.876561, 11998.876576];
        for fitted in SEEN {
            for now in SEEN {
                assert!(
                    !(fitted > now * WIDER_THAN_PASSBAND),
                    "a view fitted at {fitted} reads as zoomed out against {now}"
                );
            }
        }
    }

    /// It still has to notice a real zoom-out. One step of the wheel is a few
    /// percent, and the smallest thing anyone would call zoomed out is well
    /// past that.
    #[test]
    fn a_real_zoom_out_is_still_recognised() {
        let rate = 11998.876241;
        assert!(rate * 1.02 > rate * WIDER_THAN_PASSBAND, "one wheel step must count");
        assert!(30e6 > rate * WIDER_THAN_PASSBAND, "the whole band certainly counts");
    }

    /// The crash this replaced: an RX-888 scrolled to the top of its band, at a
    /// zoom where the 2× slack covers the whole device window. `dev_hi - slack`
    /// then lands one ULP *below* `dev_lo` — the numbers below are the ones from
    /// the panic — and `f64::clamp` panics rather than pinning the value.
    #[test]
    fn a_viewport_at_the_top_of_the_band_does_not_panic() {
        let (center, span) = (32_996_365.607_435_618_f64, 2_000_000.0_f64);
        let (dev_lo, dev_hi) = (center - span / 2.0, center + span / 2.0);
        assert!(dev_hi - span < dev_lo, "the rounding case being guarded stopped happening");

        // A view wide enough that the slack saturates at the device span, and
        // sitting at the very top of it.
        let vspan = span / 1.5;
        let vp = slack_viewport(center, span, (dev_hi - vspan, dev_hi));
        assert_eq!(vp, (dev_lo, dev_lo + span), "the whole window is the only fit");
    }

    /// Away from that edge the slack still does its job: the window is twice the
    /// visible span, centred on it, and inside the device window.
    #[test]
    fn a_zoomed_in_viewport_keeps_its_slack() {
        let (center, span) = (16_000_000.0, 32_000_000.0);
        let (lo, hi) = slack_viewport(center, span, (10_000_000.0, 11_000_000.0));
        assert_eq!((lo, hi), (9_500_000.0, 11_500_000.0));
        // ...and it is pushed inside the device window rather than hanging off it.
        let (lo, hi) = slack_viewport(center, span, (31_500_000.0, 32_000_000.0));
        assert_eq!((lo, hi), (31_000_000.0, 32_000_000.0));
    }

    /// The scope taking over an Icom's panadapter — the audio band (a few kHz)
    /// gives way to the sweep span (hundreds of kHz), and the view, left on the
    /// old sliver, has to open out to the whole sweep rather than stay zoomed in.
    #[test]
    fn a_scope_taking_over_the_panadapter_refits_the_view() {
        assert!(refit_on_window_growth(true, 4_000.0, 200_000.0, 4_000.0));
        // A wider span chosen mid-session refits too.
        assert!(refit_on_window_growth(true, 200_000.0, 1_000_000.0, 200_000.0));
    }

    #[test]
    fn a_steady_window_leaves_a_deliberate_zoom_alone() {
        // Same span frame after frame while zoomed into 20 kHz of a 200 kHz
        // sweep: no growth, so the zoom is not pulled back out.
        assert!(!refit_on_window_growth(true, 200_000.0, 200_000.0, 20_000.0));
        // A gentle change (un-decimating 2x) is below the factor gate.
        assert!(!refit_on_window_growth(true, 100_000.0, 200_000.0, 50_000.0));
    }

    #[test]
    fn an_sdr_window_growth_is_left_to_the_operator() {
        // Not audio mode: an SDR that un-decimates 8x keeps whatever zoom the
        // operator set, exactly as before.
        assert!(!refit_on_window_growth(false, 250_000.0, 2_000_000.0, 100_000.0));
        // And the cold-start transition (no previous rate) never fits here —
        // that is `spectrum_view`'s job on first draw.
        assert!(!refit_on_window_growth(true, 0.0, 200_000.0, 4_000.0));
    }

    /// The measured fault: an IC-705 over its LAN port on the 12 kHz IF runs
    /// the engine at 24 kHz, where the 32768 chip covers 1.365 s of signal and
    /// hands the waterfall 1.46 new pictures a second. Capped to 2048 it covers
    /// 85 ms — and loses nothing a narrow lane could have drawn anyway: 2048
    /// bins over 24 kHz is already 12 Hz apiece.
    ///
    /// The two largest chips make this cap matter more, not less. They are for
    /// front ends that stream megahertz; on a lane like this one they would be
    /// seconds of smear apiece, and the rate is what refuses them.
    #[test]
    fn a_narrow_lane_cannot_afford_the_widest_chip() {
        assert_eq!(base_fft_for_rate(32_768, 24_000.0), 2048);
        assert_eq!(base_fft_for_rate(super::MAX_FFT, 24_000.0), 2048);
        assert!(2048.0 / 24_000.0 <= super::MAX_FFT_WINDOW_S);
        // The CAT/Audio path, at twice the rate, affords twice the window.
        assert_eq!(base_fft_for_rate(32_768, 48_000.0), 4096);
    }

    /// A persisted `fft_size` is not trusted: an absurd one used to make the
    /// `afford * 2` loop overflow and spin forever on the first frame. It is
    /// bounded to the chips' ceiling, and fast whatever is asked.
    #[test]
    fn an_absurd_stored_fft_size_cannot_spin() {
        assert_eq!(base_fft_for_rate(u32::MAX, 1e300), super::MAX_FFT);
        assert_eq!(base_fft_for_rate(u32::MAX, 0.0), super::MAX_FFT);
        assert_eq!(base_fft_for_rate(1, 48_000.0), 1024, "floored at the chip row's start");
    }

    #[test]
    fn a_wideband_front_end_keeps_what_the_operator_chose() {
        // 2 Msps affords 131072, so every chip on the row passes through.
        assert_eq!(base_fft_for_rate(32_768, 2_000_000.0), 32_768);
        assert_eq!(base_fft_for_rate(4096, 2_000_000.0), 4096);
        assert_eq!(base_fft_for_rate(super::MAX_FFT, 2_000_000.0), super::MAX_FFT);
        // And a rate we do not know yet changes nothing.
        assert_eq!(base_fft_for_rate(32_768, 0.0), 32_768);
    }

    /// Every chip on the row is a power of two the rate cap can actually
    /// deliver somewhere, and none of them is past the ceiling the zoom
    /// multiplier stops at.
    #[test]
    fn every_offered_fft_size_is_reachable() {
        for n in [2048u32, 4096, 8192, 16_384, 32_768, 65_536, 131_072] {
            assert!(n.is_power_of_two());
            assert!(n <= super::MAX_FFT, "{n} is past the ceiling");
            // The rate at which one transform is exactly the longest window
            // allowed; anything faster affords the chip whole.
            let rate = f64::from(n) / super::MAX_FFT_WINDOW_S;
            assert_eq!(base_fft_for_rate(n, rate), n, "{n} at {rate} Hz");
        }
    }

    #[test]
    fn the_floor_holds_however_slow_the_lane() {
        // Below the chip row's own smallest size there is nothing to gain: the
        // cap must not shrink the panadapter to a handful of bins.
        assert_eq!(base_fft_for_rate(32_768, 1_000.0), 1024);
        assert_eq!(base_fft_for_rate(1024, 24_000.0), 1024);
    }

    /// Map a dB value to the u8 code used by a frame spanning `[lo, hi]`.
    fn code(db: f32, lo: f32, hi: f32) -> u8 {
        (((db - lo) / (hi - lo) * 255.0).clamp(0.0, 255.0)) as u8
    }

    #[test]
    fn levels_bracket_noise_and_signals() {
        // Frame mapped over a wide [-120, -20]: mostly noise near -110 with a
        // handful of strong signals near -45.
        let (lo, hi) = (-120.0f32, -20.0f32);
        let mut bins = vec![code(-110.0, lo, hi); 1000];
        bins.extend(std::iter::repeat(code(-45.0, lo, hi)).take(20));
        let (floor, ceil) = pick_levels(&bins, lo, hi).unwrap();
        // Floor just below the noise; ceiling just above the signals.
        assert!((-120.0..-100.0).contains(&floor), "floor {floor}");
        assert!((-55.0..-30.0).contains(&ceil), "ceil {ceil}");
        assert!(ceil - floor >= 24.0, "range {}", ceil - floor);
    }

    #[test]
    fn flat_band_keeps_minimum_range() {
        // A noise-only band still gets a usable contrast window, not a sliver.
        let (lo, hi) = (-120.0f32, -20.0f32);
        let bins = vec![code(-108.0, lo, hi); 512];
        let (floor, ceil) = pick_levels(&bins, lo, hi).unwrap();
        assert!(ceil - floor >= 24.0, "range {}", ceil - floor);
        assert!(floor >= -160.0 && ceil <= 20.0);
    }

    #[test]
    fn empty_frame_returns_none() {
        assert!(pick_levels(&[], -120.0, -20.0).is_none());
        assert!(pick_levels(&[10, 20], -50.0, -50.0).is_none());
    }

    /// A band whose noise has fallen below the window it is mapped over reads
    /// as a solid block of zeroes, and the frame cannot say how far below it
    /// went. Stepping down by the usual few dB would take a fit per interval to
    /// find it again — which, with auto-fit running, is a black waterfall for
    /// half a minute. One fit has to make real ground.
    #[test]
    fn a_frame_clipped_flat_against_the_floor_reaches_well_past_it() {
        let (lo, hi) = (-120.0f32, -20.0f32);
        let (floor, _) = pick_levels(&vec![0u8; 512], lo, hi).unwrap();
        assert!(floor <= lo - 10.0, "floor {floor} barely moved off {lo}");
    }

    /// The same the other way up: signals stronger than the ceiling all read as
    /// 255, so the ceiling has to reach past them rather than creep.
    #[test]
    fn a_frame_clipped_against_the_ceiling_reaches_well_past_it() {
        let (lo, hi) = (-120.0f32, -20.0f32);
        let mut bins = vec![code(-110.0, lo, hi); 1000];
        bins.extend(std::iter::repeat_n(255u8, 40));
        let (_, ceil) = pick_levels(&bins, lo, hi).unwrap();
        assert!(ceil >= hi + 10.0, "ceil {ceil} barely moved off {hi}");
    }

    /// The interval is the whole of the rate limit: nothing — not a band
    /// change, not a pan — starts a fit sooner than [`FIT_MIN_GAP_S`] after the
    /// last one began.
    #[test]
    fn no_two_fits_inside_the_interval() {
        let settled = AutoFit { started_at: Some(100.0), pending: true, ..Default::default() };
        assert!(!fit_due(&settled, 100.0 + FIT_MIN_GAP_S - 0.01), "refitted too soon");
        assert!(fit_due(&settled, 100.0 + FIT_MIN_GAP_S), "the queued fit never ran");
    }

    /// A pan or zoom is a reason to refit, but only once it has stopped moving:
    /// mid-drag the engine is still re-cutting its viewport, and the frame in
    /// hand is of the window being left.
    #[test]
    fn a_fit_waits_for_the_view_to_settle() {
        let moving = AutoFit { changed_at: 100.0, pending: true, ..Default::default() };
        assert!(!fit_due(&moving, 100.0 + FIT_SETTLE_S - 0.01), "fitted mid-drag");
        assert!(fit_due(&moving, 100.0 + FIT_SETTLE_S), "never fitted after the drag");
    }

    /// The drift test is what "too high or too low" means once nothing has been
    /// asked for explicitly: small movement is left alone (so a steady band is
    /// fitted once), a window the signal no longer sits in is taken.
    #[test]
    fn only_real_drift_moves_a_settled_window() {
        let have = (-120.0, -20.0);
        assert!(!levels_drifted((-121.0, -18.5), have), "a couple of dB is not drift");
        assert!(levels_drifted((-140.0, -20.0), have), "the noise fell out of the window");
        assert!(levels_drifted((-120.0, 5.0), have), "the band blew through the ceiling");
    }

    /// An automatic fit slides the levels across rather than switching them:
    /// no single step is a jump, and the whole move is done in about the
    /// interval between fits.
    #[test]
    fn a_glide_covers_the_distance_over_the_interval_without_a_jump() {
        let (from, target) = ((-120.0f32, -20.0f32), (-100.0f32, -40.0f32));
        let mut have = from;
        let mut steps = 0;
        while let Some(next) = glide_step(have, target) {
            let moved = (next.0 - have.0).abs().max((next.1 - have.1).abs());
            assert!(moved <= 0.3 * (target.0 - from.0).abs(), "step {steps} of {moved} dB jumped");
            have = next;
            steps += 1;
            assert!(steps < 100, "the glide never arrived");
        }
        let took = steps as f64 * FIT_STEP_S;
        assert!(
            (FIT_MIN_GAP_S..3.0 * FIT_MIN_GAP_S).contains(&took),
            "a 20 dB move took {took}s, not about {FIT_MIN_GAP_S}s",
        );
        assert!((have.0 - target.0).abs() <= FIT_ARRIVED_DB, "stopped short at {}", have.0);
    }

    /// The target is an average, so a station that comes up for a second or two
    /// barely moves it and is given back the moment it drops — that is the
    /// difference between the contrast following the band and it flinching at
    /// every burst. A band that has really changed keeps feeding the same
    /// measurement, and carries it.
    #[test]
    fn a_brief_signal_hardly_moves_the_target_but_a_changed_band_does() {
        let steady = (-120.0f32, -20.0f32);
        let over = |avg: (f32, f32), up: f32, secs: f64| {
            let mut avg = avg;
            for _ in 0..(secs / FIT_STEP_S) as usize {
                avg = average_in(Some(avg), (steady.0, steady.1 + up));
            }
            avg
        };
        // Two seconds of a signal 10 dB over the rest of the band: not even a
        // third of what it takes to start a fit at all.
        let brief = over(steady, 10.0, 2.0);
        assert!(!levels_drifted(brief, steady), "a two-second burst would start a fit: {brief:?}");
        // Even a huge one only nudges the target — the point being that nothing
        // *jumps*, whatever turns up.
        let huge = over(steady, 40.0, 2.0);
        assert!(huge.1 - steady.1 < 0.25 * 40.0, "a burst took the target most of the way up");
        // And it is handed back over the following half-minute, at the same
        // rate it was taken on: an average that let go faster than it took on
        // would be no protection against a burst that ends and starts again.
        let after = over(huge, 0.0, 20.0);
        assert!((after.1 - steady.1).abs() < 1.5, "the target never came back down: {after:?}");
        // The same measurement for a minute is the band, not a burst.
        let changed = over(steady, 40.0, 60.0);
        assert!(
            levels_drifted(changed, steady),
            "a minute of it never moved the target off {steady:?}",
        );
    }

    /// Nothing to average against yet — a fresh band, a fresh window — is not a
    /// reason to creep towards the right levels from the last band's.
    #[test]
    fn the_first_measurement_of_a_window_is_the_target() {
        assert_eq!(average_in(None, (-130.0, -30.0)), (-130.0, -30.0));
    }
}

#[cfg(test)]
mod focus_tests {
    use super::{Mode, focus_hz};

    /// The whole point: RTTY's tone pair is a couple of kilohertz above the
    /// dial, so the window belongs on the pair. Zoomed in tighter than the
    /// offset — the case the operator hit — the dial is not even on screen,
    /// and anchoring the picture there takes the signal off the far edge.
    #[test]
    fn rtty_anchors_on_the_tone_pair_not_the_dial() {
        let dial = 14_080_000.0;
        let pair = focus_hz(Mode::Rtty, dial, sdroxide_types::RTTY_CENTER_HZ, false);
        assert_eq!(pair, dial + 2210.0, "the window was anchored on the dial");
        // A 1 kHz window centred on the pair holds it; centred on the dial it
        // would not have held it at all.
        assert!(!(pair - 500.0..=pair + 500.0).contains(&dial), "the dial was on screen after all");
    }

    /// A nudged offset moves the anchor with it: the pair is wherever the
    /// operator put it, not wherever the standard says it usually is.
    #[test]
    fn a_nudged_rtty_offset_carries_the_anchor() {
        let dial = 7_040_000.0;
        assert_eq!(focus_hz(Mode::Rtty, dial, 1000.0, false), dial + 1000.0);
    }

    /// NAVTEX holds its tone by convention exactly as RTTY does — the assigned
    /// channel is the centre of the pair, 1700 Hz up — so it comes along.
    #[test]
    fn navtex_anchors_on_its_tone_too() {
        let dial = 518_000.0 - f64::from(sdroxide_types::NAVTEX_TONE_HZ);
        assert_eq!(focus_hz(Mode::Navtex, dial, sdroxide_types::NAVTEX_TONE_HZ, false), 518_000.0);
    }

    /// The modes that pick a slot inside a sub-band stay on the dial. FT8's
    /// tone offset moves by itself whenever the engine chooses a new transmit
    /// frequency, and a window that followed it would slide out from under the
    /// operator mid-QSO.
    #[test]
    fn the_slotted_modes_stay_on_the_dial() {
        let dial = 14_074_000.0;
        for m in [Mode::Ft8, Mode::Ft4, Mode::Js8, Mode::Psk, Mode::Olivia] {
            assert_eq!(focus_hz(m, dial, 1500.0, false), dial, "{m:?} chased its own tone offset");
        }
    }

    /// CW follows the setting that already moved its readout and its tuning
    /// line onto the signal, and nothing changes for an operator who has not
    /// turned it on.
    #[test]
    fn cw_anchors_where_its_readout_reads() {
        let dial = 14_030_000.0;
        assert_eq!(focus_hz(Mode::Cw, dial, 700.0, false), dial, "cw_qrg off moved the window");
        assert_eq!(focus_hz(Mode::Cw, dial, 700.0, true), dial + 700.0);
    }

    /// The analog modes are on their dial and the offset is meaningless there,
    /// whatever `cw_qrg` happens to be set to.
    #[test]
    fn the_analog_modes_are_untouched() {
        let dial = 14_200_000.0;
        for m in [Mode::Usb, Mode::Lsb, Mode::Am, Mode::Nfm] {
            for qrg in [false, true] {
                assert_eq!(focus_hz(m, dial, 700.0, qrg), dial, "{m:?} moved off its dial");
            }
        }
    }

    /// RTTY-FM keys the carrier itself, so the dial *is* the frequency and the
    /// audio offset is internal to the modem. Handed the same tone figure as
    /// its sideband cousin, it must not move — `Mode::on_air_hz` sees a
    /// carrier-centred mode and answers with the dial.
    #[test]
    fn carrier_centred_rtty_stays_on_its_carrier() {
        let dial = 145_500_000.0;
        assert_eq!(focus_hz(Mode::RttyFm, dial, sdroxide_types::RTTY_CENTER_HZ, false), dial);
    }
}

/// Whether a view `(lo, hi)` shows a stretch outside the I/Q window
/// `centre ± span / 2` while it is drawn from the I/Q — black, nothing there.
/// A view wider than the window is drawn from the full-band lane instead
/// (see [`WIDER_THAN_PASSBAND`]) and has no hole; a view inside the window has
/// none either. A thousandth of the view's span of overhang is not a hole: it
/// is rounding, well under a column.
// Why: HDZ 2026-11-10 (picture, RX-888 at 10.73 MHz with the left of the view
//      black): "Autodwell sollte nur ausgelöst werden, wenn auf dem Screen ein
//      leerer Bereich wirklich da ist, sonst so lassen".
pub(in crate::app) fn view_shows_a_hole(view: (f64, f64), centre_hz: f64, span_hz: f64) -> bool {
    let (lo, hi) = view;
    let width = hi - lo;
    if span_hz <= 0.0 || width <= 0.0 || width > span_hz * WIDER_THAN_PASSBAND {
        return false;
    }
    let tol = width / 1000.0;
    lo < centre_hz - span_hz / 2.0 - tol || hi > centre_hz + span_hz / 2.0 + tol
}

#[cfg(test)]
mod hole_tests {
    use super::view_shows_a_hole;

    /// Proves: the hole is what the picture has — a view running off the I/Q
    /// window (the RX-888 case: 2.025 MHz at 11.35, view 9.98–10.78) has one,
    /// a view inside the window has none however far the dial is off centre,
    /// and a view wider than the window (drawn from the full-band lane) has
    /// none.
    #[test]
    fn a_hole_is_a_stretch_of_view_outside_the_window() {
        let (c, s) = (11_350_000.0, 2_025_000.0);
        assert!(view_shows_a_hole((9_980_000.0, 10_780_000.0), c, s));
        assert!(!view_shows_a_hole((10_400_000.0, 11_200_000.0), c, s));
        assert!(!view_shows_a_hole((0.0, 32_400_000.0), c, s));
        // Right on the edge, within rounding: no hole.
        assert!(!view_shows_a_hole((c - s / 2.0 - 1.0, c - s / 2.0 + 800_000.0), c, s));
    }
}
