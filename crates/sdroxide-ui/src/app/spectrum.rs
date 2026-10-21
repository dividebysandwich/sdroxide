//! The panadapter and waterfall: what the engine is asked for, and what gets
//! drawn on top of what comes back.
//!
//! Viewport and FFT changes are debounced before they go out (dragging the
//! span would otherwise reconfigure the engine every frame), and the overlays
//! — CW skimmer boxes, FT8 callsign labels, network spot flags — are rebuilt
//! here each frame from state the rest of the app maintains.

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

/// How long the picture has to stay uniformly at the floor or the ceiling,
/// with auto-fit off, before the panadapter says so over the waterfall. Long
/// enough not to blink on a single dead frame, short enough that nobody has to
/// stare at a flat block wondering whether the radio has stopped.
const LEVELS_HIDDEN_S: f64 = 1.2;

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

/// Whether a slice of drawn bins is uniformly at one end — every bin on the
/// floor or every bin on the ceiling, so the picture is a flat block of one
/// colour that carries no information.
///
/// This is what a waterfall that has "gone black" actually is: the frame is
/// real, the floor/ceiling are simply past everything on it (or below it), and
/// every bin clamps to the same value. A slice too short to judge answers
/// `false`, as does one with even a single signal in it — one visible carrier
/// is a picture, not a flat block.
fn saturated_bins(bins: &[u8]) -> bool {
    if bins.len() < 16 {
        return false;
    }
    bins.iter().all(|&b| b <= 1) || bins.iter().all(|&b| b >= 254)
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
    /// When the picture first went uniformly black or white with auto-fit off.
    /// `None` while the display carries information, or while auto-fit is on
    /// and would be about to fix it itself.
    hidden_since: Option<f64>,
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
        // **SSTV on a demod-audio front end.** Every other mode on such a source
        // wants the whole audio band — it is all there is to see. SSTV does not:
        // the picture is a fixed 1200-2300 Hz band of audio and the dial sits on
        // the carrier, so a view fitted to the whole band shows it as a sliver
        // off to one side and, at the span this one was set to, half of it off
        // the right edge (the operator's own screenshot, 2.0.1). Fit the mode's
        // own band instead — the same "the mode decides the window" rule
        // `conventional_dial_for` applies to the dial.
        //
        // Only on an audio-mode source: an SDR owner may well want the wide view
        // to watch the band around the picture, and on such a front end the
        // picture lands centred anyway because the dial is not the carrier.
        if self.caps.as_ref().is_some_and(|c| c.audio_mode)
            && let Some(window) =
                sstv_audio_window(self.state.rx[0].mode, self.state.active_freq_hz())
        {
            return window;
        }
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
            None => "This window has no GPU renderer, so the waterfall stays at its                      standard width."
                .to_string(),
            Some(c) if c.device_type == crate::egui_wgpu::wgpu::DeviceType::Cpu => {
                "This machine is drawing without a GPU — every column of the waterfall                  is the processor's work, and it is already sharing that with the radio."
                    .to_string()
            }
            Some(c) if c.backend == crate::egui_wgpu::wgpu::Backend::Gl => {
                "This window renders through OpenGL, which is sdroxide's compatibility                  path — a Raspberry Pi, an older graphics chip, or a browser without                  WebGPU. A wider waterfall is not worth the frame rate there."
                    .to_string()
            }
            Some(c) if c.max_texture_dim < waterfall_gpu::MAX_TEX_W => format!(
                "This renderer will not hold a texture wider than {} pixels.",
                c.max_texture_dim
            ),
            Some(_) => "Wider than this renderer can draw.".to_string(),
        };
        DetailReport { chosen: self.bins_for_detail(detail), ceiling, reason }
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
        // "Freeze the waterfall while transmitting": the same mechanism as a
        // stalled stream — no rows, and the clock pinned (see `wf_now_pin`
        // below), so the over leaves no gap and no block of the transmitter's
        // own signal in the received history. Deliberately the waterfall only:
        // the 3D surface below is a *spectrum* feature and stays gated on the
        // stream alone, so this option cannot surprise an operator who was not
        // looking at the waterfall when they set it.
        let tx = self.state.tx.ptt || self.state.tx.tune;
        let freeze_wf = tx && self.ui_settings.waterfall_freeze_on_tx;
        let rows_per_sec = self.ui_settings.waterfall_rows_per_sec() * self.wf_row_scale;
        // Clamp dt so a hitch/tab-away can't dump a huge run of rows at once.
        let dt =
            if self.wf_last_now > 0.0 { (now - self.wf_last_now).clamp(0.0, 0.3) } else { 0.0 };
        self.wf_last_now = now;
        let rows_live = live && !freeze_wf;
        let rows_to_write = if rows_live {
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
        // history that is not moving. A TX freeze is a *delay*, not a stall, so
        // the pin is lifted again the moment rows resume and the gridlines
        // return to the wall clock — the frozen block reads as the times those
        // rows were received, which is what it is.
        if rows_live || self.wf_now_pin == 0.0 {
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
            fill: s.spectrum_fill.then_some((s.spectrum_fill_color, s.spectrum_fill_style)),
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
                let text = if self.ui_settings.memory_waterfall_show_folder {
                    match folder {
                        Some(f) => format!("{f} / {}", m.name),
                        None => m.name.clone(),
                    }
                } else {
                    m.name.clone()
                };
                crate::widgets::memories::MemMark {
                    id: m.id,
                    freq_hz: m.freq_hz,
                    text,
                    color: eframe::egui::Color32::from_rgb(
                        self.ui_settings.memory_waterfall_color[0],
                        self.ui_settings.memory_waterfall_color[1],
                        self.ui_settings.memory_waterfall_color[2],
                    ),
                }
            })
            .collect()
    }

    pub(in crate::app) fn net_overlay(&self, now_utc: i64) -> (Vec<Spot>, Vec<f32>) {
        let max_age = self.net_cfg_edit.spot_max_age_secs.max(60) as i64;
        let mut spots = Vec::new();
        let mut alpha = Vec::new();
        for s in self.all_spots() {
            if !self.spot_visible(s) {
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
            spots.push(s.clone());
            alpha.push(a.clamp(0.15, 1.0));
        }
        // One name for a station several sources carry — `broadcast_label_dedupe`.
        // Once EiBi, Aoki and HFCC are merged this is not a tidy-up, it is the
        // difference between a readable band and the same broadcaster printed
        // twice on top of itself.
        let kept = self.broadcast_label_dedupe(&spots);
        let alpha: Vec<f32> = kept
            .iter()
            .map(|k| {
                spots
                    .iter()
                    .find(|o| o.id == k.id)
                    .map(|o| {
                        spots.iter().position(|x| x.id == o.id).map(|i| alpha[i]).unwrap_or(1.0)
                    })
                    .unwrap_or(1.0)
            })
            .collect();
        (kept, alpha)
    }

    /// The floor/ceiling a fit would pick from the current frame for best
    /// waterfall contrast (noise dark, signals visible, no over-blow). Only the
    /// bins inside the visible viewport are considered, so signals scrolled or
    /// zoomed off-screen (e.g. a strong broadcaster) don't skew the levels —
    /// the emitted frame carries slack beyond the view. `None` when there is no
    /// frame to measure yet.
    fn wanted_levels(&self) -> Option<(f32, f32)> {
        let (slice, floor, ceil) = self.visible_bins()?;
        pick_levels(slice, floor, ceil)
    }

    /// The bins the operator can actually see: the slice of the current frame
    /// inside the view window, or the whole frame when the window does not
    /// intersect it — together with the frame's own `[db_floor, db_ceil]`
    /// mapping, which is what its u8 bins stand for. `None` when there is no
    /// frame to measure yet, or it is degenerate.
    ///
    /// Shared so the fit and the "levels are hiding everything" check always
    /// look at exactly the same bins.
    fn visible_bins(&self) -> Option<(&[u8], f32, f32)> {
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
        Some((slice, f.db_floor, f.db_ceil))
    }

    /// Whether the visible span is uniformly at one end, so the waterfall reads
    /// as a flat block — see [`saturated_bins`].
    fn display_saturated(&self) -> bool {
        self.visible_bins().is_some_and(|(slice, _, _)| saturated_bins(slice))
    }

    /// Track how long the picture has been uniformly black or white with
    /// auto-fit off. Runs every frame, whether or not auto-fit is on, so
    /// switching auto-fit off on an already-flat display starts the clock
    /// rather than waiting for a fit that will never come.
    fn note_levels_hidden(&mut self, now: f64) {
        let hidden = !self.view.auto_fit && self.display_saturated();
        match (hidden, self.fit.hidden_since) {
            (false, _) => self.fit.hidden_since = None,
            (true, None) => self.fit.hidden_since = Some(now),
            (true, Some(_)) => {}
        }
    }

    /// Whether the panadapter should say, over the picture, that the levels are
    /// hiding everything. Only after the state has lasted a moment, so a single
    /// dead frame on a band change does not flash a hint.
    pub(in crate::app) fn levels_hidden(&self, now: f64) -> bool {
        self.fit.hidden_since.is_some_and(|t| now - t >= LEVELS_HIDDEN_S)
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
        // Ahead of the auto-fit gate: the "levels are hiding everything" clock
        // has to run while auto-fit is *off*, which is the only time the hint
        // is warranted.
        self.note_levels_hidden(now);
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
        let audio_front_end = self.caps.as_ref().is_some_and(|c| c.audio_mode);
        focus_hz(
            mode,
            self.state.active_freq_hz(),
            audio_hz,
            self.ui_settings.cw_qrg,
            audio_front_end,
        )
    }

    /// Open SSTV on its picture: the view centred on the picture's band at
    /// [`SSTV_VIEW_SPAN_HZ`]. Called when the mode *becomes* SSTV, never per
    /// frame, so the operator's own zoom afterwards stands.
    pub(in crate::app) fn fit_sstv_view(&mut self) {
        if let Some((centre, span)) =
            sstv_audio_window(self.state.rx[0].mode, self.state.active_freq_hz())
        {
            let span =
                if self.state.sample_rate > 0.0 { span.min(self.state.sample_rate) } else { span };
            self.view.fit(centre, span);
        }
    }

    /// Open WEFAX with the tuned frequency at the centre of the view, as wide as
    /// [`WEFAX_VIEW_SPAN_HZ`]. Called when the mode *becomes* WEFAX, never per
    /// frame, so the operator's own zoom and centring afterwards stand.
    pub(in crate::app) fn fit_wefax_view(&mut self) {
        if let Some((centre, span)) = wefax_open_window(
            self.state.rx[0].mode,
            self.state.active_freq_hz(),
            self.state.sample_rate,
        ) {
            self.view.fit(centre, span);
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
fn focus_hz(mode: Mode, dial_hz: f64, audio_hz: f32, cw_qrg: bool, audio_front_end: bool) -> f64 {
    // **SSTV's picture is not on the dial — on a front end that only has
    // demodulated audio.** The dial there sits on the suppressed carrier and the
    // picture is 1200-2300 Hz of audio to one side, so "what must stay on
    // screen" is the picture, not the carrier: without this the window is
    // centred on the carrier however narrow it is made, which is what the first
    // attempt did and the screenshots showed — an empty centre line with the
    // picture either side of it.
    //
    // **On every front end now, SDRs included** (2026-10-12, at the
    // operator's word). It used to be demod-audio only, on the reasoning that
    // an SDR operator tunes the dial onto the picture. On the air they do not:
    // the dial sits on the SSTV calling frequency, the picture sits 1200-2300
    // Hz to one side of it exactly as on a sound-card rig, and a HackRF on
    // 80 m showed it as a sliver left of a view centred on nothing. The same
    // view on every radio is also what the browser and the native builds then
    // agree on.
    //
    // Kept here rather than as a `standard_tone_offset_hz` because this is a
    // **view** question, not a logging or transmit one: the contact's frequency
    // for SSTV is the dial, and `on_air_hz` is the answer the log reads.
    if mode.is_sstv() {
        return if mode.is_lower_sideband_at(dial_hz) {
            dial_hz - SSTV_TONE_HZ
        } else {
            dial_hz + SSTV_TONE_HZ
        };
    }
    //
    // **And the same for RTTY, NAVTEX and DSC — the SSTV case's twin.** Their
    // tone pair is off the dial *only* on a front end that hands over demodulated
    // audio. On an SDR the dial is on the station; `Mode`'s own doc says RTTY
    // stations are worked "by moving the dial onto them". Anchoring 2210 Hz above
    // it left the station outside the window at any span narrow enough to read the
    // tones — the span a contest operator runs — and CTR is on by default, so
    // nobody had to ask for it (Thieu, VERON, on 2.0.7.1).
    //
    // A **view** question only: the *log* reads `Mode::on_air_hz` directly
    // (`app/mod.rs`), so a NAVTEX or DSC decode is still recorded on the channel
    // centre and not 1.7 kHz low.
    if audio_front_end && mode.holds_standard_tones() {
        return mode.on_air_hz(dial_hz, audio_hz);
    }
    // CW keeps the cursor centring unconditionally: it sits behind the named
    // `cw_qrg` setting rather than being forced by the mode, so the operator
    // asks for it and knows what they asked for. Left alone on purpose — if
    // `cw_qrg` turns out to want the same split, that is its own change with
    // its own report behind it.
    let on_cursor = mode == Mode::Cw && cw_qrg;
    if on_cursor { mode.on_air_hz(dial_hz, audio_hz) } else { dial_hz }
}

/// The SSTV picture's audio band, in hertz from the suppressed carrier: the
/// sync pulse is at 1200 and white at 2300, so the picture occupies 1200–2300
/// and its centre is 1750. A protocol fact rather than a taste — every mode in
/// the published SSTV set uses the same two frequencies.
pub(in crate::app) const SSTV_TONE_HZ: f64 = 1750.0;

/// The view span SSTV opens at: the 1100 Hz picture in the middle and about
/// as much again either side, so a neighbouring picture shows at the edges.
/// One value for every front end and every build.
const SSTV_VIEW_SPAN_HZ: f64 = 3000.0;

/// Where to put the view for SSTV — `None` for every other mode, so the
/// caller keeps its own answer. On a demod-audio front end it is also the
/// widest the view goes (see `zoom_out_window`); on an SDR it is only where
/// the view opens (see `fit_sstv_view`).
///
/// The side is the band's, not a guess: SSTV rides the **lower** sideband on
/// 160/80/40 m, and [`Mode::is_lower_sideband_at`] already knows that because
/// the demodulator needs the same answer.
fn sstv_audio_window(mode: Mode, dial_hz: f64) -> Option<(f64, f64)> {
    if !mode.is_sstv() {
        return None;
    }
    let centre = if mode.is_lower_sideband_at(dial_hz) {
        dial_hz - SSTV_TONE_HZ
    } else {
        dial_hz + SSTV_TONE_HZ
    };
    Some((centre, SSTV_VIEW_SPAN_HZ))
}

/// The width WEFAX opens at, in Hz, centred on the tuned frequency. It is
/// clamped to the sample rate where one is known, as SSTV's is.
const WEFAX_VIEW_SPAN_HZ: f64 = 16_000.0;

/// Where to put the view for WEFAX — `None` for every other mode, so the caller
/// keeps its own answer. The centre is the tuned frequency: the dial is not
/// moved, only the view is.
fn wefax_open_window(mode: Mode, dial_hz: f64, sample_rate: f64) -> Option<(f64, f64)> {
    if !mode.is_wefax() {
        return None;
    }
    let span =
        if sample_rate > 0.0 { WEFAX_VIEW_SPAN_HZ.min(sample_rate) } else { WEFAX_VIEW_SPAN_HZ };
    Some((dial_hz, span))
}

#[cfg(test)]
mod tests {
    use super::Mode;
    use super::WIDER_THAN_PASSBAND;
    use super::{
        AutoFit, FIT_ARRIVED_DB, FIT_MIN_GAP_S, FIT_SETTLE_S, FIT_STEP_S, average_in,
        base_fft_for_rate, fit_due, focus_hz, glide_step, levels_drifted, pick_levels,
        refit_on_window_growth, saturated_bins, slack_viewport, sstv_audio_window,
        wefax_open_window,
    };

    /// The SSTV view on a demod-audio front end. The picture is 1200-2300 Hz of
    /// audio and the dial sits on the carrier, so the window belongs 1750 Hz to
    /// the side — the upper sideband where SSTV uses it, the lower one on
    /// 160/80/40 m, which is the band's answer and not ours to guess.
    #[test]
    fn the_sstv_view_sits_on_the_picture_not_the_carrier() {
        // 11 m: USB, so the picture is above the dial. This is the reported
        // case — 27.700 dial, picture off the right edge.
        let (c, s) = sstv_audio_window(Mode::Sstv, 27_700_000.0).expect("SSTV answers");
        assert_eq!(c, 27_701_750.0);
        assert!(s >= 1_100.0, "the span must hold 1200-2300 Hz of picture: {s}");
        // 80 m: SSTV rides the lower sideband there.
        let (c, _) = sstv_audio_window(Mode::Sstv, 3_730_000.0).expect("SSTV answers");
        assert_eq!(c, 3_728_250.0, "80 m is LSB for SSTV");
        // Every other mode keeps the window it would have had.
        assert!(sstv_audio_window(Mode::Ft8, 14_074_000.0).is_none());
        // **And the view must be centred there too.** The first attempt at this
        // narrowed the window and left it on the carrier, which is what the
        // screenshots showed: the picture either side of an empty centre line.
        assert_eq!(focus_hz(Mode::Sstv, 27_700_000.0, 0.0, false, true), 27_701_750.0);
        assert_eq!(focus_hz(Mode::Sstv, 3_730_000.0, 0.0, false, true), 3_728_250.0);
        // A mode that really is worked on the dial is untouched...
        assert_eq!(focus_hz(Mode::Ft8, 14_074_000.0, 0.0, false, true), 14_074_000.0);
        // ...and **an SDR centres on the picture too** — a HackRF on 80 m
        // showed it as a sliver left of a view centred on the empty dial.
        assert_eq!(focus_hz(Mode::Sstv, 3_730_000.0, 0.0, false, false), 3_728_250.0);
        // The span it opens at, the same on every front end: the picture and
        // about as much again either side for the neighbours.
        let (_, s) = sstv_audio_window(Mode::Sstv, 3_730_000.0).expect("SSTV answers");
        assert_eq!(s, 3_000.0);
    }

    /// WEFAX opens 16 kHz wide with the tuned frequency at its centre, and the
    /// dial is never the view's business. A narrower front end clamps the span
    /// to its sample rate; every other mode keeps its own answer.
    #[test]
    fn wefax_opens_16_khz_wide_on_the_tuned_frequency() {
        assert_eq!(
            wefax_open_window(Mode::Wefax, 7_000_000.0, 2_400_000.0),
            Some((7_000_000.0, 16_000.0)),
        );
        assert_eq!(
            wefax_open_window(Mode::Wefax, 7_000_000.0, 12_000.0),
            Some((7_000_000.0, 12_000.0)),
        );
        assert!(wefax_open_window(Mode::Ft8, 14_074_000.0, 2_400_000.0).is_none());
    }

    /// The "levels are hiding everything" test: a slice that is *all* at one
    /// end is a flat block and warrants the hint; anything with a signal in it
    /// is a picture, however dark, and does not. A slice too short to judge is
    /// not evidence either way.
    #[test]
    fn only_a_uniformly_flat_span_counts_as_saturated() {
        assert!(saturated_bins(&[0u8; 64]), "an all-floor span is flat black");
        assert!(saturated_bins(&[255u8; 64]), "an all-ceiling span is flat white");
        assert!(!saturated_bins(&[0u8; 8]), "too short to mean anything");
        let mut one_signal = [0u8; 100];
        one_signal[42] = 180;
        assert!(!saturated_bins(&one_signal), "one carrier is still a picture");
        let mut one_dip = [255u8; 100];
        one_dip[7] = 20;
        assert!(!saturated_bins(&one_dip), "and so is one gap in a blown-out span");
    }

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

    /// **RTTY: the anchor follows whichever side of the offset the *signal* is
    /// on, and which side that is depends on the radio.**
    ///
    /// On an SDR the dial *is* the station — `Mode`'s own doc says RTTY
    /// stations are worked "by moving the dial onto them" — so the window
    /// belongs on the dial. Anchoring 2210 Hz above it put the station
    /// **outside** a 1 kHz window entirely, which is the span a contest
    /// operator runs (Thieu, VERON, on 2.0.7.1: *"lijkt nog moeilijker te
    /// remmen dan 201"*).
    ///
    /// On a front end that hands over demodulated audio the dial sits on a
    /// suppressed carrier and the pair really is 2210 Hz away, so there the
    /// tone anchor is the correct one. This is SSTV's split applied to the
    /// mode next door — and it is why this test used to assert the other half
    /// of it, on every radio.
    #[test]
    fn rtty_anchors_on_whichever_side_the_signal_is() {
        let dial = 14_080_000.0;
        let pair = sdroxide_types::RTTY_CENTER_HZ;
        // SDR: the operator puts the dial on the station, so the window does too.
        let on_sdr = focus_hz(Mode::Rtty, dial, pair, false, false);
        assert_eq!(on_sdr, dial, "the window left the station it is tuned to");
        // And the 1 kHz view that provoked the report now holds it.
        assert!(
            (on_sdr - 500.0..=on_sdr + 500.0).contains(&dial),
            "a 1 kHz window missed the station"
        );
        // Sound card: the dial is on a suppressed carrier and the pair is the
        // signal, so here the anchor stays where it was.
        assert_eq!(focus_hz(Mode::Rtty, dial, pair, false, true), dial + 2210.0);
    }

    /// A nudged offset moves the anchor with it: the pair is wherever the
    /// operator put it, not wherever the standard says it usually is. Only
    /// meaningful on a sound card now — on an SDR the operator moves the dial
    /// itself, so a nudged offset has nothing to move.
    #[test]
    fn a_nudged_rtty_offset_carries_the_anchor() {
        let dial = 7_040_000.0;
        assert_eq!(focus_hz(Mode::Rtty, dial, 1000.0, false, true), dial + 1000.0);
        assert_eq!(focus_hz(Mode::Rtty, dial, 1000.0, false, false), dial);
    }

    /// NAVTEX holds its tone by convention exactly as RTTY does — the assigned
    /// channel is the centre of the pair, 1700 Hz up — and the split is the
    /// same for the same reason.
    ///
    /// The **log** is untouched by any of this: it reads `Mode::on_air_hz`
    /// directly, so a NAVTEX decode is recorded on the channel centre and not
    /// 1.7 kHz low whichever radio it arrived on. Only the window moves.
    #[test]
    fn navtex_anchors_on_its_tone_only_on_a_sound_card() {
        let dial = 518_000.0 - f64::from(sdroxide_types::NAVTEX_TONE_HZ);
        assert_eq!(
            focus_hz(Mode::Navtex, dial, sdroxide_types::NAVTEX_TONE_HZ, false, true),
            518_000.0
        );
        assert_eq!(
            focus_hz(Mode::Navtex, dial, sdroxide_types::NAVTEX_TONE_HZ, false, false),
            dial
        );
    }

    /// DSC's channel frequencies are the centre of *its* tone pair too, so the
    /// same 1700 Hz arithmetic and the same front-end split apply. The logging
    /// it was written for reads `Mode::on_air_hz` directly and is unaffected.
    #[test]
    fn dsc_anchors_on_its_tone_only_on_a_sound_card() {
        let dial = 2_187_500.0 - f64::from(sdroxide_types::DSC_TONE_HZ);
        assert_eq!(
            focus_hz(Mode::Dsc, dial, sdroxide_types::DSC_TONE_HZ, false, true),
            2_187_500.0
        );
        assert_eq!(focus_hz(Mode::Dsc, dial, sdroxide_types::DSC_TONE_HZ, false, false), dial);
    }

    /// The modes that pick a slot inside a sub-band stay on the dial. FT8's
    /// tone offset moves by itself whenever the engine chooses a new transmit
    /// frequency, and a window that followed it would slide out from under the
    /// operator mid-QSO.
    #[test]
    fn the_slotted_modes_stay_on_the_dial() {
        let dial = 14_074_000.0;
        for m in [Mode::Ft8, Mode::Ft4, Mode::Js8, Mode::Psk, Mode::Olivia] {
            assert_eq!(
                focus_hz(m, dial, 1500.0, false, false),
                dial,
                "{m:?} chased its own tone offset"
            );
        }
    }

    /// CW follows the setting that already moved its readout and its tuning
    /// line onto the signal, and nothing changes for an operator who has not
    /// turned it on.
    #[test]
    fn cw_anchors_where_its_readout_reads() {
        let dial = 14_030_000.0;
        assert_eq!(
            focus_hz(Mode::Cw, dial, 700.0, false, false),
            dial,
            "cw_qrg off moved the window"
        );
        assert_eq!(focus_hz(Mode::Cw, dial, 700.0, true, false), dial + 700.0);
    }

    /// The analog modes are on their dial and the offset is meaningless there,
    /// whatever `cw_qrg` happens to be set to.
    #[test]
    fn the_analog_modes_are_untouched() {
        let dial = 14_200_000.0;
        for m in [Mode::Usb, Mode::Lsb, Mode::Am, Mode::Nfm] {
            for qrg in [false, true] {
                assert_eq!(focus_hz(m, dial, 700.0, qrg, false), dial, "{m:?} moved off its dial");
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
        assert_eq!(
            focus_hz(Mode::RttyFm, dial, sdroxide_types::RTTY_CENTER_HZ, false, false),
            dial
        );
    }
}
