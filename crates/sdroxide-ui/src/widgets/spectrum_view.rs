//! The panadapter: spectrum line + frequency scale + GPU waterfall, with
//! drag-pan, wheel-zoom (around the cursor), click-to-tune (shift-click places
//! the second receiver — the sub when it is running, VFO B otherwise),
//! draggable passband filter edges for both receivers, a sub-receiver that
//! tunes by dragging inside its own passband, and a shift+drag bandwidth
//! measurement ruler.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
    pos2, vec2,
};
use sdroxide_types::{
    Command, Mode, RadioState, RxId, SkimmerKind, SkimmerSpot, SpectrumFrame, Spot, Vfo,
    WheelAction, WheelSettings,
};

use crate::view::ViewState;
use crate::waterfall_gpu::WaterfallCallback;

const SCALE_H: f32 = 18.0;

/// Height of the frequency-scale strip — [`SCALE_H`] grown with the
/// panadapter font-size setting, so a bigger scale font keeps its strip.
fn scale_h() -> f32 {
    SCALE_H * crate::theme::panadapter_font_scale()
}
/// Tuning rounds to this step on click-tune, unless the operator has changed
/// [`WheelSettings::click_tune_step_hz`].
const CLICK_TUNE_STEP: f64 = 10.0;
/// Pixel distance for grabbing a filter edge with a mouse, and the floor a
/// touched layout's wider zone is never squeezed below. See `edge_grab`.
const EDGE_GRAB_PX: f32 = 6.0;
/// Pixel distance for grabbing the sub receiver's tuning line — its passband is
/// grabbable in full, but in SSB the line sits outside it. Scaled with the edge
/// zone so the two keep their relative feel on a touched layout.
const MARKER_GRAB_PX: f32 = 5.0;
/// Constant bearing friction a coasting dial loses speed to, in screen points
/// per second². This sets the character of the flywheel: the coast lasts
/// roughly the release speed divided by this, so a flick twice as fast runs
/// about twice as long — which is how a weighted VFO knob behaves and what
/// makes the gesture predictable.
const FLING_FRICTION: f32 = 1400.0;
/// How far a drag on the frequency-scale strip has to travel before it is
/// claimed as a resize or as a sideways pan. Far enough that the pointer's
/// direction means something, short enough that neither gesture feels like it
/// took a moment to start.
const SCALE_AXIS_LOCK: f32 = 4.0;

/// Viscous drag, per second, on top of the friction. Stops a very hard flick
/// from crossing half a band without making a gentle one feel sticky: an
/// ordinary flick coasts under a second and about half a panadapter width, a
/// hard one a second and a half and a couple of widths.
const FLING_DRAG: f32 = 0.6;
/// Release speed below which the drag was *placing* the dial rather than
/// spinning it, so it stops exactly where it was let go. Careful tuning must
/// stay exact — this is the whole reason there is a threshold at all.
const FLING_MIN_LAUNCH: f32 = 220.0;
/// Speed at which a coast has run out and the dial is parked.
const FLING_STOP: f32 = 25.0;

/// The speed a coasting dial has left after `dt` seconds of friction and drag.
///
/// Both terms are taken as one step so the decay can never carry the speed
/// through zero and spin the dial back the way it came: a decay larger than the
/// speed itself parks it.
fn fling_decay(vx: f32, dt: f32) -> f32 {
    let decay = vx * FLING_DRAG * dt + vx.signum() * FLING_FRICTION * dt;
    if decay.abs() >= vx.abs() { 0.0 } else { vx - decay }
}

/// A dial still coasting after the drag that spun it let go.
///
/// The velocity is in screen points per second, not Hz: a mechanical dial does
/// not know what band scale the display is at, so the same flick has to coast
/// the same *visible* distance whether the view spans 3 kHz or 3 MHz. The Hz
/// step is derived per frame from the span in force at the time.
#[derive(Clone, Copy)]
struct Fling {
    /// True when the sub receiver is the dial that is turning.
    sub: bool,
    /// Signed pointer velocity in points/second; positive is rightwards.
    vx: f32,
}

/// What a pan is allowed to do to the *captured* window once the view has run
/// off the end of it — the case the operator meets the moment they are fully
/// zoomed out, where the view is the window and has nowhere left to slide.
///
/// Built by [`WindowPan::of`] from the front end's capabilities. A drag that
/// reaches the edge hands the rest of itself to the front end, which moves its
/// centre and keeps the picture scrolling (issue #133).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowPan {
    /// The centre is the front end's own and can be commanded while the dial
    /// stays put — every SDR. False where the centre *is* the dial (a
    /// transceiver whose I/Q output feeds a sound card, an Icom sending its
    /// 12 kHz IF): there the drag already moves the window by moving the dial,
    /// and asking for a centre as well would be a second CAT write per frame.
    pub movable: bool,
    /// How far the centre may be taken, `(lo, hi)`. The receive range the
    /// centre is currently inside, so a pan slides the window along that range
    /// and stops at its ends rather than hopping the gap to another one — an
    /// RX-888's HF and VHF ranges are not one stretch of dial, and asking for
    /// the gap between them is a refusal the engine would answer sixty times a
    /// second.
    pub limits: (f64, f64),
    /// The widest window the view may be zoomed out to, `(centre, span)`, when
    /// that is wider than the I/Q the front end is streaming.
    ///
    /// Set from the full-band lane, where there is one: a KiwiSDR sends 12 kHz
    /// of I/Q and a picture of the whole 0-30 MHz, a SpyServer a decimated
    /// stream and an FFT of the band around it. Zooming out past the I/Q is
    /// then a real question with a real answer, and the engine draws those
    /// spans from the wide bins.
    ///
    /// `None` on a front end whose I/Q is all there is, where the passband is
    /// the end of the zoom and always was.
    pub outer: Option<(f64, f64)>,
    /// Whether the front end has described itself yet.
    ///
    /// False in the gap between the first `State` and the first
    /// `Capabilities` — a couple of frames, and long enough to do damage: the
    /// passband is known there but the full-band lane is not, so bounding the
    /// view would shrink a restored window to the I/Q and the operator's zoom
    /// would be gone before the answer arrived.
    pub known: bool,
}

impl WindowPan {
    /// What the front end at `center_hz` will take. Nothing at all until the
    /// capabilities have arrived: a pan that guessed would be guessing about
    /// the one thing that decides whether it may act.
    pub fn of(caps: Option<&sdroxide_types::DeviceCaps>, center_hz: f64) -> Self {
        let Some(caps) = caps else { return Self::default() };
        // A device that publishes no ranges is taken at its word, the same way
        // `DeviceCaps::may_rx_hz` takes it.
        let limits = caps
            .freq_ranges_rx
            .iter()
            .find(|&&(lo, hi)| (lo..=hi).contains(&center_hz))
            .copied()
            .unwrap_or((f64::MIN, f64::MAX));
        Self { movable: !caps.audio_mode && !caps.center_is_dial, limits, outer: None, known: true }
    }

    /// Add the full-band window the view may zoom out into — see
    /// [`WindowPan::outer`]. Ignored unless it is genuinely wider than the I/Q,
    /// so a front end whose "wide" lane is no bigger than its passband (an
    /// Icom's scope at a narrow span) is left exactly as it was.
    pub fn with_outer(mut self, wide: Option<(f64, f64)>, iq_span: f64) -> Self {
        self.outer = wide.filter(|&(_, span)| span > iq_span && iq_span > 0.0);
        self
    }

    /// The window the view may be zoomed and panned inside: the full-band lane
    /// where there is one, else the I/Q passband.
    fn view_bounds(&self, dev_center: f64, dev_span: f64) -> (f64, f64) {
        self.outer.unwrap_or((dev_center, dev_span))
    }
}

/// Slide the view by `dhz` and report what would not fit: how far the front
/// end's own centre has to move for the rest of the gesture.
///
/// Zero while the view still has room inside the captured span, so panning
/// within a zoomed-in view never disturbs the hardware. Fully zoomed out the
/// view *is* the span and there is no room at all, so this is the whole of
/// `dhz` from the first pixel — which is the case issue #133 reported, where
/// the drag did nothing until the dial had crept 45% of the span away and the
/// engine jumped the window after it.
///
/// The view is inside the window when this is called — [`ViewState::clamp_to`]
/// at the end of every frame guarantees it — so the overshoot is exactly the
/// part of this pan the window could not absorb.
fn pan_view(view: &mut ViewState, dhz: f64, dev_center: f64, dev_span: f64) -> f64 {
    view.view_lo_hz += dhz;
    view.view_hi_hz += dhz;
    let (lo, hi) = (dev_center - dev_span / 2.0, dev_center + dev_span / 2.0);
    if view.view_lo_hz < lo {
        view.view_lo_hz - lo
    } else if view.view_hi_hz > hi {
        view.view_hi_hz - hi
    } else {
        0.0
    }
}

/// Hand the part of a pan the captured span could not absorb to the front end:
/// move its centre by `over` so the picture carries on scrolling instead of
/// stopping dead against the edge of the window.
///
/// `dev_center` is updated in step and `state` echoes it optimistically, both
/// so that this frame's [`ViewState::clamp_to`] pins the view inside the window
/// being *asked for* rather than the one being left — without that the view
/// would be dragged back a frame at a time and the drag would fight itself.
///
/// The engine reports the centre it actually reached (an RX-888's downconverter
/// clamps its own, at the ends of the half-spectrum), so the picture simply
/// stops there.
/// Where a tuning gesture has the dial after one more frame's delta.
///
/// `carried` is what this gesture had last frame — `None` on the frame it
/// starts — and `echoed` is the dial in the state the engine last sent. The
/// gesture's own value wins whenever it has one, and that is the whole point:
/// `echoed` lags, so building on it throws away every delta sent since it was
/// stamped. See the `gesture-dial` comment in the interaction handler for what
/// that looks like on screen.
fn gesture_step(carried: Option<f64>, echoed: f64, dhz: f64) -> f64 {
    carried.unwrap_or(echoed) + dhz
}

/// Bring the receiver to a view that has been zoomed back in somewhere it is
/// not listening.
///
/// Only on a front end whose I/Q is a narrow window onto a wider band — one
/// with a full-band lane ([`WindowPan::outer`]). There, zooming out shows the
/// whole band from that lane and zooming back in lands wherever the pointer
/// was, which is usually *not* the slice the receiver is streaming. The
/// panadapter then goes on drawing from the coarse full-band bins however far
/// in the operator zooms, because those really are the only bins covering what
/// they are looking at — detail that never arrives, and no visible reason why.
///
/// So the receiver follows. On these front ends the panadapter *is* the
/// receiver: there is no looking without listening, and a zoom onto a signal
/// is a request to hear it — the same request clicking the full-band strip
/// makes, answered the same way.
///
/// Two conditions keep it quiet. The view has to have come back inside a
/// passband's *width*, so panning about while zoomed out never retunes
/// anything; and the dial has to be outside the view, which is what makes this
/// fire once and then stop — once the engine has brought the receiver over,
/// the dial is in the view and there is nothing left to ask for.
fn follow_view_into_the_band(
    view: &ViewState,
    state: &RadioState,
    dev_center: f64,
    dev_span: f64,
    pan: WindowPan,
    cmds: &mut Vec<Command>,
) {
    if pan.outer.is_none() || !(dev_span > 0.0) || view.span() > dev_span {
        return;
    }
    let (v_lo, v_hi) = (view.view_lo_hz, view.view_hi_hz);
    // Already looking at what is being received: the passband covers the view.
    let (dev_lo, dev_hi) = (dev_center - dev_span / 2.0, dev_center + dev_span / 2.0);
    if v_lo >= dev_lo && v_hi <= dev_hi {
        return;
    }
    // Already asked: the dial is in the view and the engine is bringing the
    // receiver over. Asking again every frame would be a retune per frame.
    let vfo = state.active_freq_hz();
    if (v_lo..=v_hi).contains(&vfo) {
        return;
    }
    cmds.push(Command::SetVfo { vfo: state.active_vfo, hz: (v_lo + v_hi) / 2.0 });
}

fn pan_center(
    dev_center: &mut f64,
    state: &mut RadioState,
    over: f64,
    pan: WindowPan,
    cmds: &mut Vec<Command>,
) {
    if !pan.movable || over == 0.0 {
        return;
    }
    let hz = (*dev_center + over).clamp(pan.limits.0, pan.limits.1);
    // The same half-hertz the engine calls "already there": a retune costs a
    // waterfall remap and, on some front ends, a skimmer restart.
    if (hz - *dev_center).abs() < 0.5 {
        return;
    }
    *dev_center = hz;
    state.center_hz = hz;
    cmds.push(Command::SetCenter(hz));
}

/// Slide the window so `vfo` sits in the middle of it, and report what the
/// captured span could not absorb — the amount the front end's own centre has
/// to move for the dial to be centred, which is nothing at all while the view
/// is still a viewport onto a wider window.
///
/// Below one display column the overshoot is reported as zero: a retune costs
/// a waterfall remap and, on some front ends, a skimmer restart, and a slide
/// narrower than a pixel is not visible to anyone. The view is still moved by
/// it, so the sub-pixel remainders accumulate rather than being lost and the
/// front end is asked once the total is worth asking for.
fn center_on_dial(
    view: &mut ViewState,
    vfo: f64,
    dev_center: f64,
    dev_span: f64,
    width_px: f32,
) -> f64 {
    let dhz = vfo - (view.view_lo_hz + view.view_hi_hz) / 2.0;
    let over = pan_view(view, dhz, dev_center, dev_span);
    let column = if width_px > 1.0 { dev_span / width_px as f64 } else { 0.0 };
    if over.abs() >= column { over } else { 0.0 }
}

/// The sub receiver's colour, on the panadapter and on its control module.
/// Deliberately nothing like the main receiver's red, the amber VFO/RTTY
/// markers, or the cyan the digital modes use: two receivers on one waterfall
/// are only readable if telling them apart never needs a second look.
pub const SUB_COLOR: Color32 = Color32::from_rgb(185, 130, 255);

/// Per-frame display tuning from the app. Both the waterfall advance and the
/// time gridlines are driven by the same wall-clock so they scroll in lockstep
/// (the app converts elapsed time × `rows_per_sec` into `rows_to_write`).
#[derive(Clone, Copy)]
pub struct WfTuning {
    /// Waterfall rows to append this frame (from elapsed wall-clock time).
    pub rows_to_write: u32,
    /// The operator's scroll rate in rows/second, before [`Self::row_scale`].
    /// The gridlines are spaced from it, so the time axis and the waterfall
    /// track each other exactly at any zoom factor.
    pub rows_per_sec: f32,
    /// History rows per egui point — the display's pixels per point, so a row
    /// is one device pixel tall rather than two on a HiDPI panel. Both the row
    /// rate and the rows on screen carry it, which leaves the seconds in view
    /// and the scroll speed unchanged. Never below 1.
    pub row_scale: f32,
    /// Columns the history texture should hold, from the client's own reading
    /// of this machine and this screen — see `SdroxideApp::panadapter_bins`.
    pub tex_w: u32,
    /// Wall-clock UTC seconds, for the minute-boundary time labels.
    pub now_unix: f64,
    /// EMA coefficient for the spectrum *line* only (1.0 = no smoothing). The
    /// waterfall uses the raw un-averaged frames, so the line's update speed is
    /// decoupled from the waterfall detail.
    pub spectrum_alpha: f32,
    /// Waterfall colour-palette index (from `UiSettings`).
    pub palette: usize,
    /// Rows a second the 3D spectrum flows away from the viewer, from the SPEC
    /// popup's **flow** row. Zero while the stream is stalled, which is what
    /// holds the surface still — the same rule that stops the waterfall
    /// scrolling rather than filling it with rows of a spectrum nobody is
    /// measuring any more.
    pub surface_rows_per_sec: f32,
    /// Optional vertical gradient `(top, bottom)` filling the spectrum area,
    /// `None` when disabled in the UI settings.
    pub gradient: Option<(Color32, Color32)>,
    /// Which radio's GPU waterfall history this panadapter scrolls. Radios are
    /// keyed apart so two tabs never write into each other's texture.
    pub wf_id: u64,
}

/// Slide a per-bin state array along with a window that has moved, and say
/// whether anything was left to slide.
///
/// A front end whose centre has moved is not showing a new picture: the span
/// and the bin count are the same, so the bins are the same width and the move
/// is a whole number of them. Everything still inside the window keeps its
/// history, and only the strip that has just scrolled in is taken raw from the
/// frame — the one place that side of the band exists at all.
///
/// This is what a panadapter drag does once the view is the whole span: the
/// window follows the gesture (issue #133), so the centre changes on *every*
/// frame for as long as the drag lasts. Treating each of those as a fresh
/// mapping threw the smoothing and the peak hold away sixty times a second,
/// and the trace went from smoothed to raw for the length of the drag
/// (issue #177).
///
/// Returns false when the window has moved further than its own width and
/// there is nothing to keep; the caller then starts again from the frame.
fn slide_bins(bins: &mut [f32], from_center: f64, f: &SpectrumFrame) -> bool {
    let n = bins.len();
    if n == 0 || n != f.bins.len() || f.span_hz <= 0.0 {
        return false;
    }
    let bin_hz = f.span_hz / n as f64;
    let d = ((f.center_hz - from_center) / bin_hz).round();
    if d.abs() >= n as f64 {
        return false;
    }
    // Bin `j` on the new axis is where bin `j + d` was on the old one.
    let d = d as isize;
    if d > 0 {
        let d = d as usize;
        bins.copy_within(d.., 0);
        for (b, &raw) in bins[n - d..].iter_mut().zip(&f.bins[n - d..]) {
            *b = raw as f32;
        }
    } else if d < 0 {
        let d = -d as usize;
        bins.copy_within(..n - d, d);
        for (b, &raw) in bins[..d].iter_mut().zip(&f.bins[..d]) {
            *b = raw as f32;
        }
    }
    true
}

/// Exponentially-smoothed spectrum for the trace line, folded once per new
/// frame. Kept separate from the (un-averaged) frame the waterfall consumes so
/// the "spectrum update speed" setting never blurs the waterfall.
#[derive(Default)]
pub struct SpectrumSmooth {
    bins: Vec<f32>,
    center: f64,
    span: f64,
    last_seq: Option<u32>,
    generation: u32,
    alpha: f32,
}

impl SpectrumSmooth {
    fn update(&mut self, f: &SpectrumFrame, alpha: f32) {
        if (self.alpha - alpha).abs() > 1e-6 {
            self.alpha = alpha;
            self.generation = self.generation.wrapping_add(1);
        }
        if self.last_seq == Some(f.seq) {
            return; // same frame redrawn
        }
        self.last_seq = Some(f.seq);
        let rescaled =
            self.bins.len() != f.bins.len() || (self.span - f.span_hz).abs() > f.span_hz * 1e-6;
        let moved = (self.center - f.center_hz).abs() > f.span_hz * 1e-6;
        // A window that has only moved keeps its average; see [`slide_bins`].
        let mapping_changed = rescaled || (moved && !slide_bins(&mut self.bins, self.center, f));
        self.center = f.center_hz;
        if mapping_changed || alpha >= 0.999 {
            self.span = f.span_hz;
            self.bins = f.bins.iter().map(|&b| b as f32).collect();
            if mapping_changed {
                self.generation = self.generation.wrapping_add(1);
            }
            return;
        }
        for (s, &b) in self.bins.iter_mut().zip(&f.bins) {
            *s += alpha * (b as f32 - *s);
        }
    }
}

// --- skimmer spot boxes ---------------------------------------------------
// The box sizes and font points here are the `Medium` step of the skimmer
// font-size setting; every use multiplies by `theme::skimmer_font_scale()`,
// so the boxes and their text grow and shrink together.
/// Widest a box may grow; a CW message tail longer than this scrolls.
const SPOT_BOX_W: f32 = 236.0;
const SPOT_BOX_H: f32 = 19.0;
/// Font size for the box's callsign / message text.
const SPOT_CALL_PT: f32 = 13.0;
const SPOT_MSG_PT: f32 = 12.5;
/// Horizontal padding inside a box.
const SPOT_PAD: f32 = 5.0;
/// Vertical gap between staggered lanes.
const SPOT_LANE_GAP: f32 = 3.0;
/// Gap from the newest edge of the waterfall to the first lane.
const SPOT_TOP_MARGIN: f32 = 4.0;
/// Minimum horizontal gap between two boxes sharing a lane.
const SPOT_H_GAP: f32 = 6.0;
/// Horizontal leader length from the signal to the box's left edge.
const SPOT_LEADER: f32 = 16.0;
/// Cap on stacked lanes; spots that would need a deeper lane are dropped.
const SPOT_MAX_LANES: usize = 6;

/// A laid-out skimmer box: its screen rect, the x of the signal it belongs to
/// (the box sits to its right, joined by a leader), and the spot's index.
struct SpotBox {
    rect: Rect,
    sig_x: f32,
    idx: usize,
}

/// Border/leader colour for a spot: cyan when hovered, dim-cyan once a
/// callsign is known, grey otherwise.
fn spot_color(spot: &SkimmerSpot, hovered: bool) -> Color32 {
    if hovered {
        crate::theme::scope().accent
    } else if spot.callsign.is_some() {
        crate::theme::scope().accent_dim
    } else {
        crate::theme::scope_gray(78)
    }
}

/// The on-screen width a box needs to hold its content, capped at
/// [`SPOT_BOX_W`]. In `callsign_only` mode (FT8) that is just the callsign;
/// otherwise (CW) the callsign plus the current message tail, so the box hugs
/// its text and the tail only starts scrolling once the box is genuinely full.
fn spot_content_width(p: &egui::Painter, spot: &SkimmerSpot, callsign_only: bool) -> f32 {
    let fs = crate::theme::skimmer_font_scale();
    let mut w = 2.0 * SPOT_PAD;
    if let Some(call) = &spot.callsign {
        w += p
            .layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT * fs), Color32::WHITE)
            .size()
            .x;
    }
    if !callsign_only {
        if spot.callsign.is_some() {
            w += 6.0;
        }
        let text = if spot.text.is_empty() { "…" } else { spot.text.as_str() };
        w += p
            .layout_no_wrap(text.to_string(), FontId::monospace(SPOT_MSG_PT * fs), Color32::WHITE)
            .size()
            .x;
    }
    w.clamp(30.0, SPOT_BOX_W * fs)
}

/// Lay skimmer spots out into staggered lanes over the waterfall. Each box sits
/// to the right of its signal (offset by a leader). Visible spots are sorted by
/// x and greedily packed into the lowest lane whose footprint — from the signal
/// x through the box's right edge — clears the previous box, so nearby signals
/// stack vertically instead of overlapping. Off-view / past-cap spots are omitted.
///
/// Boxes are sized to their content (capped at [`SPOT_BOX_W`]): in
/// `callsign_only` mode (FT8 station boxes) that is just the callsign;
/// otherwise the callsign plus the CW skimmer's live-growing message tail.
fn layout_spots(
    p: &egui::Painter,
    view: &ViewState,
    rect: &Rect,
    wf_rect: &Rect,
    spots: &[SkimmerSpot],
    callsign_only: bool,
) -> Vec<SpotBox> {
    let mut vis: Vec<(f32, usize)> = spots
        .iter()
        .enumerate()
        .filter(|(_, s)| (view.view_lo_hz..=view.view_hi_hz).contains(&s.freq_hz))
        .map(|(i, s)| (view.freq_to_x(s.freq_hz, rect), i))
        .collect();
    vis.sort_by(|a, b| a.0.total_cmp(&b.0));

    let box_h = SPOT_BOX_H * crate::theme::skimmer_font_scale();
    let mut lane_right: Vec<f32> = Vec::new();
    let mut out = Vec::with_capacity(vis.len());
    for (xc, idx) in vis {
        let box_w = spot_content_width(p, &spots[idx], callsign_only);
        let box_left = xc + SPOT_LEADER;
        // Footprint spans the signal tick through the box's right edge, so a
        // later box's tick can't land on top of an earlier box in the lane.
        let foot_right = box_left + box_w + SPOT_H_GAP;
        let mut lane = lane_right.len();
        for (k, &r) in lane_right.iter().enumerate() {
            if xc >= r {
                lane = k;
                break;
            }
        }
        if lane >= SPOT_MAX_LANES {
            continue; // too crowded here — drop the box
        }
        if lane == lane_right.len() {
            lane_right.push(0.0);
        }
        lane_right[lane] = foot_right;
        // Lanes stack away from the newest row, so the boxes sit with the fresh
        // signals they label whichever way the waterfall scrolls.
        let lane_off = SPOT_TOP_MARGIN + lane as f32 * (box_h + SPOT_LANE_GAP);
        let top = if view.waterfall_flip {
            wf_rect.bottom() - lane_off - box_h
        } else {
            wf_rect.top() + lane_off
        };
        out.push(SpotBox {
            rect: Rect::from_min_size(pos2(box_left, top), vec2(box_w, box_h)),
            sig_x: xc,
            idx,
        });
    }
    out
}

/// Scale a colour's alpha by `a` (for fading spots out).
fn fade(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a).round() as u8)
}

/// Draw one skimmer box at opacity `alpha` (1 = solid, 0 = gone). In
/// `callsign_only` mode (FT8/FT4) just the callsign is shown — green when the
/// station is calling CQ, white otherwise; otherwise the CW layout is used
/// (green callsign + rolling message).
fn draw_spot_box(
    p: &egui::Painter,
    b: &SpotBox,
    spot: &SkimmerSpot,
    hovered: bool,
    alpha: f32,
    callsign_only: bool,
) {
    let rect = b.rect;
    p.rect_filled(rect, 2.0, fade(Color32::from_rgba_unmultiplied(5, 11, 18, 225), alpha));
    let border = spot_color(spot, hovered);
    p.rect_stroke(
        rect,
        2.0,
        Stroke::new(if hovered { 1.5 } else { 1.0 }, fade(border, alpha)),
        StrokeKind::Inside,
    );

    let fs = crate::theme::skimmer_font_scale();
    let pad = SPOT_PAD;
    let cy = rect.center().y;

    if callsign_only {
        if let Some(call) = &spot.callsign {
            // FT8 message text starts with "CQ" for callers; colour those green.
            let base = if spot.text.starts_with("CQ") {
                crate::theme::scope().good
            } else {
                Color32::WHITE
            };
            let cc = fade(base, alpha);
            let g = p.layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT * fs), cc);
            p.galley(pos2(rect.left() + pad, cy - g.size().y * 0.5), g, cc);
        }
        return;
    }

    let mut x = rect.left() + pad;
    if let Some(call) = &spot.callsign {
        let cc = fade(crate::theme::scope().good, alpha);
        let g = p.layout_no_wrap(call.clone(), FontId::monospace(SPOT_CALL_PT * fs), cc);
        p.galley(pos2(x, cy - g.size().y * 0.5), g.clone(), cc);
        x += g.size().x + 6.0;
    }

    // Message area: whatever width is left of the box. The message is anchored
    // to the right edge and clipped on the left, so the newest decoded text is
    // always shown (older text slides off the left as it arrives) — steadier to
    // read than a marquee.
    let msg_rect = Rect::from_min_max(pos2(x, rect.top()), pos2(rect.right() - pad, rect.bottom()));
    if msg_rect.width() < 6.0 {
        return;
    }
    let text = if spot.text.is_empty() { "…" } else { spot.text.as_str() };
    let col = fade(crate::theme::scope().ink, alpha);
    let g = p.layout_no_wrap(text.to_string(), FontId::monospace(SPOT_MSG_PT * fs), col);
    let ty = cy - g.size().y * 0.5;
    // Left-align while it fits; once it overflows, pin the tail to the right.
    let gx = if g.size().x <= msg_rect.width() {
        msg_rect.left()
    } else {
        msg_rect.right() - g.size().x
    };
    let mp = p.with_clip_rect(msg_rect);
    mp.galley(pos2(gx, ty), g, col);
}

// --- network spot markers (DX cluster / POTA / SOTA / PSK Reporter) --------
// Anchored at the oldest edge of the waterfall (its floor, or its top when the
// waterfall is flipped) so they stay clear of the skimmer / FT8 boxes, which
// hug the newest edge. Sized by the skimmer font-size setting like those —
// the two sets are the same kind of callsign box and must read as one family.
const NET_BOX_H: f32 = 18.0;
const NET_CALL_PT: f32 = 12.5;
const NET_TAG_PT: f32 = 9.5;
const NET_PAD: f32 = 5.0;
const NET_LANE_GAP: f32 = 3.0;
const NET_BOTTOM_MARGIN: f32 = 4.0;
const NET_H_GAP: f32 = 6.0;
const NET_LEADER: f32 = 14.0;
const NET_MAX_LANES: usize = 5;

/// A laid-out network-spot box (mirrors [`SpotBox`] but bottom-anchored).
struct NetBox {
    rect: Rect,
    sig_x: f32,
    idx: usize,
}

fn net_spot_width(p: &egui::Painter, spot: &Spot) -> f32 {
    let fs = crate::theme::skimmer_font_scale();
    let mut w = 2.0 * NET_PAD;
    w += p
        .layout_no_wrap(spot.call.clone(), FontId::monospace(NET_CALL_PT * fs), Color32::WHITE)
        .size()
        .x;
    w += 6.0
        + p.layout_no_wrap(
            spot.kind.label().to_string(),
            FontId::monospace(NET_TAG_PT * fs),
            Color32::WHITE,
        )
        .size()
        .x;
    w.clamp(40.0, 220.0 * fs)
}

/// One ISM device to label on the waterfall.
///
/// A slim type of its own rather than the whole [`sdroxide_types::IsmReport`]:
/// what goes in the box is a formatted line, and formatting readings into units
/// belongs with the rest of the panel's formatting rather than in the painter.
pub struct IsmLabel {
    /// Measured RF frequency of the device.
    pub freq_hz: f64,
    /// Already formatted — protocol, identity and reading.
    pub text: String,
    /// The frame validated but its payload is encrypted, so it is tinted like a
    /// warning rather than like a reading.
    pub encrypted: bool,
}

const ISM_LANE_GAP: f32 = 3.0;
const ISM_H_GAP: f32 = 6.0;
const ISM_LEADER: f32 = 14.0;
const ISM_MAX_LANES: usize = 4;
const ISM_PAD_X: f32 = 5.0;
const ISM_PAD_Y: f32 = 3.0;
const ISM_PT: f32 = 11.5;

/// Widest an ISM label may grow before it wraps onto another line.
///
/// A decoded weather station says a lot — temperature, humidity, direction,
/// gust, average, rain, light and UV is eight readings — and on one line that is
/// wider than most panadapters. It used to be clamped to this width and then
/// painted at full length anyway, so the text simply ran out of its own box and
/// across the waterfall. Wrapping is the honest version of that clamp.
const ISM_BOX_MAX_W: f32 = 300.0;

struct IsmBox {
    rect: Rect,
    sig_x: f32,
    idx: usize,
    /// Laid out once during placement, because the wrapped height is what decides
    /// where the next lane starts.
    galley: Arc<egui::Galley>,
}

/// Lay visible ISM labels into staggered lanes at the *newest* waterfall edge,
/// stacked past where the skimmer's boxes end.
///
/// Both edges of the waterfall are already claimed — the skimmer's spot boxes at
/// the newest, the network spots at the oldest — so these start below the
/// skimmer's lanes rather than beside them. In practice the two never appear
/// together, because the skimmers are gated to the HF band segments and there are
/// none at 868 MHz; the offset is there so that if they ever do, they stack
/// instead of overprinting.
fn layout_ism_labels(
    p: &egui::Painter,
    view: &ViewState,
    rect: &Rect,
    wf_rect: &Rect,
    labels: &[IsmLabel],
) -> Vec<IsmBox> {
    let fs = crate::theme::skimmer_font_scale();
    let mut vis: Vec<(f32, usize)> = labels
        .iter()
        .enumerate()
        .filter(|(_, l)| (view.view_lo_hz..=view.view_hi_hz).contains(&l.freq_hz))
        .map(|(i, l)| (view.freq_to_x(l.freq_hz, rect), i))
        .collect();
    vis.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Two passes, because a wrapped label's height is not known until it has been
    // laid out and a lane cannot be positioned until every box in the lane above
    // it has been. First: wrap each label, assign it a lane, and record how tall
    // that lane has to be.
    let mut lane_right: Vec<f32> = Vec::new();
    let mut lane_h: Vec<f32> = Vec::new();
    let mut placed: Vec<(usize, usize, f32, Arc<egui::Galley>)> = Vec::with_capacity(vis.len());
    for (xc, idx) in vis {
        // Laid out with a placeholder colour so the tint can be chosen at paint
        // time — a hovered box is drawn brighter than a resting one.
        let galley = p.layout(
            labels[idx].text.clone(),
            FontId::proportional(ISM_PT * fs),
            Color32::PLACEHOLDER,
            ISM_BOX_MAX_W * fs,
        );
        let box_w = galley.size().x.max(40.0) + ISM_PAD_X * 2.0;
        let box_h = galley.size().y + ISM_PAD_Y * 2.0;

        let box_left = xc + ISM_LEADER;
        let foot_right = box_left + box_w + ISM_H_GAP;
        let mut lane = lane_right.len();
        for (k, &r) in lane_right.iter().enumerate() {
            if xc >= r {
                lane = k;
                break;
            }
        }
        if lane >= ISM_MAX_LANES {
            continue;
        }
        if lane == lane_right.len() {
            lane_right.push(0.0);
            lane_h.push(0.0);
        }
        lane_right[lane] = foot_right;
        lane_h[lane] = lane_h[lane].max(box_h);
        placed.push((idx, lane, box_left, galley));
    }

    // Second: each lane starts below the tallest box in every lane above it, so a
    // three-line label pushes the next lane down instead of being written over.
    let mut lane_top: Vec<f32> = Vec::with_capacity(lane_h.len());
    let mut run = SPOT_TOP_MARGIN + SPOT_MAX_LANES as f32 * (SPOT_BOX_H * fs + SPOT_LANE_GAP);
    for h in &lane_h {
        lane_top.push(run);
        run += h + ISM_LANE_GAP;
    }

    let mut out = Vec::with_capacity(placed.len());
    for (idx, lane, box_left, galley) in placed {
        let size =
            vec2(galley.size().x.max(40.0) + ISM_PAD_X * 2.0, galley.size().y + ISM_PAD_Y * 2.0);
        // Past the skimmer's lanes, at whichever end of the waterfall is newest.
        let top = if view.waterfall_flip {
            wf_rect.bottom() - lane_top[lane] - size.y
        } else {
            wf_rect.top() + lane_top[lane]
        };
        out.push(IsmBox {
            rect: Rect::from_min_size(pos2(box_left, top), size),
            sig_x: view.freq_to_x(labels[idx].freq_hz, rect),
            idx,
            galley,
        });
    }
    out
}

/// Draw one ISM label box: the formatted device line, tinted green for a reading
/// and amber for a frame whose payload is encrypted.
fn draw_ism_box(p: &egui::Painter, b: &IsmBox, label: &IsmLabel, hovered: bool) {
    let tint = if label.encrypted { crate::theme::YELLOW() } else { crate::theme::GREEN() };
    let border = if hovered {
        tint
    } else {
        Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 210)
    };
    p.rect_filled(b.rect, 2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 200));
    p.rect_stroke(b.rect, 2.0, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    // The galley was wrapped during layout, so this paints however many lines it
    // came to — the box was sized around exactly that.
    p.galley(pos2(b.rect.left() + ISM_PAD_X, b.rect.top() + ISM_PAD_Y), b.galley.clone(), border);
}

/// Kind tint, brightened on hover.
fn net_spot_color(spot: &Spot, hovered: bool) -> Color32 {
    let (r, g, b) = crate::theme::spot_color(spot.kind);
    if hovered { Color32::from_rgb(r, g, b) } else { Color32::from_rgba_unmultiplied(r, g, b, 225) }
}

/// Lay visible network spots into bottom-anchored staggered lanes.
fn layout_net_spots(
    p: &egui::Painter,
    view: &ViewState,
    rect: &Rect,
    wf_rect: &Rect,
    spots: &[Spot],
) -> Vec<NetBox> {
    let mut vis: Vec<(f32, usize)> = spots
        .iter()
        .enumerate()
        .filter(|(_, s)| (view.view_lo_hz..=view.view_hi_hz).contains(&s.freq_hz))
        .map(|(i, s)| (view.freq_to_x(s.freq_hz, rect), i))
        .collect();
    vis.sort_by(|a, b| a.0.total_cmp(&b.0));

    let box_h = NET_BOX_H * crate::theme::skimmer_font_scale();
    let mut lane_right: Vec<f32> = Vec::new();
    let mut out = Vec::with_capacity(vis.len());
    for (xc, idx) in vis {
        let box_w = net_spot_width(p, &spots[idx]);
        let box_left = xc + NET_LEADER;
        let foot_right = box_left + box_w + NET_H_GAP;
        let mut lane = lane_right.len();
        for (k, &r) in lane_right.iter().enumerate() {
            if xc >= r {
                lane = k;
                break;
            }
        }
        if lane >= NET_MAX_LANES {
            continue;
        }
        if lane == lane_right.len() {
            lane_right.push(0.0);
        }
        lane_right[lane] = foot_right;
        // Anchored at the *oldest* edge, opposite the skimmer boxes, so the two
        // sets never collide — which end that is depends on the flip.
        let lane_off = NET_BOTTOM_MARGIN + lane as f32 * (box_h + NET_LANE_GAP);
        let top = if view.waterfall_flip {
            wf_rect.top() + lane_off
        } else {
            wf_rect.bottom() - lane_off - box_h
        };
        out.push(NetBox {
            rect: Rect::from_min_size(pos2(box_left, top), vec2(box_w, box_h)),
            sig_x: xc,
            idx,
        });
    }
    out
}

/// Draw one network-spot box at opacity `alpha`: callsign in the kind colour,
/// then a small kind badge (DX/POTA/SOTA/PSK).
fn draw_net_box(p: &egui::Painter, b: &NetBox, spot: &Spot, hovered: bool, alpha: f32) {
    let rect = b.rect;
    p.rect_filled(rect, 2.0, fade(Color32::from_rgba_unmultiplied(6, 12, 20, 232), alpha));
    let border = net_spot_color(spot, hovered);
    p.rect_stroke(
        rect,
        2.0,
        Stroke::new(if hovered { 1.5 } else { 1.0 }, fade(border, alpha)),
        StrokeKind::Inside,
    );
    let fs = crate::theme::skimmer_font_scale();
    let cy = rect.center().y;
    let mut x = rect.left() + NET_PAD;
    let call_col = fade(border, alpha);
    let g = p.layout_no_wrap(spot.call.clone(), FontId::monospace(NET_CALL_PT * fs), call_col);
    p.galley(pos2(x, cy - g.size().y * 0.5), g.clone(), call_col);
    x += g.size().x + 6.0;
    let tag_col = fade(crate::theme::scope_gray(180), alpha);
    let tg = p.layout_no_wrap(
        spot.kind.label().to_string(),
        FontId::monospace(NET_TAG_PT * fs),
        tag_col,
    );
    if x + tg.size().x <= rect.right() - NET_PAD {
        p.galley(pos2(x, cy - tg.size().y * 0.5), tg, tag_col);
    }
}

/// Tune the active VFO onto a network spot and set its mode. A spot names the
/// frequency the signal is *on*, so the modes that listen at an audio offset
/// from the dial are dialed that offset below it: CW by a sidetone-pitch, RTTY
/// by its tone pair (as click-tune on a skimmer spot does). Everything else —
/// phone, and the digital modes whose spots quote the agreed dial rather than
/// a station's tone — goes straight onto the spotted frequency.
///
/// [`Command::TuneInSpan`], like every click on the picture: the receiver
/// moves onto the spot, and a radio that tunes with its dial comes along
/// exactly as it does for the readout (see the command's docs for why it no
/// longer stays put).
fn tune_to_net_spot(spot: &Spot, state: &RadioState, cmds: &mut Vec<Command>) {
    match spot.radio_mode() {
        Some(Mode::Cw) => {
            let (lo, hi) = Mode::Cw.default_filter();
            let pitch = ((lo + hi) * 0.5) as f64;
            cmds.push(Command::TuneInSpan { vfo: state.active_vfo, hz: spot.freq_hz - pitch });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
        }
        Some(m) if m.holds_standard_tones() => {
            let af = m.standard_tone_offset_hz().unwrap_or_default();
            cmds.push(Command::TuneInSpan {
                vfo: state.active_vfo,
                hz: spot.freq_hz - f64::from(af),
            });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
            cmds.push(Command::SetDigiAudioFreq(af));
        }
        Some(m) => {
            cmds.push(Command::TuneInSpan { vfo: state.active_vfo, hz: spot.freq_hz });
            cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
        }
        None => cmds.push(Command::TuneInSpan { vfo: state.active_vfo, hz: spot.freq_hz }),
    }
}

/// Decaying peak-hold trace, reset whenever the frame mapping changes.
#[derive(Default)]
pub struct PeakHold {
    bins: Vec<f32>,
    center: f64,
    span: f64,
    /// Sequence of the frame last folded in, so decay runs once per *new*
    /// frame rather than once per repaint (repaint rate must not change the
    /// decay speed).
    last_seq: Option<u32>,
    /// Bumped whenever `bins` change outside the seq progression (mapping
    /// change, clear) — part of the trace-cache key.
    generation: u32,
}

/// dB-scale u8 units per new spectrum frame (~3.5 dB/s at 30 fps over a
/// 100 dB range).
const PEAK_DECAY: f32 = 0.3;

impl PeakHold {
    fn update(&mut self, f: &SpectrumFrame) {
        if self.last_seq == Some(f.seq) {
            return; // same frame redrawn — nothing new to fold in
        }
        self.last_seq = Some(f.seq);
        let rescaled =
            self.bins.len() != f.bins.len() || (self.span - f.span_hz).abs() > f.span_hz * 1e-6;
        let moved = (self.center - f.center_hz).abs() > f.span_hz * 1e-6;
        // The held peaks travel with the window too — see [`slide_bins`].
        let mapping_changed = rescaled || (moved && !slide_bins(&mut self.bins, self.center, f));
        self.center = f.center_hz;
        if mapping_changed {
            self.span = f.span_hz;
            self.bins = f.bins.iter().map(|&b| b as f32).collect();
            self.generation = self.generation.wrapping_add(1);
            return;
        }
        for (p, &b) in self.bins.iter_mut().zip(&f.bins) {
            *p = (b as f32).max(*p - PEAK_DECAY);
        }
    }

    fn clear(&mut self) {
        self.bins.clear();
        self.last_seq = None;
        self.generation = self.generation.wrapping_add(1);
    }
}

/// Screen-space spectrum polylines, recomputed only when the underlying frame,
/// viewport, or rect actually change — pure repaints reuse the cached points.
#[derive(Default)]
pub struct TraceCache {
    live: TraceEntry,
    hold: TraceEntry,
}

#[derive(Default)]
struct TraceEntry {
    key: Option<TraceKey>,
    points: Vec<Pos2>,
}

/// Everything the per-pixel trace math depends on. Float fields are compared
/// as exact bits — any change must invalidate. (`db_floor`/`db_ceil` need no
/// entry: they reach the trace via the engine's u8 mapping, i.e. a new seq.)
#[derive(Clone, Copy, PartialEq)]
struct TraceKey {
    seq: u32,
    generation: u32,
    view_lo: u64,
    view_hi: u64,
    /// The zoom factor the polyline was sampled at: the trace is one vertex per
    /// device pixel, so moving the window to a screen with a different one has
    /// to recompute it.
    ppp: u32,
    rect: [u32; 4],
}

fn trace_key(
    f: &SpectrumFrame,
    generation: u32,
    view: &ViewState,
    rect: &Rect,
    ppp: f32,
) -> TraceKey {
    TraceKey {
        seq: f.seq,
        generation,
        view_lo: view.view_lo_hz.to_bits(),
        view_hi: view.view_hi_hz.to_bits(),
        ppp: ppp.to_bits(),
        rect: [
            rect.left().to_bits(),
            rect.top().to_bits(),
            rect.right().to_bits(),
            rect.bottom().to_bits(),
        ],
    }
}

impl TraceEntry {
    /// Recompute the polyline only when `key` changed; hand back a copy for
    /// `Shape::line` (a memcpy — far cheaper than the per-pixel f64 math).
    fn points_for(&mut self, key: TraceKey, compute: impl FnOnce() -> Vec<Pos2>) -> Vec<Pos2> {
        if self.key != Some(key) {
            self.points = compute();
            self.key = Some(key);
        }
        self.points.clone()
    }
}

/// An audio-offset cursor on the panadapter: a marker at `dial + hz`, and what
/// a left-click on the waterfall means while it is there.
#[derive(Clone, Copy, Debug)]
pub struct AudioCursor {
    /// Offset from the dial, in Hz.
    pub hz: f32,
    /// `true` — a click sets the offset. The digital modes, where the dial is
    /// parked on a sub-band and the operator picks a signal *within* it.
    ///
    /// `false` — a click tunes the dial so that the clicked frequency lands on
    /// the cursor. CW, where the panadapter spans the whole band: the cursor is
    /// the pitch being listened at, and clicking a signal means "put that one
    /// here", not "move my pitch to the far side of the passband".
    pub click_sets_offset: bool,
    /// Draw the receiver's tuning line on the cursor rather than on the dial.
    ///
    /// The dial in CW sits a sidetone pitch below the signal, so the tuning
    /// line lands at the *edge* of a passband the engine has already centred
    /// on the pitch — the line, the shading and the signal all disagree about
    /// where the receiver is pointed. With this set the line moves onto the
    /// cursor, where the passband is already centred and the signal already
    /// is, and the three finally say the same thing.
    ///
    /// Display only: the dial is unchanged and everything that tunes, stores
    /// or checks a band edge still uses it.
    pub line_on_cursor: bool,
}

/// The panadapter: spectrum trace, frequency scale and waterfall, with the
/// tuning, filter and spot interactions on them.
///
/// `cursor` is the optional audio-offset marker — see [`AudioCursor`]. `skimmer`
/// are the overlay boxes (CW skimmer spots, or FT8 station callsigns in digital
/// mode) and `alpha` is a parallel per-box opacity for fade-out.
pub fn show_ext(
    ui: &mut Ui,
    view: &mut ViewState,
    state: &mut RadioState,
    frame: Option<&Arc<SpectrumFrame>>,
    peaks: &mut PeakHold,
    smooth: &mut SpectrumSmooth,
    trace: &mut TraceCache,
    // The remembered spectra behind the 3D surface, when the SPEC popup's 3D
    // chip is lit. Kept by the app rather than here for the same reason the
    // smoothing and the peak hold are: it is history, and a widget redrawn
    // from scratch every frame has nowhere to keep any.
    surface: &mut crate::widgets::spectrum3d::Surface,
    cursor: Option<AudioCursor>,
    // FT8 DXpedition role. Anything but `Normal` shades the two halves of the
    // passband the pile-up divides itself into, with the half we operate in
    // named — a Hound calling down among the Fox's signals is the single most
    // common way to be unworkable, and it is invisible without this.
    dxped: sdroxide_types::DxpedMode,
    // FT8/FT4: the engine is choosing the transmit frequency, so clicking a
    // station label must not drag it onto that station.
    auto_tx_freq: bool,
    // FT8/FT4: the operator has pinned the transmit tone, so NO click moves it
    // — a station label or bare waterfall alike. The engine refuses the command
    // regardless; suppressing it here keeps the UI from asking for something it
    // has been told cannot happen.
    hold_tx_freq: bool,
    // Extra audio-offset tuning lines (Hz relative to the dial), e.g. the RTTY
    // mark/space pair. Drawn as thin amber lines to aid exact tuning.
    markers: &[f32],
    skimmer: &[SkimmerSpot],
    alpha: &[f32],
    // Network spots (DX cluster / POTA / SOTA / PSK) with a parallel fade alpha.
    // On click, the clicked spot is returned via `clicked_spot` (and the VFO is
    // tuned + mode set here) so the app can pre-fill a log entry.
    net_spots: &[Spot],
    net_alpha: &[f32],
    clicked_spot: &mut Option<Spot>,
    // ISM devices heard on the visible span, labelled on the waterfall. Not
    // clickable: the ISM window is where a device is acted on, and a box here is
    // a "this one, there" annotation.
    ism: &[IsmLabel],
    // Stored memories whose dial lands on the visible span, marked along the
    // waterfall's oldest edge under the band-plan strip. Like the ISM labels,
    // an annotation and not a control: the memory list is where a channel is
    // recalled from.
    mem: &[crate::widgets::memories::MemMark],
    // Operator's pointer preferences: what the wheel does (plain and with
    // Shift), whether left-drag tunes, and the click-tune rounding.
    wheel: WheelSettings,
    // What the front end will let a pan that has run off the end of the
    // captured window do to the window itself. See [`WindowPan`].
    pan: WindowPan,
    wf: WfTuning,
    // The mode's operating panel is on screen under the panadapter, so the
    // waterfall is only part of the height. Keeps the bandplan strip from
    // eating what little of it is left — see `bandplan::overlay`.
    panel_below: bool,
    cmds: &mut Vec<Command>,
) {
    let cursor_hz = cursor.map(|c| c.hz);
    // True only where a click *sets* the offset. In CW a click tunes the dial
    // instead, so everything keyed to "is there a cursor" has to ask which kind.
    let click_sets_offset = cursor.is_some_and(|c| c.click_sets_offset);

    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_gray(8));

    let Some(f) = frame.filter(|f| f.span_hz > 0.0 && !f.bins.is_empty()) else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "waiting for spectrum…",
            FontId::proportional(16.0),
            Color32::GRAY,
        );
        return;
    };

    // The pan/zoom bounds are the full device passband (frames may cover
    // only the zoomed viewport). Mutable because a pan that reaches the edge of
    // the window moves the window — everything below has to be bounded by the
    // one being asked for, not the one being left.
    let mut dev_center = state.center_hz;
    let dev_span = state.sample_rate;
    // How far the view may zoom and pan: the full-band lane where the front end
    // has one, else the passband. Distinct from `dev_*` above, which stays the
    // I/Q and goes on bounding the things that really are limited by it — the
    // sub receiver's DDC, and the zoom-in resolution floor.
    let (zoom_center, zoom_span) = pan.view_bounds(dev_center, dev_span);
    // Both receivers are DDCs on this one IQ stream, so nothing outside these
    // edges can be tuned — the sub gets clamped to them.
    let dev_lo = dev_center - dev_span / 2.0;
    let dev_hi = dev_center + dev_span / 2.0;
    if view.is_unset() {
        view.fit(dev_center, dev_span);
    }

    // Layout first — interactions depend on which strip the pointer is in.
    // Either layer can be switched off in the SPEC popup: with the spectrum
    // hidden the waterfall has the lot, with the waterfall hidden the spectrum
    // does, and the frequency scale stays between them either way.
    // A phone draws the waterfall and nothing else: a spectrum trace in a
    // 360 pt-wide window is a strip too thin to read that costs the waterfall a
    // third of its height. The operator's own layer switches and
    // `spectrum_fraction` are only overridden here, never written, so a wider
    // window gets back exactly the split it was left with.
    let waterfall_only = crate::layout::tier(ui.ctx()).waterfall_only();
    let frac = if waterfall_only { 0.0 } else { view.effective_spectrum_fraction() };
    let usable = rect.height() - scale_h();
    let spec_h = if frac <= 0.0 { 0.0 } else { (usable * frac).max(40.0).min(usable.max(0.0)) };
    let spec_rect = Rect::from_min_size(rect.min, vec2(rect.width(), spec_h));
    let scale_rect =
        Rect::from_min_size(pos2(rect.left(), spec_rect.bottom()), vec2(rect.width(), scale_h()));
    // What the marks that annotate the *current* spectrum are drawn across —
    // the passband wash, the filter edge grips, the tuning lines. The whole
    // strip for the flat trace, and only its front plane for the 3D surface:
    // there the newest spectrum is the bottom of the picture and everything
    // above it is time already gone, so a line drawn the full height would cut
    // through rows the filter was never set on. See `spectrum3d::front_plane_h`.
    let spec_marks = if view.spectrum_3d {
        Rect::from_min_max(
            pos2(
                spec_rect.left(),
                spec_rect.bottom() - crate::widgets::spectrum3d::front_plane_h(spec_h),
            ),
            spec_rect.max,
        )
    } else {
        spec_rect
    };
    let wf_rect = Rect::from_min_max(pos2(rect.left(), scale_rect.bottom()), rect.max);

    // Where the spot lanes and their ticks go. Normally the waterfall, since
    // that is what they label; with the waterfall switched off they move onto
    // the spectrum rather than vanish — the boxes are the skimmers' only
    // output on the panadapter, and the painter clips to `rect`, so laid out
    // over a zero-height strip they would silently be gone.
    let lanes_rect = if wf_rect.height() > 1.0 { wf_rect } else { spec_rect };

    // Skimmer boxes are laid out up front so the click hit-test (below) and the
    // draw pass (bottom) agree on their rects. Boxes fit their text: callsign
    // only for FT8 (digital), callsign + message tail for the CW skimmer.
    let spot_boxes = layout_spots(&painter, view, &rect, &lanes_rect, skimmer, click_sets_offset);
    let net_boxes = layout_net_spots(&painter, view, &rect, &lanes_rect, net_spots);
    let ism_boxes = layout_ism_labels(&painter, view, &rect, &lanes_rect, ism);

    // --- interactions -----------------------------------------------------
    // Model: grabbing a filter edge (left button, spectrum strip) always
    // wins and only moves the edge. Otherwise: left-drag pans the view AND
    // drags the tuning with it; right-drag pans the view only.
    let vfo_hz = state.rx_freq_hz();
    let sub_hz = state.sub_rx_hz;
    let sub_on = state.sub_rx_enabled;
    let px0 = view.freq_to_x(vfo_hz + state.rx[0].filter_lo as f64, &rect);
    let px1 = view.freq_to_x(vfo_hz + state.rx[0].filter_hi as f64, &rect);
    let spx0 = view.freq_to_x(sub_hz + state.rx[1].filter_lo as f64, &rect);
    let spx1 = view.freq_to_x(sub_hz + state.rx[1].filter_hi as f64, &rect);
    let sub_marker_x = view.freq_to_x(sub_hz, &rect);

    // How close counts as grabbing an edge. A mouse lands where it is pointed;
    // a fingertip covers about nine millimetres, so a touched layout needs a
    // wider zone — but never wider than a third of the passband itself, or the
    // two edges of a 5 pt-wide SSB filter would meet in the middle and swallow
    // click-to-tune across the whole signal.
    let edge_grab =
        crate::layout::tier(ui.ctx()).grab_px().min(((px1 - px0).abs() / 3.0).max(EDGE_GRAB_PX));
    let marker_grab = edge_grab * (MARKER_GRAB_PX / EDGE_GRAB_PX);

    // Which passband edge the pointer can grab: the receiver it belongs to, and
    // whether it is the high edge.
    let edge_at = |p: egui::Pos2| -> Option<(RxId, bool)> {
        // Grabbable in either the spectrum or the waterfall strip, so the edges
        // can still be dragged when the spectrum line is collapsed.
        if !(spec_rect.contains(p) || wf_rect.contains(p)) {
            return None;
        }
        // Nearest edge wins, and the sub's edges break a tie (they come last):
        // where the two passbands overlap, the pointer means the receiver the
        // operator just parked there, not the one they have been on all along.
        let mut best = None;
        let mut best_d = edge_grab;
        for (x, rx, hi) in [
            (px0, RxId::Main, false),
            (px1, RxId::Main, true),
            (spx0, RxId::Sub, false),
            (spx1, RxId::Sub, true),
        ] {
            if rx == RxId::Sub && !sub_on {
                continue;
            }
            let d = (p.x - x).abs();
            if d <= best_d {
                best_d = d;
                best = Some((rx, hi));
            }
        }
        best
    };

    // Dragging inside a receiver's passband tunes that receiver. The main one
    // needs no special case — a left-drag anywhere already slides the spectrum
    // and takes the dial with it — so this only has to claim the sub's, plus
    // its tuning line, which in SSB sits outside the passband it opens onto.
    let sub_grab_at = |p: egui::Pos2| -> bool {
        if !sub_on || !(spec_rect.contains(p) || wf_rect.contains(p)) {
            return false;
        }
        (spx0.min(spx1)..=spx0.max(spx1)).contains(&p.x) || (p.x - sub_marker_x).abs() < marker_grab
    };

    let edge_id = ui.id().with("pb-edge");
    let mut edge: Option<(RxId, bool)> = ui.data(|d| d.get_temp(edge_id)).unwrap_or(None);
    let hover_edge = resp.hover_pos().and_then(edge_at);

    let sub_drag_id = ui.id().with("sub-drag");
    let mut sub_drag: bool = ui.data(|d| d.get_temp(sub_drag_id)).unwrap_or(false);
    let hover_sub = resp.hover_pos().map(sub_grab_at).unwrap_or(false) && hover_edge.is_none();

    // The frequency-scale strip doubles as the spectrum/waterfall resize grip
    // and as the frequency axis: a vertical drag there changes the spectrum
    // height, a sideways one slides the band along under the picture
    // (issue #174).
    let resize_id = ui.id().with("spec-resize");
    let mut resizing: bool = ui.data(|d| d.get_temp(resize_id)).unwrap_or(false);
    // Which of the two a strip drag turned out to be — `None` until it has
    // travelled far enough to have a direction, then `Some(true)` for the
    // sideways pan and `Some(false)` for the resize, and it keeps that for the
    // rest of the gesture. Without the lock a resize would shift the band every
    // time the pointer wandered and a pan would quietly rewrite the split.
    let scale_axis_id = ui.id().with("scale-axis");
    let mut scale_axis: Option<bool> = ui.data(|d| d.get_temp(scale_axis_id)).unwrap_or(None);
    let hover_resize =
        !waterfall_only && resp.hover_pos().map(|p| scale_rect.contains(p)).unwrap_or(false);

    // Shift+drag bandwidth measurement: remembers the frequency where the drag
    // started; `None` when not measuring.
    let measure_id = ui.id().with("bw-measure");
    let mut measuring: Option<f64> = ui.data(|d| d.get_temp(measure_id)).unwrap_or(None);
    // After releasing, the frozen span lingers and fades out over `BW_FADE_SECS`:
    // (start_hz, end_hz, release_time_secs).
    let fade_id = ui.id().with("bw-fade");
    let mut bw_fade: Option<(f64, f64, f64)> = ui.data(|d| d.get_temp(fade_id)).unwrap_or(None);

    // Dial inertia: a tuning drag released while still moving leaves the dial
    // coasting to a stop. Written back once, at the end of the interaction
    // handling below.
    let fling_id = ui.id().with("dial-fling");
    let mut fling: Option<Fling> = ui.data(|d| d.get_temp(fling_id)).unwrap_or(None);
    // Set when a press has stopped a coast, so the click it becomes is swallowed:
    // catching a spinning knob means "stop here", never "tune to where my hand
    // landed". Lives across the press/release frame pair.
    let stop_click_id = ui.id().with("fling-stop-click");
    let mut stop_click: bool = ui.data(|d| d.get_temp(stop_click_id)).unwrap_or(false);

    // The dial the gesture in progress is turning, as *this screen* has it.
    //
    // A tuning drag and the coast after it move the dial by a delta a frame,
    // and the obvious way to write that — read the dial out of the state, add
    // the delta, put it back — is a read-modify-write on a number the engine
    // keeps overwriting from behind. `RadioEvent::State` is the engine's whole
    // snapshot, and mid-drag it is its answer to a `SetVfo` sent some frames
    // ago; adding this frame's delta to *that* throws away every delta sent
    // since. The dial then advances one delta per round trip where the view —
    // which belongs to this screen and is echoed by nobody — advances one per
    // frame, so the picture slides out from under the marker at a whole
    // multiple of the right speed, and the marker splits into as many
    // interleaved positions as there are frames in the round trip. Two,
    // usually, which is what it looks like: the waterfall running at double
    // speed and the marker twitching between two places. Barely visible in the
    // shack, plain over a network — the browser client is where it was found.
    //
    // So the gesture keeps its own dial: seeded from the state when it starts,
    // carried across every frame it lasts, and dropped when it ends — by which
    // time the engine has had the last word, which is the right place to start
    // the next one from.
    let gesture_dial_id = ui.id().with("gesture-dial");
    let mut gesture_dial: Option<f64> = ui.data(|d| d.get_temp(gesture_dial_id)).unwrap_or(None);

    if resp.drag_started_by(egui::PointerButton::Primary) {
        // Decide from the PRESS position, not the current pointer position —
        // by the time the drag threshold trips, the pointer may already have
        // left the grab zone.
        let origin = ui.input(|i| i.pointer.press_origin());
        if ui.input(|i| i.modifiers.shift) {
            // Shift+drag measures bandwidth — overrides edge grab / resize / pan,
            // and cancels any lingering fade from a previous measurement.
            measuring = origin.map(|p| view.x_to_freq(p.x, &rect));
            edge = None;
            sub_drag = false;
            resizing = false;
            bw_fade = None;
            scale_axis = None;
        } else {
            edge = origin.and_then(edge_at);
            sub_drag = edge.is_none() && origin.map(sub_grab_at).unwrap_or(false);
            // Not where the spectrum is suppressed rather than collapsed: the
            // scale strip is at the top of the widget there, and a drag on it
            // would quietly rewrite a split the operator cannot see.
            resizing = !waterfall_only
                && edge.is_none()
                && !sub_drag
                && origin.map(|p| scale_rect.contains(p)).unwrap_or(false);
            measuring = None;
            scale_axis = None;
        }
        ui.data_mut(|d| {
            d.insert_temp(edge_id, edge);
            d.insert_temp(sub_drag_id, sub_drag);
            d.insert_temp(resize_id, resizing);
            d.insert_temp(scale_axis_id, scale_axis);
            d.insert_temp(measure_id, measuring);
            d.insert_temp(fade_id, bw_fade);
        });
    }
    if resp.drag_stopped() {
        // Hand the drag's momentum to the dial it was turning, before the flags
        // that say which dial that was are cleared below. Only the two tuning
        // gestures have a flywheel: a filter edge, the resize grip, the
        // measurement ruler and a pan that does not tune (right-drag, or
        // left-drag with `drag_tunes` off) have nothing to spin. egui
        // deliberately keeps the pointer's final velocity alive for the frame a
        // drag ends on, which is exactly the release speed this needs.
        let dial = if !resp.drag_stopped_by(egui::PointerButton::Primary) {
            None
        } else if sub_drag {
            Some(true)
        } else if wheel.drag_tunes && edge.is_none() && !resizing && measuring.is_none() {
            Some(false)
        } else {
            None
        };
        if let Some(sub) = dial {
            let vx = ui.input(|i| i.pointer.velocity().x);
            fling = (vx.abs() >= FLING_MIN_LAUNCH).then_some(Fling { sub, vx });
        }
        // If we were measuring, freeze the span and start its fade-out.
        if let Some(start_hz) = measuring {
            let end_hz =
                resp.interact_pointer_pos().map(|p| view.x_to_freq(p.x, &rect)).unwrap_or(start_hz);
            bw_fade = Some((start_hz, end_hz, ui.input(|i| i.time)));
        }
        edge = None;
        sub_drag = false;
        resizing = false;
        measuring = None;
        ui.data_mut(|d| {
            d.insert_temp(edge_id, edge);
            d.insert_temp(sub_drag_id, sub_drag);
            d.insert_temp(resize_id, resizing);
            d.insert_temp(measure_id, measuring);
            d.insert_temp(fade_id, bw_fade);
        });
    }
    if measuring.is_some() || (resp.hovered() && ui.input(|i| i.modifiers.shift)) {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    } else if hover_edge.is_some() || edge.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    } else if sub_drag {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if hover_sub {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    } else if scale_axis == Some(true) {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    } else if hover_resize || resizing {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    // Secondary-button panning is tracked manually — egui only registers
    // widget drags started with the primary button.
    let sec_id = ui.id().with("sec-pan");
    let mut sec_pan: bool = ui.data(|d| d.get_temp(sec_id)).unwrap_or(false);
    let (sec_down, press_origin, pointer_delta) =
        ui.input(|i| (i.pointer.secondary_down(), i.pointer.press_origin(), i.pointer.delta()));
    let sec_pan_before = sec_pan;
    if sec_down && !sec_pan && press_origin.is_some_and(|p| rect.contains(p)) {
        sec_pan = true;
    }
    if !sec_down {
        sec_pan = false;
    }
    if sec_pan != sec_pan_before {
        ui.data_mut(|d| d.insert_temp(sec_id, sec_pan));
    }

    // A hand back on a spinning knob stops it dead. This catches the press
    // itself, not the drag it may become: a click-tune, or the first few pixels
    // before the drag threshold trips, must not have the old coast running
    // underneath them.
    if fling.is_some()
        && ui.input(|i| i.pointer.any_pressed())
        && press_origin.is_some_and(|p| rect.contains(p))
    {
        fling = None;
        stop_click = true;
    }

    // Two fingers on the panadapter zoom the span about the point between them
    // — the only way to change it on a screen with no wheel.
    //
    // First in the chain, and it swallows the drag: the browser keeps reporting
    // a one-finger drag from the *first* finger down for the whole gesture, so
    // without this a pinch would pan the view and turn the dial at the same
    // time as zooming it.
    let pinch = ui.input(|i| i.multi_touch()).filter(|mt| rect.contains(mt.center_pos));
    if let Some(mt) = pinch {
        let zoom = mt.zoom_delta as f64;
        if zoom > 0.0 {
            // Fingers apart is zoom *in*, i.e. a smaller span.
            zoom_about(view, mt.center_pos.x, &rect, 1.0 / zoom, dev_span);
        }
        // A gesture is a fresh command to the view; a coast still running would
        // fight it, and the lift at the end must not land as a click-tune.
        fling = None;
        stop_click = true;
    } else if measuring.is_some() && resp.dragged_by(egui::PointerButton::Primary) {
        // Bandwidth measurement — no tuning or panning; drawn in the overlay
        // pass at the end.
    } else if resizing && resp.dragged_by(egui::PointerButton::Primary) {
        // A strip drag is one gesture or the other, decided once it has gone
        // far enough to mean something and held for the rest of the drag.
        // Below that it does nothing at all: the resize reads the pointer's
        // absolute height, so a gesture that turns out to be one catches up in
        // the frame it is claimed and nothing is lost by waiting.
        if scale_axis.is_none() {
            let origin = ui.input(|i| i.pointer.press_origin());
            if let (Some(p), Some(o)) = (resp.interact_pointer_pos(), origin) {
                let travel = p - o;
                if travel.length() >= SCALE_AXIS_LOCK {
                    scale_axis = Some(travel.x.abs() > travel.y.abs());
                }
            }
        }
        match scale_axis {
            // Sideways: slide the band under the picture, and only the band —
            // the axis says where the window is, not where the dial is, so
            // this never turns the VFO. What the captured span cannot absorb
            // moves the front end's own centre, which is the only way the axis
            // can keep going once the view is the whole window.
            Some(true) => {
                let dhz = -resp.drag_delta().x as f64 * view.span() / rect.width() as f64;
                let over = pan_view(view, dhz, zoom_center, zoom_span);
                pan_center(&mut dev_center, state, over, pan, cmds);
            }
            // Spectrum/waterfall resize — set the spectrum height from the
            // pointer.
            Some(false) => {
                if let Some(p) = resp.interact_pointer_pos() {
                    let usable = (rect.height() - scale_h()).max(1.0);
                    view.spectrum_fraction = ((p.y - rect.top()) / usable).clamp(0.10, 0.85);
                    // Dragging the divider asks for a split, so whichever layer
                    // was switched off comes back — this is the way out of
                    // either one-layer view without going back to the SPEC
                    // popup.
                    view.spectrum_collapsed = false;
                    view.waterfall_collapsed = false;
                }
            }
            None => {}
        }
    } else if let (Some((rx, is_hi)), true) = (edge, resp.dragged_by(egui::PointerButton::Primary))
    {
        // Filter edge drag — exclusive, never pans. Edges are relative to the
        // receiver they belong to, so the sub's grips measure from where the
        // sub sits, not from the dial.
        if let Some(p) = resp.interact_pointer_pos() {
            let anchor = if rx == RxId::Main { vfo_hz } else { sub_hz };
            let rel = view.x_to_freq(p.x, &rect) - anchor;
            let r = &mut state.rx[rx.index()];
            let max_hz = r.mode.max_filter_hz() as f64;
            let (mut lo, mut hi) = (r.filter_lo as f64, r.filter_hi as f64);
            if r.mode.filter_symmetric() {
                // AM and FM carve a channel out about the carrier, so the grip
                // being dragged sets the *half* width and the other edge
                // follows it (issue #256). Read from whichever grip was
                // grabbed, so either one widens and narrows the passband.
                let half = rel.abs().clamp(25.0, max_hz);
                (lo, hi) = (-half, half);
            } else if is_hi {
                hi = rel.clamp(lo + 50.0, max_hz);
            } else {
                lo = rel.clamp(-max_hz, hi - 50.0);
            }
            // Optimistic echo so the grip tracks the pointer exactly.
            (r.filter_lo, r.filter_hi) = (lo as f32, hi as f32);
            cmds.push(Command::SetFilter { rx, lo: lo as f32, hi: hi as f32 });
        }
    } else if sub_drag && resp.dragged_by(egui::PointerButton::Primary) {
        // Sub-receiver tuning drag — exclusive, never pans, and clamped to the
        // device passband because that is all its DDC can reach.
        //
        // By the pointer's *delta*, not its absolute position: the grab is the
        // whole passband, so snapping the receiver's centre onto the pointer
        // would jump it by half a filter width the moment the drag began.
        // The sub follows the pointer rather than the grab-the-content sense a
        // pan has — the operator has hold of the receiver itself.
        let dhz = resp.drag_delta().x as f64 * view.span() / rect.width() as f64;
        let hz = gesture_step(gesture_dial, state.sub_rx_hz, dhz).clamp(dev_lo, dev_hi);
        gesture_dial = Some(hz);
        state.sub_rx_hz = hz; // optimistic echo, as for the filter grips
        cmds.push(Command::SetSubRxFreq(hz));
    } else if sec_pan {
        // Right-drag: pan only, grab-the-content semantics — and only inside
        // the captured window. Moving the window is moving the receiver, which
        // is what this gesture exists to promise it will not do; at the edge it
        // stops.
        let dhz = -pointer_delta.x as f64 * view.span() / rect.width() as f64;
        pan_view(view, dhz, zoom_center, zoom_span);
    } else if resp.dragged_by(egui::PointerButton::Primary) {
        // Left-drag: grab the spectrum and slide it — content follows the
        // mouse, and the tuning follows the content (dragging right tunes
        // down). The view pans along so the VFO marker keeps its place.
        // With `drag_tunes` off it pans only, like the right-drag.
        //
        // Whatever the view cannot take goes to the front end, which moves its
        // centre and keeps the picture scrolling — but only when this drag is
        // already turning the dial, because a window that moves takes the dial
        // with it whether anyone asked or not (issue #133).
        let dhz = -resp.drag_delta().x as f64 * view.span() / rect.width() as f64;
        let over = pan_view(view, dhz, zoom_center, zoom_span);
        if wheel.drag_tunes {
            // Ahead of the dial: the two move by the same amount, so a window
            // moved first leaves the dial exactly where it was inside it and
            // the engine's own span guard has nothing to do.
            pan_center(&mut dev_center, state, over, pan, cmds);
            let hz = gesture_step(gesture_dial, state.active_freq_hz(), dhz).max(0.0);
            gesture_dial = Some(hz);
            match state.active_vfo {
                Vfo::A => state.vfo_a_hz = hz, // optimistic echo
                Vfo::B => state.vfo_b_hz = hz,
            }
            // The dial, not a click: dragging turns the VFO continuously, so a
            // radio that tunes with its dial comes along (see `SetVfo`).
            cmds.push(Command::SetVfo { vfo: state.active_vfo, hz });
        }
    } else {
        // --- wheel (zoom or tune) / click-tune -----------------------------
        if let Some(pos) = resp.hover_pos() {
            let (delta, shift) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.shift));
            // Shift is egui's `horizontal_scroll_modifier`, so while it is held
            // it folds the whole wheel delta onto the horizontal axis and `.y`
            // reads zero. Take whichever axis the detents landed on, or the
            // shift gesture is dead whatever it is bound to.
            let scroll = if shift { delta.x + delta.y } else { delta.y };
            let scroll = if wheel.invert { -scroll } else { scroll };
            let act = if shift { wheel.wheel_shift } else { wheel.wheel };
            // Zooming rides the smoothed points, which is what makes it ramp;
            // tuning counts whole notches instead, so it needs a bank kept
            // across frames. See `wheel_detents`.
            let detents = if act == WheelAction::Tune {
                let acc_id = ui.id().with("wheel-acc");
                let mut acc = ui.data(|d| d.get_temp::<f32>(acc_id)).unwrap_or(0.0);
                let n = super::wheel_detents(ui, shift, &mut acc);
                ui.data_mut(|d| d.insert_temp(acc_id, acc));
                if wheel.invert { -n } else { n }
            } else {
                0.0
            };
            if scroll.abs() > 0.1 || detents != 0.0 {
                // Zooming or wheel-tuning is a fresh command to the dial; a coast
                // still running would fight it.
                fling = None;
                match act {
                    WheelAction::Zoom => {
                        // Zoom around the cursor; scroll up = zoom in.
                        let rate = wheel.zoom_rate.clamp(0.1, 5.0) as f64;
                        let factor = 0.998f64.powf(scroll as f64 * 2.0 * rate);
                        zoom_about(view, pos.x, &rect, factor, dev_span);
                    }
                    WheelAction::Tune if detents != 0.0 => {
                        // Onto the step grid, not one step from wherever the
                        // dial was left: dragging the panadapter tunes
                        // continuously, and the fraction of a step it leaves
                        // behind would otherwise ride along for good. Click-to-
                        // tune has always rounded — this is the same courtesy.
                        let step = wheel.tune_step_hz;
                        let hz = if step > 0.0 {
                            (state.active_freq_hz() / step).round() * step + detents as f64 * step
                        } else {
                            state.active_freq_hz()
                        };
                        // Same optimistic echo as the drag arm above, so the
                        // readout keeps up with a fast scroll.
                        let hz = hz.max(0.0);
                        match state.active_vfo {
                            Vfo::A => state.vfo_a_hz = hz,
                            Vfo::B => state.vfo_b_hz = hz,
                        }
                        // Wheel-tuning is the dial too — the same gesture the
                        // readout's digits take, made over the picture.
                        cmds.push(Command::SetVfo { vfo: state.active_vfo, hz });
                    }
                    WheelAction::Tune | WheelAction::None => {}
                }
            }
        }
        if resp.clicked() && !stop_click {
            if let Some(pos) = resp.interact_pointer_pos() {
                let step = if wheel.click_tune_step_hz > 0.0 {
                    wheel.click_tune_step_hz
                } else {
                    CLICK_TUNE_STEP
                };
                if ui.input(|i| i.modifiers.shift) {
                    // Shift-click places the *second* receiver: the sub when it
                    // is running, VFO B otherwise — what this gesture meant
                    // before the sub had a frequency of its own.
                    //
                    // Taken ahead of the spot boxes and the digital-mode branch
                    // so it means the same thing everywhere on the panadapter.
                    // That matters most in FT8, where a plain click moves the
                    // transmit offset instead of tuning: without this there
                    // would be no way to park the sub on a signal at all.
                    let hz = net_boxes
                        .iter()
                        .find(|b| b.rect.contains(pos))
                        .map(|b| net_spots[b.idx].freq_hz)
                        .or_else(|| {
                            spot_boxes
                                .iter()
                                .find(|b| b.rect.contains(pos))
                                .map(|b| skimmer[b.idx].freq_hz)
                        })
                        .unwrap_or_else(|| (view.x_to_freq(pos.x, &rect) / step).round() * step);
                    if state.sub_rx_enabled {
                        let hz = hz.clamp(dev_lo, dev_hi);
                        state.sub_rx_hz = hz; // optimistic echo
                        cmds.push(Command::SetSubRxFreq(hz));
                    } else {
                        state.vfo_b_hz = hz;
                        cmds.push(Command::TuneInSpan { vfo: Vfo::B, hz });
                    }
                } else if let Some(nb) = net_boxes.iter().find(|b| b.rect.contains(pos)) {
                    // Network spot: tune + set mode, and hand the spot back so the
                    // app can pre-fill a log entry (and optionally look it up).
                    let spot = &net_spots[nb.idx];
                    tune_to_net_spot(spot, state, cmds);
                    *clicked_spot = Some(spot.clone());
                } else if let Some(sb) = spot_boxes.iter().find(|b| b.rect.contains(pos)) {
                    let spot = &skimmer[sb.idx];
                    let spot_hz = spot.freq_hz;
                    if click_sets_offset {
                        // FT8 station box. Moving our transmit onto the station
                        // is what Auto TX FRQ exists to avoid — they transmit in
                        // the period opposite ours, so their frequency says
                        // nothing about who is there when we key. With it on the
                        // label is a label; with it off it tunes, as before.
                        if !auto_tx_freq && !hold_tx_freq {
                            let audio = (spot_hz - state.rx_freq_hz()) as f32;
                            cmds.push(Command::SetDigiAudioFreq(audio.clamp(200.0, 3500.0)));
                        }
                    } else {
                        // Skimmer spot: switch to the spot's mode and tune onto it.
                        match spot.kind {
                            SkimmerKind::Cw => {
                                // Dial a sidetone-pitch below so it lands on the
                                // CW cursor — the operator's own pitch when one
                                // is already set, the mode default otherwise.
                                let pitch = cursor_hz.map(f64::from).unwrap_or_else(|| {
                                    let (lo, hi) = Mode::Cw.default_filter();
                                    ((lo + hi) * 0.5) as f64
                                });
                                cmds.push(Command::TuneInSpan {
                                    vfo: state.active_vfo,
                                    hz: spot_hz - pitch,
                                });
                                cmds.push(Command::SetMode { rx: RxId::Main, mode: Mode::Cw });
                            }
                            SkimmerKind::Psk | SkimmerKind::Rtty => {
                                // Put the signal at the mode's own audio offset
                                // and open the PSK/RTTY panel there for a clean
                                // decode. RTTY's is the standard tone pair
                                // rather than a convenient number: the tones
                                // this lands on are the ones it will transmit.
                                // Already in the mode, the operator's own
                                // cursor wins — the offset they set is where
                                // they want the signal, and a spot must not
                                // silently move it.
                                let m = spot.kind.mode();
                                let af = f64::from(
                                    cursor_hz
                                        .filter(|_| state.rx[0].mode == m)
                                        .or_else(|| m.standard_tone_offset_hz())
                                        .unwrap_or(1500.0),
                                );
                                cmds.push(Command::TuneInSpan {
                                    vfo: state.active_vfo,
                                    hz: spot_hz - af,
                                });
                                cmds.push(Command::SetMode { rx: RxId::Main, mode: m });
                                cmds.push(Command::SetDigiAudioFreq(af as f32));
                            }
                        }
                    }
                } else if click_sets_offset && !hold_tx_freq {
                    // Digital mode: set the audio TX offset, not the VFO. Held
                    // means held even for a deliberate click on clear water:
                    // the case this exists for is a licence edge, where a slip
                    // is an out-of-band transmission.
                    let audio = (view.x_to_freq(pos.x, &rect) - state.rx_freq_hz()) as f32;
                    cmds.push(Command::SetDigiAudioFreq(audio.clamp(200.0, 3500.0)));
                } else {
                    // CW: tune so the clicked signal lands *on* the cursor, not
                    // on the dial. The dial in CW is a sidetone-pitch below what
                    // you hear, so tuning a signal to the dial is the one place
                    // it is guaranteed not to be audible.
                    let off = cursor_hz.map(f64::from).unwrap_or(0.0);
                    let hz = ((view.x_to_freq(pos.x, &rect) - off) / step).round() * step;
                    cmds.push(Command::TuneInSpan { vfo: state.active_vfo, hz });
                }
            }
        }
    }

    // --- dial inertia -----------------------------------------------------
    // Carry a released tuning drag on, decelerating, so the dial coasts to rest
    // instead of stopping the instant the button comes up. Both terms of the
    // deceleration are in screen points, and the Hz step is derived from the
    // span in force *this* frame, so a coast that is zoomed into mid-flight
    // slows down in Hz with it and keeps sliding the display at the same rate.

    // A sub receiver switched off mid-coast leaves no dial to turn, and its
    // momentum must not be waiting when it comes back.
    if fling.is_some_and(|f| f.sub && !sub_on) {
        fling = None;
    }
    if let Some(f) = fling.filter(|_| !resp.dragged_by(egui::PointerButton::Primary) && !sec_pan) {
        // Clamped: a stalled frame (a mode change reloading the FFT, say) must
        // not teleport the dial across the band.
        let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.1);
        let vx = fling_decay(f.vx, dt);
        if vx.abs() < FLING_STOP {
            fling = None;
        } else {
            let dhz = (vx * dt) as f64 * view.span() / rect.width() as f64;
            fling = Some(Fling { vx, ..f });
            if f.sub {
                // The sub follows the pointer, so it coasts the same way it was
                // dragged; the device passband is the end stop its DDC cannot
                // reach past, and hitting it parks the dial.
                // `was` is the same number `gesture_step` builds on, kept so
                // the end stop is read off the coast's own dial: clamping to a
                // value it already had is what "ran into the edge" means, and
                // testing that against the engine's echo instead would park the
                // coast every time a snapshot arrived a little behind it.
                let was = gesture_dial.unwrap_or(state.sub_rx_hz);
                let hz = gesture_step(gesture_dial, state.sub_rx_hz, dhz).clamp(dev_lo, dev_hi);
                if hz == was {
                    fling = None;
                } else {
                    gesture_dial = Some(hz);
                    state.sub_rx_hz = hz; // optimistic echo, as during the drag
                    cmds.push(Command::SetSubRxFreq(hz));
                }
            } else {
                // Grab-the-content sense, matching the left-drag: the view slides
                // with the dial so the VFO marker keeps its place on screen, and
                // the window follows once the view has run out of room.
                let over = pan_view(view, -dhz, zoom_center, zoom_span);
                pan_center(&mut dev_center, state, over, pan, cmds);
                let hz = gesture_step(gesture_dial, state.active_freq_hz(), -dhz).max(0.0);
                gesture_dial = Some(hz);
                match state.active_vfo {
                    Vfo::A => state.vfo_a_hz = hz,
                    Vfo::B => state.vfo_b_hz = hz,
                }
                // Still the drag that let go, so still the dial.
                cmds.push(Command::SetVfo { vfo: state.active_vfo, hz });
                if hz == 0.0 {
                    fling = None; // ran into the bottom of the spectrum
                }
            }
            // The coast is animation, not a response to input: without this it
            // would advance only when something else happened to repaint.
            crate::repaint::animate(ui.ctx());
        }
    }
    // The swallow only covers the one press/release pair that caught the dial.
    if ui.input(|i| i.pointer.any_released()) {
        stop_click = false;
    }
    // Nothing is turning the dial any more, so the next gesture seeds itself
    // from the state — which by then is where the radio actually ended up, and
    // not this screen's guess at it.
    if fling.is_none() && !resp.dragged_by(egui::PointerButton::Primary) {
        gesture_dial = None;
    }
    ui.data_mut(|d| {
        d.insert_temp(fling_id, fling);
        d.insert_temp(stop_click_id, stop_click);
        d.insert_temp(gesture_dial_id, gesture_dial);
        d.insert_temp(scale_axis_id, scale_axis);
    });

    // Centre tuning (issue #174): keep the dial in the middle of the window.
    //
    // Edge-triggered on the dial *moving*, not held every frame, so a pan or a
    // zoom-about-the-cursor still goes where it was put and only the next tune
    // brings the marker home. The memo is dropped while the mode is off, which
    // is what makes switching it on centre at once — that click is also the
    // "centre the display now" button the issue asked for.
    //
    // Zoomed in this costs nothing: the window slides inside the captured span
    // and the hardware never hears about it. Zoomed right out `pan_view` has no
    // room to give and hands the whole move on, and the front end's own centre
    // follows the dial — the same path a drag at the edge takes. Below a
    // column's worth of movement the front end is left alone: a retune costs a
    // waterfall remap and, on some front ends, a skimmer restart, and nobody
    // can see a sub-pixel slide.
    let centring_id = ui.id().with("centre-on-vfo");
    let centred_at: Option<f64> = ui.data(|d| d.get_temp(centring_id)).unwrap_or(None);
    if view.center_on_vfo {
        let vfo = state.active_freq_hz();
        if centred_at.is_none_or(|was| (was - vfo).abs() > 0.5) {
            let over = center_on_dial(view, vfo, zoom_center, zoom_span, rect.width());
            pan_center(&mut dev_center, state, over, pan, cmds);
        }
        ui.data_mut(|d| d.insert_temp(centring_id, Some(vfo)));
    } else if centred_at.is_some() {
        ui.data_mut(|d| d.insert_temp(centring_id, None::<f64>));
    }

    let view_log_id = ui.id().with("view-log");

    // The panadapter's whole zoom/pan story in one line, and only when it
    // moves. Three numbers because three different failures look identical on
    // screen: a gesture that never reached the view (span unchanged frame after
    // frame while the wheel turns), a view that moved and was clamped back
    // (`span_after` < `span_before`), and a view that moved freely while the
    // picture did not follow (span changes here, waterfall does not).
    let span_before = view.span();
    // Not before the front end has said what it is: see [`WindowPan::known`].
    if pan.known {
        // Against the window as it is *now*. `zoom_center` was measured at the
        // top of the frame, and on a front end with no full-band lane the pan
        // bounds *are* the passband — which [`pan_center`] has just moved, by
        // exactly the overshoot this clamp would otherwise pull the view back
        // over. That undoes the pan every frame, and the next frame finds the
        // view outside the window again on the other side: the window and the
        // view chase each other, one frame apart, and the panadapter jitters
        // sideways for as long as the button is held. Measured on a 1.5 Msps
        // generator: a steady drag reversed direction on 40 of its 50 frames,
        // and went on oscillating ±76 kHz with the pointer standing still.
        //
        // A front end that *has* a full-band lane is unaffected either way —
        // its bounds are that lane's fixed window and do not move with the
        // passband — which is why this never showed on a KiwiSDR.
        let (clamp_center, clamp_span) = pan.view_bounds(dev_center, dev_span);
        view.clamp_to(clamp_center, clamp_span);
    }
    follow_view_into_the_band(view, state, dev_center, dev_span, pan, cmds);
    let (lo_now, hi_now, span_now) = (view.view_lo_hz, view.view_hi_hz, view.span());
    if ui.data(|d| d.get_temp::<(f64, f64)>(view_log_id)) != Some((lo_now, hi_now)) {
        ui.data_mut(|d| d.insert_temp(view_log_id, (lo_now, hi_now)));
        tracing::debug!(
            target: "sdroxide::panadapter",
            dev_center,
            dev_span,
            view_lo = lo_now,
            view_hi = hi_now,
            span_before,
            span_after = span_now,
            clamped = span_now < span_before * 0.999,
            "panadapter view moved",
        );
    }

    // --- drawing ----------------------------------------------------------
    // Recompute after this frame's pan/tune/edge updates.
    let vfo_hz = state.rx_freq_hz();
    let px0 = view.freq_to_x(vfo_hz + state.rx[0].filter_lo as f64, &rect);
    let px1 = view.freq_to_x(vfo_hz + state.rx[0].filter_hi as f64, &rect);
    let sub_hz = state.sub_rx_hz;
    let spx0 = view.freq_to_x(sub_hz + state.rx[1].filter_lo as f64, &rect);
    let spx1 = view.freq_to_x(sub_hz + state.rx[1].filter_hi as f64, &rect);

    if spec_h > 1.0 {
        // Optional vertical background gradient behind the grid and trace.
        if let Some((top, bottom)) = wf.gradient {
            let mut mesh = egui::epaint::Mesh::default();
            let uv = egui::epaint::WHITE_UV;
            mesh.vertices.push(egui::epaint::Vertex { pos: spec_rect.left_top(), uv, color: top });
            mesh.vertices.push(egui::epaint::Vertex { pos: spec_rect.right_top(), uv, color: top });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: spec_rect.right_bottom(),
                uv,
                color: bottom,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: spec_rect.left_bottom(),
                uv,
                color: bottom,
            });
            mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
            painter.add(Shape::mesh(mesh));
        }
        // UI-smoothed (per the spectrum-speed setting) so the line's reaction
        // speed is independent of the un-averaged waterfall detail. Folded in
        // before either display is drawn: the 3D surface remembers these same
        // bins, so the reaction setting steadies it exactly as it steadies the
        // flat trace.
        smooth.update(f, wf.spectrum_alpha);
        if view.spectrum_3d {
            // Time has the depth axis instead of being thrown away. The flat
            // grid and the peak hold are both statements about a single line
            // and have nothing to say about a surface, so neither is drawn —
            // the surface brings its own floor and its own amplitude axis.
            peaks.clear();
            surface.push(
                f.center_hz,
                f.span_hz,
                &smooth.bins,
                wf.now_unix,
                wf.surface_rows_per_sec,
            );
            crate::widgets::spectrum3d::draw(
                &painter,
                &spec_rect,
                view,
                surface,
                view.spectrum_3d_solid,
                wf.palette,
            );
        } else {
            surface.clear();
            draw_grid(&painter, view, &spec_rect);
            if view.peak_hold {
                peaks.update(f);
                let key = trace_key(f, peaks.generation, view, &spec_rect, wf.row_scale);
                let pts = trace.hold.points_for(key, || {
                    compute_trace(view, f, Some(&peaks.bins), &spec_rect, wf.row_scale)
                });
                painter.add(Shape::line(
                    pts,
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 90, 170)),
                ));
            } else {
                peaks.clear();
            }
            let key = trace_key(f, smooth.generation, view, &spec_rect, wf.row_scale);
            let pts = trace.live.points_for(key, || {
                compute_trace(view, f, Some(&smooth.bins), &spec_rect, wf.row_scale)
            });
            painter.add(Shape::line(pts, Stroke::new(1.0, Color32::from_rgb(120, 220, 255))));
        }
    }
    draw_scale(&painter, view, &scale_rect);

    // Resize grip: a short centred handle in the scale strip, brightening when
    // hovered/dragged, so the divider is discoverable.
    {
        let cy = scale_rect.top() + 1.5;
        let cx = scale_rect.center().x;
        let col = if hover_resize || resizing {
            crate::theme::scope().accent
        } else {
            crate::theme::scope_gray(70)
        };
        for dx in [-16.0f32, 0.0, 16.0] {
            painter.line_segment(
                [pos2(cx + dx - 6.0, cy), pos2(cx + dx + 6.0, cy)],
                Stroke::new(2.0, col),
            );
        }
    }

    // Waterfall (GPU): viewport expressed in frame coordinates.
    let base = f.center_hz - f.span_hz / 2.0;
    let u_lo = ((view.view_lo_hz - base) / f.span_hz) as f32;
    let u_hi = ((view.view_hi_hz - base) / f.span_hz) as f32;
    ui.painter().add(crate::egui_wgpu::Callback::new_paint_callback(
        wf_rect,
        WaterfallCallback {
            // Arc::clone, not a bins deep-clone — keep it explicit.
            frame: Some(Arc::clone(f)),
            u_lo,
            u_hi,
            // Rows, not points: one history row per device pixel. Clamped to
            // the ring, or the shader's own `min(1.0)` would quietly draw the
            // same history over a taller widget and the timestamps would slide
            // off the rows they belong to.
            rows_visible: (wf_rect.height() * wf.row_scale).min(crate::waterfall_gpu::TEX_H as f32),
            lut: wf.palette,
            rows_to_write: wf.rows_to_write,
            flip: view.waterfall_flip,
            wf_id: wf.wf_id,
            tex_w: wf.tex_w,
        },
    ));

    // Bandplan strip along the bottom of the waterfall (over the GPU layer),
    // with the stored memories marked on the same edge, stacked inwards from
    // however deep the strip ended up.
    let strip_h = crate::widgets::bandplan::overlay(&painter, view, &wf_rect, panel_below);
    crate::widgets::memories::overlay(&painter, view, &wf_rect, mem, strip_h, panel_below);

    // --- VFO markers + passband shading -----------------------------------
    let in_view = |hz: f64| (view.view_lo_hz..=view.view_hi_hz).contains(&hz);

    let (x0c, x1c) = (px0.clamp(rect.left(), rect.right()), px1.clamp(rect.left(), rect.right()));
    if x1c > x0c {
        // Passband shading on the spectrum strip and (fainter) on the
        // waterfall, so the filter width stays visible when collapsed.
        if spec_h > 1.0 {
            painter.rect_filled(
                Rect::from_min_max(pos2(x0c, spec_marks.top()), pos2(x1c, spec_marks.bottom())),
                0.0,
                Color32::from_rgba_unmultiplied(255, 90, 90, 26),
            );
        }
        painter.rect_filled(
            Rect::from_min_max(pos2(x0c, wf_rect.top()), pos2(x1c, wf_rect.bottom())),
            0.0,
            Color32::from_rgba_unmultiplied(255, 90, 90, 16),
        );
    }
    // Edge grips (brighter when grabbable). Drawn on both strips so the
    // filter edges remain visible with the spectrum line collapsed.
    for (x, e) in [(px0, (RxId::Main, false)), (px1, (RxId::Main, true))] {
        if rect.x_range().contains(x) {
            let hot = hover_edge == Some(e) || edge == Some(e);
            let w = if hot { 2.0 } else { 1.0 };
            if spec_h > 1.0 {
                let color = if hot {
                    Color32::from_rgb(255, 170, 90)
                } else {
                    crate::theme::scope_gray(90)
                };
                painter.vline(x, spec_marks.y_range(), Stroke::new(w, color));
            }
            let wf_color = if hot {
                Color32::from_rgba_unmultiplied(255, 170, 90, 200)
            } else {
                Color32::from_rgba_unmultiplied(150, 160, 170, 120)
            };
            painter.vline(x, wf_rect.y_range(), Stroke::new(w, wf_color));
        }
    }

    // Where the receiver is actually pointed, which in CW read as the signal
    // is a sidetone pitch up from the dial — see [`AudioCursor::line_on_cursor`].
    let line_hz = vfo_hz + cursor.filter(|c| c.line_on_cursor).map_or(0.0, |c| f64::from(c.hz));
    if in_view(line_hz) {
        let x = view.freq_to_x(line_hz, &rect);
        painter.vline(x, spec_marks.y_range(), Stroke::new(1.0, Color32::from_rgb(255, 60, 60)));
        painter.vline(
            x,
            wf_rect.y_range(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 60, 60, 140)),
        );
    }

    // --- sub receiver ------------------------------------------------------
    // Drawn with the same vocabulary as the main receiver — passband wash, edge
    // grips, tuning line — so a second receiver reads as a receiver and not as
    // one more annotation. Only its colour differs.
    if sub_on {
        let (c_r, c_g, c_b) = (SUB_COLOR.r(), SUB_COLOR.g(), SUB_COLOR.b());
        let (sx0c, sx1c) =
            (spx0.clamp(rect.left(), rect.right()), spx1.clamp(rect.left(), rect.right()));
        if sx1c > sx0c {
            if spec_h > 1.0 {
                painter.rect_filled(
                    Rect::from_min_max(
                        pos2(sx0c, spec_marks.top()),
                        pos2(sx1c, spec_marks.bottom()),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(c_r, c_g, c_b, 30),
                );
            }
            painter.rect_filled(
                Rect::from_min_max(pos2(sx0c, wf_rect.top()), pos2(sx1c, wf_rect.bottom())),
                0.0,
                Color32::from_rgba_unmultiplied(c_r, c_g, c_b, 20),
            );
        }
        for (x, e) in [(spx0, (RxId::Sub, false)), (spx1, (RxId::Sub, true))] {
            if rect.x_range().contains(x) {
                let hot = hover_edge == Some(e) || edge == Some(e);
                let w = if hot { 2.0 } else { 1.0 };
                if spec_h > 1.0 {
                    let color = if hot {
                        SUB_COLOR
                    } else {
                        Color32::from_rgba_unmultiplied(c_r, c_g, c_b, 110)
                    };
                    painter.vline(x, spec_marks.y_range(), Stroke::new(w, color));
                }
                painter.vline(
                    x,
                    wf_rect.y_range(),
                    Stroke::new(
                        w,
                        Color32::from_rgba_unmultiplied(c_r, c_g, c_b, if hot { 200 } else { 90 }),
                    ),
                );
            }
        }
        if in_view(sub_hz) {
            let x = view.freq_to_x(sub_hz, &rect);
            // The tuning line thickens under the pointer so the operator can
            // see they have hold of it before they start dragging.
            let w = if hover_sub || sub_drag { 2.5 } else { 1.5 };
            painter.vline(x, spec_marks.y_range(), Stroke::new(w, SUB_COLOR));
            painter.vline(
                x,
                wf_rect.y_range(),
                Stroke::new(w, Color32::from_rgba_unmultiplied(c_r, c_g, c_b, 170)),
            );
            // "SUB" tag on the waterfall, on whichever side of the line has
            // room — at the band edge it would otherwise be clipped away.
            let (anchor, tx) = if x > rect.right() - 34.0 {
                (Align2::RIGHT_TOP, x - 3.0)
            } else {
                (Align2::LEFT_TOP, x + 3.0)
            };
            painter.text(
                pos2(tx, wf_rect.top() + 2.0),
                anchor,
                "SUB",
                FontId::proportional(9.5 * crate::theme::panadapter_font_scale()),
                SUB_COLOR,
            );
        }
    }

    // DXpedition zones: the Fox's half of the passband and the Hound calling
    // zone above it, drawn under the markers so they read as background.
    if dxped != sdroxide_types::DxpedMode::Normal {
        let dial = state.rx_freq_hz();
        let mine = |z: sdroxide_types::DxpedMode| z == dxped;
        for (lo, hi, label, base, ours) in [
            (
                0.0,
                sdroxide_types::FOX_ZONE_MAX_HZ as f64,
                "FOX",
                Color32::from_rgb(255, 120, 160),
                mine(sdroxide_types::DxpedMode::Fox),
            ),
            (
                sdroxide_types::FOX_ZONE_MAX_HZ as f64,
                sdroxide_types::HOUND_ZONE_MAX_HZ as f64,
                "HOUNDS",
                crate::theme::scope().accent,
                mine(sdroxide_types::DxpedMode::Hound),
            ),
        ] {
            let (x0, x1) = (view.freq_to_x(dial + lo, &rect), view.freq_to_x(dial + hi, &rect));
            if x1 <= rect.left() || x0 >= rect.right() {
                continue;
            }
            let band = Rect::from_min_max(
                pos2(x0.max(rect.left()), rect.top()),
                pos2(x1.min(rect.right()), rect.bottom()),
            );
            // The half we transmit in is tinted; the other only outlined, so
            // "where am I allowed to be" is legible at a glance.
            let fill = if ours { 26 } else { 10 };
            painter.rect_filled(
                band,
                0.0,
                Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), fill),
            );
            painter.vline(
                x1,
                rect.y_range(),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 90)),
            );
            if band.width() > 34.0 {
                painter.text(
                    pos2(band.left() + 4.0, rect.top() + 2.0),
                    Align2::LEFT_TOP,
                    label,
                    FontId::proportional(9.5 * crate::theme::panadapter_font_scale()),
                    Color32::from_rgba_unmultiplied(
                        base.r(),
                        base.g(),
                        base.b(),
                        if ours { 220 } else { 110 },
                    ),
                );
            }
        }
    }

    // The audio-offset cursor (cyan) at dial + offset.
    if let Some(a) = cursor_hz {
        let hz = state.rx_freq_hz() + a as f64;
        if in_view(hz) {
            let x = view.freq_to_x(hz, &rect);
            painter.vline(x, spec_marks.y_range(), Stroke::new(1.5, crate::theme::scope().accent));
            painter.vline(
                x,
                wf_rect.y_range(),
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 208, 244, 160)),
            );
        }
    }
    // Extra tuning lines (RTTY mark/space): thin amber, to aid exact tuning.
    for &a in markers {
        let hz = state.rx_freq_hz() + a as f64;
        if in_view(hz) {
            let x = view.freq_to_x(hz, &rect);
            let c = Color32::from_rgb(255, 190, 90);
            painter.vline(x, spec_marks.y_range(), Stroke::new(1.0, c));
            painter.vline(
                x,
                wf_rect.y_range(),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 190, 90, 150)),
            );
        }
    }
    // Inactive VFO marker — where a split transmit goes, nothing more. The sub
    // receiver used to be pinned here and is drawn separately now that it has a
    // frequency of its own, so this stays quiet: it is a reference line, not a
    // receiver.
    let inactive_hz = match state.active_vfo {
        Vfo::A => state.vfo_b_hz,
        Vfo::B => state.vfo_a_hz,
    };
    if in_view(inactive_hz) {
        let x = view.freq_to_x(inactive_hz, &rect);
        let color = Color32::from_rgba_unmultiplied(255, 170, 40, 90);
        painter.vline(x, spec_marks.y_range(), Stroke::new(1.0, color));
        painter.vline(
            x,
            wf_rect.y_range(),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 170, 40, 110)),
        );
    }

    // --- RIT / XIT incremental-tuning offsets -----------------------------
    // When enabled, show the receive (RIT) and transmit (XIT) offsets from the
    // dial as labelled brackets, with a dashed dial reference (RIT — the red RX
    // marker/passband already sits at dial+RIT) and a TX marker line (XIT).
    let shifted = state.repeater.shift != sdroxide_types::Shift::Simplex;
    if state.rit.enabled || state.xit.enabled || shifted {
        let top_y = if spec_h > 1.0 { spec_rect.top() } else { wf_rect.top() } + 4.0;
        let rit_col = Color32::from_rgb(120, 200, 255);
        let xit_col = Color32::from_rgb(120, 230, 140);
        let mut row = 0.0;
        if state.rit.enabled {
            let dial = state.active_freq_hz();
            if in_view(dial) {
                marker_line(
                    &painter,
                    &spec_rect,
                    &wf_rect,
                    spec_h,
                    view.freq_to_x(dial, &rect),
                    crate::theme::scope_gray(150),
                    true,
                );
            }
            offset_bracket(
                &painter,
                rect,
                view.freq_to_x(dial, &rect),
                view.freq_to_x(state.rx_freq_hz(), &rect),
                top_y + row * 16.0,
                rit_col,
                &format!("RIT {:+} Hz", state.rit.hz),
            );
            row += 1.0;
        }
        if state.xit.enabled {
            let tx = state.tx_freq_hz();
            let base = tx - state.xit.effective_hz();
            if in_view(tx) {
                marker_line(
                    &painter,
                    &spec_rect,
                    &wf_rect,
                    spec_h,
                    view.freq_to_x(tx, &rect),
                    xit_col,
                    false,
                );
            }
            offset_bracket(
                &painter,
                rect,
                view.freq_to_x(base, &rect),
                view.freq_to_x(tx, &rect),
                top_y + row * 16.0,
                xit_col,
                &format!("XIT {:+} Hz", state.xit.hz),
            );
            row += 1.0;
        }
        // The repeater shift, drawn like XIT because it is the same kind of
        // thing: the transmitter is somewhere the receiver is not. The label
        // stands whether or not the transmit frequency is on screen — a 70 cm
        // shift is 7.6 MHz away and no panadapter shows both ends of that — and
        // the marker and bracket only appear when there is somewhere to put
        // them.
        if shifted {
            let tx = state.tx_freq_hz();
            let dup_col = Color32::from_rgb(255, 205, 80);
            let label = format!("TX {} · {:.6}", state.repeater.shift_label(), tx / 1e6);
            if in_view(tx) {
                marker_line(
                    &painter,
                    &spec_rect,
                    &wf_rect,
                    spec_h,
                    view.freq_to_x(tx, &rect),
                    dup_col,
                    false,
                );
                offset_bracket(
                    &painter,
                    rect,
                    view.freq_to_x(state.active_freq_hz(), &rect),
                    view.freq_to_x(tx, &rect),
                    top_y + row * 16.0,
                    dup_col,
                    &label,
                );
            } else {
                label_box(
                    &painter,
                    pos2(rect.left() + 6.0, top_y + row * 16.0 - 6.0),
                    &label,
                    dup_col,
                    rect,
                );
            }
        }
    }

    // Skimmer spot boxes over the waterfall (staggered lanes, on top of the
    // markers so they stay readable and clickable). Each box sits to the right
    // of its signal: a faint vertical line marks the signal's centre and a
    // horizontal leader joins it to the box.
    let hover_pos = resp.hover_pos();
    for b in &spot_boxes {
        let spot = &skimmer[b.idx];
        let a = alpha.get(b.idx).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let hovered = hover_pos.is_some_and(|p| b.rect.contains(p));
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let border = spot_color(spot, hovered);
        let cy = b.rect.center().y;
        // Vertical indicator over the signal centre (faint, full waterfall).
        let vcol = fade(
            Color32::from_rgba_unmultiplied(
                border.r(),
                border.g(),
                border.b(),
                if hovered { 170 } else { 90 },
            ),
            a,
        );
        painter.vline(b.sig_x, lanes_rect.y_range(), Stroke::new(1.0, vcol));
        // Horizontal leader from the signal to the box, with a junction node.
        painter.line_segment(
            [pos2(b.sig_x, cy), pos2(b.rect.left(), cy)],
            Stroke::new(1.0, fade(border, a)),
        );
        painter.circle_filled(pos2(b.sig_x, cy), 1.8, fade(border, a));
        draw_spot_box(&painter, b, spot, hovered, a, click_sets_offset);
    }

    // Network spot boxes (bottom-anchored lanes) — DX cluster / POTA / SOTA /
    // PSK. A vertical tick over the signal, a leader up/down to the box.
    for b in &net_boxes {
        let spot = &net_spots[b.idx];
        let a = net_alpha.get(b.idx).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let hovered = hover_pos.is_some_and(|p| b.rect.contains(p));
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        let border = net_spot_color(spot, hovered);
        let cy = b.rect.center().y;
        let vcol = fade(
            Color32::from_rgba_unmultiplied(
                border.r(),
                border.g(),
                border.b(),
                if hovered { 170 } else { 85 },
            ),
            a,
        );
        painter.vline(b.sig_x, lanes_rect.y_range(), Stroke::new(1.0, vcol));
        painter.line_segment(
            [pos2(b.sig_x, cy), pos2(b.rect.left(), cy)],
            Stroke::new(1.0, fade(border, a)),
        );
        painter.circle_filled(pos2(b.sig_x, cy), 1.8, fade(border, a));
        draw_net_box(&painter, b, spot, hovered, a);
    }

    // ISM device labels. No fade: a device that has stopped transmitting is
    // still there, and the ISM window's age column is where "how long ago" is
    // answered — a box that dimmed away would only hide the quiet ones.
    for b in &ism_boxes {
        let label = &ism[b.idx];
        let hovered = hover_pos.is_some_and(|p| b.rect.contains(p));
        let tint = if label.encrypted { crate::theme::YELLOW() } else { crate::theme::GREEN() };
        let cy = b.rect.center().y;
        painter.vline(
            b.sig_x,
            lanes_rect.y_range(),
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    tint.r(),
                    tint.g(),
                    tint.b(),
                    if hovered { 150 } else { 70 },
                ),
            ),
        );
        painter.line_segment([pos2(b.sig_x, cy), pos2(b.rect.left(), cy)], Stroke::new(1.0, tint));
        painter.circle_filled(pos2(b.sig_x, cy), 1.8, tint);
        draw_ism_box(&painter, b, label, hovered);
    }

    // --- time gridlines on the waterfall ----------------------------------
    // The newest row (top of the waterfall, or its bottom when flipped) is
    // "now"; rows away from it are older at `rows_per_sec` rows/second (≈ 1 row
    // per pixel). Draw a faint gray line at each round UTC time that falls in
    // the visible window.
    //
    // The interval has to follow the scroll speed rather than being fixed at a
    // minute. At the default speed a minute is some 1700 pixels of waterfall,
    // so on any ordinary window no minute boundary is on screen — the labels
    // existed but an operator would see one flick past every sixty seconds and
    // otherwise read the waterfall as having no timestamps at all. Picking the
    // step from what is actually visible keeps a handful of them in view at
    // every speed, which is the only way the axis is any use for saying when
    // something was heard.
    let rows_per_sec = wf.rows_per_sec as f64;
    if rows_per_sec > 0.01 && wf_rect.height() > 4.0 {
        // Seconds on screen from the rows actually on screen, not from the
        // widget's height: the two agree until the ring runs out, and where it
        // does the labels have to follow the rows rather than the pixels or
        // they would name times the waterfall is no longer showing.
        let rows_visible =
            ((wf_rect.height() * wf.row_scale) as f64).min(f64::from(crate::waterfall_gpu::TEX_H));
        let visible_secs = rows_visible / (rows_per_sec * wf.row_scale as f64);
        let pts_per_sec = wf_rect.height() as f64 / visible_secs.max(1e-6);
        let step = time_grid_step_s(visible_secs);
        let now = wf.now_unix;
        let oldest = now - visible_secs;
        let grid = Color32::from_rgba_unmultiplied(200, 205, 215, 60);
        let mut t = (oldest / step).ceil() * step; // first boundary ≥ oldest
        while t <= now {
            let age_px = ((now - t) * pts_per_sec) as f32;
            let y = if view.waterfall_flip {
                wf_rect.bottom() - age_px
            } else {
                wf_rect.top() + age_px
            };
            if (wf_rect.top()..=wf_rect.bottom()).contains(&y) {
                painter.hline(wf_rect.x_range(), y, Stroke::new(1.0, grid));
                let tod = (t as i64).rem_euclid(86_400);
                // Seconds only when the step is finer than a minute — on a slow
                // waterfall they would all read ":00" and cost width for it.
                let text = if step < 60.0 {
                    format!("{:02}:{:02}:{:02}", tod / 3600, (tod % 3600) / 60, tod % 60)
                } else {
                    format!("{:02}:{:02}", tod / 3600, (tod % 3600) / 60)
                };
                label_box(
                    &painter,
                    pos2(wf_rect.left() + 2.0, y + 1.0),
                    &text,
                    crate::theme::scope_gray(215),
                    wf_rect,
                );
            }
            t += step;
        }
    }

    // --- cursor readouts (hover) ------------------------------------------
    // A hovered filter edge shows its offset from its own receiver's centre;
    // otherwise hovering the spectrum/waterfall shows a faint crosshair + the
    // frequency a click would tune to. Suppressed while dragging.
    if let Some(p) = hover_pos {
        if let Some((rx, is_hi)) = hover_edge {
            // Item 7: filter edge offset from the VFO.
            let r = &state.rx[rx.index()];
            let off = if is_hi { r.filter_hi } else { r.filter_lo };
            let edge_x = match (rx, is_hi) {
                (RxId::Main, false) => px0,
                (RxId::Main, true) => px1,
                (RxId::Sub, false) => spx0,
                (RxId::Sub, true) => spx1,
            };
            let ytop = if spec_h > 1.0 { spec_rect.top() } else { wf_rect.top() };
            let tint = if rx == RxId::Main { Color32::from_rgb(255, 190, 120) } else { SUB_COLOR };
            label_box(
                &painter,
                pos2(edge_x + 7.0, ytop + 3.0),
                &format!("{:+} Hz", off.round() as i64),
                tint,
                rect,
            );
        } else if (spec_rect.contains(p) || wf_rect.contains(p))
            && edge.is_none()
            && !resizing
            && !resp.dragged()
            && !spot_boxes.iter().any(|b| b.rect.contains(p))
            && !net_boxes.iter().any(|b| b.rect.contains(p))
        {
            // Item 6: crosshair + click-tune frequency readout.
            let line = Color32::from_rgba_unmultiplied(185, 205, 225, 70);
            if spec_h > 1.0 {
                // The whole strip, not `spec_marks`: this one follows the
                // pointer, and a crosshair that stopped short of wherever the
                // pointer actually is would read as a broken one.
                painter.vline(p.x, spec_rect.y_range(), Stroke::new(1.0, line));
            }
            painter.vline(p.x, wf_rect.y_range(), Stroke::new(1.0, line));
            let text = click_tune_label(view, state, &rect, p.x, cursor);
            label_box(&painter, pos2(p.x + 8.0, p.y - 9.0), &text, Color32::WHITE, rect);
        }
    }

    // --- bandwidth measurement (shift+drag) + fade-out --------------------
    // Drawn on top of everything so the markers/label stay readable.
    if let (Some(start_hz), Some(p)) = (measuring, resp.interact_pointer_pos()) {
        // Active drag: full opacity, tracking the pointer.
        let end_hz = view.x_to_freq(p.x, &rect);
        draw_bw_measure(&painter, |hz| view.freq_to_x(hz, &rect), &rect, start_hz, end_hz, 1.0);
    } else if let Some((start_hz, end_hz, t0)) = bw_fade {
        // After release: fade the frozen span out over BW_FADE_SECS, anchored to
        // its frequencies (so it tracks the view if you pan meanwhile).
        let elapsed = ui.input(|i| i.time) - t0;
        if elapsed < BW_FADE_SECS {
            let alpha = (1.0 - elapsed / BW_FADE_SECS) as f32;
            draw_bw_measure(
                &painter,
                |hz| view.freq_to_x(hz, &rect),
                &rect,
                start_hz,
                end_hz,
                alpha,
            );
            crate::repaint::animate(ui.ctx()); // keep animating the fade to completion
        } else {
            ui.data_mut(|d| d.insert_temp(fade_id, None::<(f64, f64, f64)>));
        }
    }

    // Chrome: pink cut-corner border + corner accents around the panadapter.
    crate::chrome::paint_cut_border(
        &painter,
        rect.shrink(0.8),
        crate::theme::scope().chrome,
        crate::theme::scope().shell,
    );
    crate::chrome::corner_brackets(&painter, rect, crate::theme::scope().chrome);
}

/// How long the released bandwidth measurement lingers while fading out.
pub(crate) const BW_FADE_SECS: f64 = 5.0;

/// Draw the shift+drag bandwidth-measurement overlay: dotted vertical markers at
/// the start and end frequencies, a horizontal span line with end ticks, and the
/// measured bandwidth as a centred label. `alpha` fades the whole thing out.
///
/// Takes the frequency→screen mapping as a closure rather than a [`ViewState`]:
/// the full-band strip ([`crate::widgets::wide_spectrum`]) offers the same
/// gesture over an axis that is not a panadapter viewport, and the ruler should
/// look and read identically on both.
pub(crate) fn draw_bw_measure(
    p: &egui::Painter,
    x_of: impl Fn(f64) -> f32,
    rect: &Rect,
    start_hz: f64,
    end_hz: f64,
    alpha: f32,
) {
    let a = alpha.clamp(0.0, 1.0);
    let base = crate::theme::scope().warn;
    let color = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), (255.0 * a) as u8);
    let stroke = Stroke::new(1.5, color);

    let x0 = x_of(start_hz).clamp(rect.left(), rect.right());
    let x1 = x_of(end_hz).clamp(rect.left(), rect.right());
    let bw = (end_hz - start_hz).abs();

    // Dotted vertical markers spanning the whole panadapter (spectrum + scale +
    // waterfall), so the tool works whether or not the spectrum line is shown.
    for x in [x0, x1] {
        p.extend(Shape::dashed_line(
            &[pos2(x, rect.top() + 1.0), pos2(x, rect.bottom() - 1.0)],
            stroke,
            2.5,
            3.5,
        ));
    }
    // Horizontal span line with end ticks.
    let (lo, hi) = (x0.min(x1), x0.max(x1));
    let yl = rect.top() + 26.0;
    p.line_segment([pos2(lo, yl), pos2(hi, yl)], stroke);
    for x in [x0, x1] {
        p.line_segment([pos2(x, yl - 4.0), pos2(x, yl + 4.0)], stroke);
    }
    // Start / end frequency labels at their markers. When the markers are close
    // the two labels would overlap, so push them symmetrically apart (keeping the
    // pair on-screen) so both stay readable.
    let s_text = fmt_mhz(start_hz);
    let e_text = fmt_mhz(end_hz);
    let font = FontId::monospace(10.5 * crate::theme::panadapter_font_scale());
    let sw = p.layout_no_wrap(s_text.clone(), font.clone(), color).size().x + 8.0;
    let ew = p.layout_no_wrap(e_text.clone(), font, color).size().x + 8.0;
    // Order the two labels left→right by marker x.
    let ((lx, lw, lt), (rx, rw, rt)) = if x0 <= x1 {
        ((x0, sw, &s_text), (x1, ew, &e_text))
    } else {
        ((x1, ew, &e_text), (x0, sw, &s_text))
    };
    let (mut cl, mut cr) = (lx, rx);
    let needed = (lw + rw) * 0.5 + 4.0; // min centre distance for a 4px gap
    if cr - cl < needed {
        let mid = (lx + rx) * 0.5;
        cl = mid - needed * 0.5;
        cr = mid + needed * 0.5;
    }
    // Shift the pair as a unit to keep both on-screen without re-overlapping.
    let over_l = rect.left() - (cl - lw * 0.5);
    if over_l > 0.0 {
        cl += over_l;
        cr += over_l;
    }
    let over_r = (cr + rw * 0.5) - rect.right();
    if over_r > 0.0 {
        cl -= over_r;
        cr -= over_r;
    }
    let fy = rect.top() + 4.0;
    faded_label(p, rect, cl, fy, lt, color, a);
    faded_label(p, rect, cr, fy, rt, color, a);
    // Bandwidth centred below the span line.
    faded_label(p, rect, (lo + hi) * 0.5, yl + 6.0, &format_bandwidth(bw), color, a);
}

/// Zoom the view about a fixed point on screen, keeping the frequency under `x`
/// under `x`. `factor` below 1 narrows the span (zooms in).
///
/// The gesture the wheel and a two-finger pinch both are, so they cannot come
/// to disagree about where a zoom is anchored or how far it may go.
fn zoom_about(view: &mut ViewState, x: f32, rect: &Rect, factor: f64, dev_span: f64) {
    let fpos = view.x_to_freq(x, rect);
    let span = view.span();
    if span <= 0.0 {
        return;
    }
    // Past this the FFT has no more detail to give and the view would go on
    // magnifying a single bin.
    let min_span = (dev_span / 1024.0).max(1_000.0);
    // ⛔ The floor applies to zooming *in* and to nothing else, and it is a
    // clamp rather than a refusal.
    //
    // This read `if hi - lo >= min_span { … }` until 25 August 2026 — the new
    // span tested against the floor whichever way the wheel had turned. A view
    // already *below* the floor was therefore frozen solid: every notch out
    // multiplied a too-small span by about 1.02 and landed short of the floor
    // as well, so it was rejected with the same test that was meant to stop
    // zooming in, and there was no gesture left that could climb out.
    //
    // Which is not a hypothetical: `min_span` is a fraction of the *front
    // end's* span, so it moves when the front end does. A view left at 24 kHz
    // by an Icom sending its 12 kHz IF, carried onto an RX-888 at 32.4 Msps,
    // meets a floor of 31.6 kHz — above where it already sits — and the
    // panadapter will not zoom at all, in either direction, for the rest of
    // the session and every session after it, because the span is persisted.
    let want = span * factor;
    let new_span = if factor < 1.0 {
        // Zooming in: never below the floor — and never below where the view
        // already is, so a view that starts under the floor is held rather
        // than shoved out to a width the operator did not ask for.
        want.max(min_span.min(span))
    } else {
        // Zooming out: always allowed. `ViewState::clamp_to` bounds it at the
        // captured window, which is the only limit that belongs on this side.
        want
    };
    let k = new_span / span;
    view.view_lo_hz = fpos - (fpos - view.view_lo_hz) * k;
    view.view_hi_hz = fpos + (view.view_hi_hz - fpos) * k;
}

/// A frequency in Hz as `xx.xxxxx MHz`.
fn fmt_mhz(hz: f64) -> String {
    format!("{:.5} MHz", hz / 1e6)
}

/// Draw `text` in a small box centred on `cx`, top at `y`, clamped to `rect`,
/// with a background whose opacity scales with `a` (for the fade-out).
fn faded_label(
    p: &egui::Painter,
    rect: &Rect,
    cx: f32,
    y: f32,
    text: &str,
    color: Color32,
    a: f32,
) {
    let galley = p.layout_no_wrap(
        text.to_string(),
        FontId::monospace(10.5 * crate::theme::panadapter_font_scale()),
        color,
    );
    let pad = vec2(4.0, 2.0);
    let size = galley.size() + pad * 2.0;
    let bx = (cx - size.x * 0.5).clamp(rect.left(), rect.right() - size.x);
    let by = y.clamp(rect.top(), rect.bottom() - size.y);
    p.rect_filled(
        Rect::from_min_size(pos2(bx, by), size),
        2.0,
        Color32::from_rgba_unmultiplied(0, 0, 0, (180.0 * a) as u8),
    );
    p.galley(pos2(bx + pad.x, by + pad.y), galley, color);
}

/// Format a bandwidth (Hz) as a compact human string.
fn format_bandwidth(hz: f64) -> String {
    if hz >= 1_000_000.0 {
        // The ruler also runs over the full-band strip, where a span of several
        // megahertz is the ordinary case and four digits of kilohertz is not a
        // number anyone reads.
        format!("{:.3} MHz", hz / 1e6)
    } else if hz >= 10_000.0 {
        format!("{:.1} kHz", hz / 1000.0)
    } else if hz >= 1_000.0 {
        format!("{:.2} kHz", hz / 1000.0)
    } else {
        format!("{:.0} Hz", hz)
    }
}

/// A vertical marker line across the spectrum + waterfall strips (dashed for a
/// reference, solid for a marker); the waterfall copy is faded so it doesn't
/// obscure the trace.
fn marker_line(
    p: &egui::Painter,
    spec: &Rect,
    wf: &Rect,
    spec_h: f32,
    x: f32,
    color: Color32,
    dashed: bool,
) {
    let wf_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150);
    let draw = |a: Pos2, b: Pos2, c: Color32| {
        if dashed {
            p.extend(Shape::dashed_line(&[a, b], Stroke::new(1.0, c), 3.0, 3.0));
        } else {
            p.line_segment([a, b], Stroke::new(1.0, c));
        }
    };
    if spec_h > 1.0 {
        draw(pos2(x, spec.top()), pos2(x, spec.bottom()), color);
    }
    draw(pos2(x, wf.top()), pos2(x, wf.bottom()), wf_color);
}

/// A labelled offset bracket: a horizontal line with end ticks between two x
/// positions, plus the label just past its right end.
fn offset_bracket(
    p: &egui::Painter,
    bounds: Rect,
    xf: f32,
    xt: f32,
    y: f32,
    color: Color32,
    label: &str,
) {
    let xf = xf.clamp(bounds.left(), bounds.right());
    let xt = xt.clamp(bounds.left(), bounds.right());
    let (lo, hi) = (xf.min(xt), xf.max(xt));
    let s = Stroke::new(1.5, color);
    p.line_segment([pos2(lo, y), pos2(hi, y)], s);
    p.line_segment([pos2(xf, y - 3.0), pos2(xf, y + 3.0)], s);
    p.line_segment([pos2(xt, y - 3.0), pos2(xt, y + 3.0)], s);
    label_box(p, pos2(hi + 4.0, y - 6.0), label, color, bounds);
}

/// Draw `text` in a small semi-transparent black box, clamped inside `bounds`.
pub(crate) fn label_box(p: &egui::Painter, top_left: Pos2, text: &str, fg: Color32, bounds: Rect) {
    let galley = p.layout_no_wrap(
        text.to_string(),
        FontId::monospace(11.0 * crate::theme::panadapter_font_scale()),
        fg,
    );
    let pad = vec2(4.0, 2.0);
    let size = galley.size() + pad * 2.0;
    let x = (top_left.x).min(bounds.right() - size.x).max(bounds.left());
    let y = (top_left.y).clamp(bounds.top(), bounds.bottom() - size.y);
    let bg = Rect::from_min_size(pos2(x, y), size);
    p.rect_filled(bg, 2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));
    p.galley(pos2(x + pad.x, y + pad.y), galley, fg);
}

/// The value a left-click at screen `x` would set: the audio TX offset in a
/// digital mode, else the (10 Hz-rounded) dial frequency it would tune to.
fn click_tune_label(
    view: &ViewState,
    state: &RadioState,
    rect: &Rect,
    x: f32,
    cursor: Option<AudioCursor>,
) -> String {
    let hz = view.x_to_freq(x, rect);
    if cursor.is_some_and(|c| c.click_sets_offset) {
        let audio = (hz - state.rx_freq_hz()).clamp(200.0, 3500.0);
        format!("{audio:.0} Hz")
    } else {
        // The label is the frequency of the *signal*, which in CW is a pitch
        // above the dial the click will actually set.
        let tuned = (hz / CLICK_TUNE_STEP).round() * CLICK_TUNE_STEP;
        format!("{:.5} MHz", tuned / 1e6)
    }
}

/// Per-pixel polyline of the frame's bins (or of `values` when given, e.g.
/// the peak-hold bins) mapped through the current viewport. This is the
/// expensive path the [`TraceCache`] avoids on unchanged repaints.
///
/// One vertex per *device* pixel, not per egui point. On a HiDPI panel the two
/// differ by the zoom factor, and a polyline sampled in points is drawn with
/// every segment spanning two pixels — the trace's own share of the coarse
/// picture of issue #172, and the reason it stair-stepped next to a waterfall
/// that had just been widened.
fn compute_trace(
    view: &ViewState,
    f: &SpectrumFrame,
    values: Option<&[f32]>,
    rect: &Rect,
    ppp: f32,
) -> Vec<Pos2> {
    let n = values.map(|v| v.len()).unwrap_or(f.bins.len());
    if n == 0 {
        return Vec::new();
    }
    let base = f.center_hz - f.span_hz / 2.0;
    let ppp = ppp.clamp(1.0, 8.0);
    let w = ((rect.width() * ppp).max(1.0)) as usize;
    let mut points = Vec::with_capacity(w);
    for px in 0..w {
        let x = rect.left() + px as f32 / ppp;
        let hz = view.x_to_freq(x, rect);
        let bin_f = (hz - base) / f.span_hz * n as f64;
        let v = if (0.0..n as f64).contains(&bin_f) {
            let i = bin_f as usize;
            let raw = match values {
                Some(v) => v[i],
                None => f.bins[i] as f32,
            };
            raw / 255.0
        } else {
            0.0
        };
        points.push(Pos2::new(x, rect.bottom() - v * rect.height()));
    }
    points
}

fn draw_grid(painter: &egui::Painter, view: &ViewState, rect: &Rect) {
    let grid = Stroke::new(0.5, crate::theme::scope_gray(42));
    // dB lines every 20 dB of the display range.
    let range = view.db_ceil - view.db_floor;
    if range > 1.0 {
        let mut db = (view.db_floor / 20.0).ceil() * 20.0;
        while db < view.db_ceil {
            let frac = (db - view.db_floor) / range;
            let y = rect.bottom() - frac * rect.height();
            painter.hline(rect.x_range(), y, grid);
            painter.text(
                pos2(rect.left() + 2.0, y - 1.0),
                Align2::LEFT_BOTTOM,
                format!("{db:.0}"),
                FontId::monospace(9.0 * crate::theme::panadapter_font_scale()),
                crate::theme::scope_gray(110),
            );
            db += 20.0;
        }
    }
    let step = freq_grid_step_for_width(view.view_lo_hz, view.view_hi_hz, rect.width());
    for hz in gridlines_at(view.view_lo_hz, view.view_hi_hz, step) {
        let x = view.freq_to_x(hz, rect);
        painter.vline(x, rect.y_range(), grid);
    }
}

fn draw_scale(painter: &egui::Painter, view: &ViewState, rect: &Rect) {
    painter.rect_filled(*rect, 0.0, Color32::from_gray(20));
    let step = freq_grid_step_for_width(view.view_lo_hz, view.view_hi_hz, rect.width());
    // Unlabelled ticks between the labelled ones, the way the full-band strip
    // above already draws its scale. A label every hundred points is as many
    // numbers as the strip can hold, but placing a signal against the axis by
    // eye wants marks in between — that is the other half of what issue #174
    // asked for, and a tick costs no width at all.
    for hz in gridlines_at(view.view_lo_hz, view.view_hi_hz, minor_grid_step(step)) {
        painter.vline(
            view.freq_to_x(hz, rect),
            egui::Rangef::new(rect.top(), rect.top() + 3.0),
            Stroke::new(1.0, crate::theme::scope_gray(80)),
        );
    }
    for hz in gridlines_at(view.view_lo_hz, view.view_hi_hz, step) {
        let x = view.freq_to_x(hz, rect);
        painter.vline(
            x,
            egui::Rangef::new(rect.top(), rect.top() + 5.0),
            Stroke::new(1.0, crate::theme::scope_gray(120)),
        );
        painter.text(
            pos2(x, rect.center().y + 2.0),
            Align2::CENTER_CENTER,
            format!("{:.4}", hz / 1e6),
            FontId::monospace(10.0 * crate::theme::panadapter_font_scale()),
            crate::theme::scope_gray(190),
        );
    }
}

/// Spacing for the waterfall's time gridlines, given how many seconds of
/// history are on screen.
///
/// Steps a human reads as round times rather than a computed interval: nobody
/// wants a line every 37 seconds. Aims for roughly four across the visible
/// height, so the labels are frequent enough to place a signal in time without
/// covering the waterfall they annotate.
fn time_grid_step_s(visible_secs: f64) -> f64 {
    const STEPS: [f64; 12] =
        [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0];
    let target = visible_secs / 4.0;
    // The smallest round step at or above the target, so the count never runs
    // away on a tall window; the largest we have if even an hour is too fine.
    *STEPS.iter().find(|&&s| s >= target).unwrap_or(STEPS.last().expect("non-empty"))
}

/// Gridline spacing for a frequency range: a 1/2/5·10^k step giving ~5–10 lines.
///
/// Takes bare edges rather than a [`ViewState`] so the full-band strip
/// ([`crate::widgets::wide_spectrum`]), whose axis is the front end's whole
/// window rather than a panadapter viewport, divides its scale the same way.
pub(crate) fn freq_grid_step(lo_hz: f64, hi_hz: f64) -> f64 {
    round_step((hi_hz - lo_hz) / 8.0)
}

/// The smallest 1/2/5·10^k step at or above `raw`, so a computed spacing comes
/// out as a frequency a human reads as round.
fn round_step(raw: f64) -> f64 {
    let mag = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0].iter().map(|m| m * mag).find(|&s| s >= raw).unwrap_or(mag * 10.0)
}

/// Points of panadapter a labelled frequency marker is given to itself.
///
/// The labels are `{:.4}` MHz in monospace at [`crate::theme::panadapter_font_scale`],
/// so the widest of them — a 10 GHz dial, ten characters — is around 60 points
/// at the smallest step of the three. This leaves room for that and a gap.
const FREQ_LABEL_PITCH: f32 = 96.0;

/// [`freq_grid_step`] for an axis `width_px` points wide.
///
/// The fixed eighth was a spacing chosen for no particular screen: across a
/// wide monitor it leaves four or five labels over a thousand points, which is
/// what issue #174 asked for more of, while on a phone the same eighth ran the
/// labels into each other. Dividing by the width instead gives one marker per
/// [`FREQ_LABEL_PITCH`] points wherever the panadapter is drawn, and the 1/2/5
/// rounding keeps them on round frequencies.
///
/// Bounded either way: at least three markers, so even the narrowest strip has
/// an axis rather than a lone number, and at most twenty, because past that
/// they are a texture and not a scale.
pub(crate) fn freq_grid_step_for_width(lo_hz: f64, hi_hz: f64, width_px: f32) -> f64 {
    let scale = crate::theme::panadapter_font_scale();
    let lines = (width_px / (FREQ_LABEL_PITCH * scale)).clamp(3.0, 20.0) as f64;
    round_step((hi_hz - lo_hz) / lines)
}

/// Tick spacing between two labelled gridlines: a fifth of the step, or a half
/// when the step is 2·10^k — the two that land on round frequencies.
pub(crate) fn minor_grid_step(step_hz: f64) -> f64 {
    let mag = 10f64.powf(step_hz.log10().floor());
    if (step_hz / mag).round() == 2.0 { step_hz / 2.0 } else { step_hz / 5.0 }
}

/// Gridline frequencies inside a range, at `step`.
pub(crate) fn gridlines_at(lo_hz: f64, hi_hz: f64, step: f64) -> Vec<f64> {
    let mut out = Vec::new();
    if !step.is_finite() || step <= 0.0 || hi_hz <= lo_hz {
        return out;
    }
    let mut hz = (lo_hz / step).ceil() * step;
    while hz <= hi_hz {
        out.push(hz);
        hz += step;
    }
    out
}

/// Gridline frequencies inside a range, at [`freq_grid_step`].
pub(crate) fn freq_gridlines(lo_hz: f64, hi_hz: f64) -> Vec<f64> {
    gridlines_at(lo_hz, hi_hz, freq_grid_step(lo_hz, hi_hz))
}

#[cfg(test)]
mod centring_tests {
    use super::*;

    fn view_spanning(lo: f64, hi: f64) -> ViewState {
        ViewState { view_lo_hz: lo, view_hi_hz: hi, ..ViewState::default() }
    }

    /// Zoomed in, centring is free: the window is a viewport onto a wider
    /// captured span, so it slides under the dial and the receiver is never
    /// told about it.
    #[test]
    fn centring_a_zoomed_view_asks_the_front_end_for_nothing() {
        // 20 kHz of a 1 MHz window, and the dial 5 kHz above the middle of it.
        let mut v = view_spanning(990_000.0, 1_010_000.0);
        let over = center_on_dial(&mut v, 1_005_000.0, 1_000_000.0, 1_000_000.0, 1000.0);
        assert_eq!(over, 0.0, "a zoomed-in slide reached the hardware");
        assert_eq!((v.view_lo_hz, v.view_hi_hz), (995_000.0, 1_015_000.0));
    }

    /// Zoomed right out there is nowhere left to slide, so the whole move is
    /// handed to the front end — which is the part that makes the picture
    /// scroll instead of jumping a window at a time (issue #174).
    #[test]
    fn centring_at_full_zoom_out_hands_the_whole_move_over() {
        let mut v = view_spanning(0.0, 1_000_000.0);
        let over = center_on_dial(&mut v, 520_000.0, 500_000.0, 1_000_000.0, 1000.0);
        assert!((over - 20_000.0).abs() < 0.5, "the front end was asked for {over} Hz, not 20 kHz");
    }

    /// A slide narrower than one column of the picture is not worth a retune:
    /// the view still takes it — so the remainders add up rather than being
    /// thrown away — but the front end is left alone until they come to a
    /// pixel.
    #[test]
    fn a_sub_pixel_slide_leaves_the_front_end_alone() {
        // 1 Msps across 1000 points is a kilohertz a column; the dial moves 100 Hz.
        let mut v = view_spanning(0.0, 1_000_000.0);
        let over = center_on_dial(&mut v, 500_100.0, 500_000.0, 1_000_000.0, 1000.0);
        assert_eq!(over, 0.0, "a tenth of a column was sent to the hardware");
        // Ten of them are a column, and by then it is.
        let mut moved = 0.0;
        for i in 1..=10 {
            let mut v = view_spanning(0.0, 1_000_000.0);
            moved = center_on_dial(&mut v, 500_000.0 + 100.0 * i as f64, 500_000.0, 1e6, 1000.0);
        }
        assert!((moved - 1_000.0).abs() < 0.5, "a whole column came out as {moved} Hz");
    }
}

#[cfg(test)]
mod axis_tests {
    use super::*;

    /// A wide panadapter earns more frequency markers than the fixed eighth
    /// gave it, which is what issue #174 asked for; a phone-width strip, which
    /// the same eighth crowded, gets fewer.
    #[test]
    fn the_marker_count_follows_the_width() {
        let (lo, hi) = (14_000_000.0, 14_350_000.0);
        let eighth = gridlines_at(lo, hi, freq_grid_step(lo, hi)).len();
        let wide = gridlines_at(lo, hi, freq_grid_step_for_width(lo, hi, 1800.0)).len();
        let narrow = gridlines_at(lo, hi, freq_grid_step_for_width(lo, hi, 360.0)).len();
        assert!(wide > eighth, "a 1800 pt axis got {wide} markers against the old {eighth}");
        assert!(narrow < eighth, "a 360 pt axis kept {narrow} markers against {eighth}");
        assert!(narrow >= 2, "a 360 pt axis was left with {narrow} markers");
    }

    /// However wide the axis and whatever it spans, two labelled markers never
    /// come closer than a label is wide.
    #[test]
    fn the_labels_never_touch() {
        for width in [320.0f32, 640.0, 1280.0, 2560.0, 3840.0] {
            for span in [3_000.0f64, 48_000.0, 350_000.0, 2_000_000.0, 32_400_000.0] {
                let (lo, hi) = (14_000_000.0, 14_000_000.0 + span);
                let step = freq_grid_step_for_width(lo, hi, width);
                let pitch = width as f64 * step / span;
                assert!(
                    pitch >= 90.0,
                    "a {width} pt axis over {span} Hz puts labels {pitch} pt apart",
                );
            }
        }
    }

    /// The minor ticks land on round frequencies between the labelled ones —
    /// four or one of them, never a count that puts a tick off the grid.
    #[test]
    fn the_minor_ticks_divide_the_step_evenly() {
        for step in [1.0f64, 2.0, 5.0, 10.0, 200.0, 500.0, 1_000.0, 2_000_000.0] {
            let minor = minor_grid_step(step);
            let n = step / minor;
            assert!((n - n.round()).abs() < 1e-9, "{step} Hz divides into {n} ticks");
            assert!(matches!(n.round() as i64, 2 | 5), "{step} Hz got {n} ticks");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of `bins` bins over 800 Hz centred on `center`: one bin per
    /// 100 Hz, so the arithmetic in these tests is readable.
    fn frame_at(center: f64, bins: Vec<u8>) -> SpectrumFrame {
        SpectrumFrame {
            seq: 0,
            center_hz: center,
            span_hz: 100.0 * bins.len() as f64,
            db_floor: -120.0,
            db_ceil: -20.0,
            bins,
            rows: Vec::new(),
            rows_clocked: false,
        }
    }

    /// The window moves up two bins: the trace's own history moves down two,
    /// and the two bins of band that have just come into view are taken from
    /// the frame because that is the only place they exist.
    #[test]
    fn a_smoothed_trace_travels_with_the_window() {
        let mut bins = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let f = frame_at(1200.0, vec![91, 92, 93, 94, 95, 96, 97, 98]);
        assert!(slide_bins(&mut bins, 1000.0, &f));
        assert_eq!(bins, [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 97.0, 98.0]);
    }

    /// And the same the other way down the band.
    #[test]
    fn a_trace_travels_the_other_way_too() {
        let mut bins = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let f = frame_at(900.0, vec![91, 92, 93, 94, 95, 96, 97, 98]);
        assert!(slide_bins(&mut bins, 1000.0, &f));
        assert_eq!(bins, [91.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    /// A window that has moved clear of where it was has nothing left to
    /// carry, and the caller starts again from the frame.
    #[test]
    fn a_window_moved_clear_of_itself_keeps_nothing() {
        let mut bins = [1.0, 2.0, 3.0, 4.0];
        let f = frame_at(2000.0, vec![9, 9, 9, 9]);
        assert!(!slide_bins(&mut bins, 1000.0, &f));
    }

    /// What the drag looked like before this: the average was thrown away on
    /// every frame the window moved, which is every frame of the drag, so the
    /// trace ran raw for as long as the operator held the mouse down. One
    /// step of the fold has to carry the history across the move.
    #[test]
    fn the_average_survives_a_window_that_moves_every_frame() {
        let mut smooth = SpectrumSmooth::default();
        // Settle on a steady band well away from the noisy bin 4.
        for seq in 0..40u32 {
            let mut f = frame_at(1000.0, vec![100; 8]);
            f.seq = seq;
            smooth.update(&f, 0.2);
        }
        // Now move the window one bin per frame while bin 4 of each frame is
        // wildly off. Smoothed, its excursion has to stay small.
        let mut worst: f32 = 0.0;
        for (i, seq) in (40..60u32).enumerate() {
            let mut bins = vec![100u8; 8];
            bins[4] = 250;
            let mut f = frame_at(1000.0 + 100.0 * i as f64, bins);
            f.seq = seq;
            smooth.update(&f, 0.2);
            worst = worst.max(smooth.bins[4]);
        }
        assert!(worst < 160.0, "the average was restarted mid-drag: bin reached {worst}");
    }

    /// Zoom the view about its own centre, the way a wheel notch over the
    /// middle of the panadapter does.
    fn wheel(view: &mut ViewState, factor: f64, dev_span: f64) {
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 400.0));
        zoom_about(view, 500.0, &rect, factor, dev_span);
    }

    fn view_spanning(lo: f64, hi: f64) -> ViewState {
        let mut v = ViewState::default();
        v.view_lo_hz = lo;
        v.view_hi_hz = hi;
        v
    }

    /// The measured fault. A view left at 24 kHz by an Icom sending its 12 kHz
    /// IF, carried onto an RX-888 at 32.4 Msps, sits below that front end's
    /// 31.6 kHz zoom floor — and the old guard tested the *new* span against
    /// the floor whichever way the wheel turned, so every notch out landed
    /// short of it too and was refused. The panadapter would not zoom in either
    /// direction, for good, because the span is persisted across sessions.
    #[test]
    fn a_view_below_the_floor_can_still_zoom_out() {
        const DEV: f64 = 32_400_000.0;
        assert!(24_000.0 < (DEV / 1024.0).max(1_000.0), "the fault needs a view under the floor");
        let mut v = view_spanning(16_086_700.0, 16_110_700.0);
        // One notch out, at the wheel's own ~2% per notch.
        wheel(&mut v, 1.02, DEV);
        assert!(v.span() > 24_000.0, "a notch out was refused: span stuck at {}", v.span());
        // And it keeps climbing, right through the floor it started under.
        for _ in 0..200 {
            wheel(&mut v, 1.02, DEV);
        }
        assert!(v.span() > DEV / 1024.0, "the view never climbed past the floor");
    }

    #[test]
    fn a_view_below_the_floor_is_held_rather_than_shrunk_or_shoved() {
        const DEV: f64 = 32_400_000.0;
        let mut v = view_spanning(16_086_700.0, 16_110_700.0);
        wheel(&mut v, 0.98, DEV);
        // Not shrunk further — but not yanked out to the floor either, which
        // would be a zoom the operator did not ask for.
        assert!((v.span() - 24_000.0).abs() < 1.0, "span moved to {}", v.span());
    }

    /// The floor still does its job on an ordinary view: zooming in stops at
    /// it instead of magnifying a single bin, and stops *at* it rather than
    /// refusing the notch that would cross it.
    #[test]
    fn zooming_in_clamps_at_the_floor() {
        const DEV: f64 = 32_400_000.0;
        let floor = (DEV / 1024.0).max(1_000.0);
        let mut v = view_spanning(16_000_000.0, 17_000_000.0);
        for _ in 0..2000 {
            wheel(&mut v, 0.98, DEV);
        }
        assert!(v.span() >= floor - 1.0, "zoomed past the floor to {}", v.span());
        assert!(v.span() < floor * 1.05, "stopped short of the floor at {}", v.span());
    }

    /// Zooming out is bounded by the captured window, not by this function —
    /// `clamp_to` owns that side, and it must still be reachable.
    #[test]
    fn zooming_out_reaches_the_whole_window() {
        const DEV: f64 = 32_400_000.0;
        let mut v = view_spanning(16_000_000.0, 16_100_000.0);
        for _ in 0..500 {
            wheel(&mut v, 1.02, DEV);
        }
        v.clamp_to(DEV / 2.0, DEV);
        assert!(v.span() >= DEV * 0.99, "never reached the full window: {}", v.span());
    }

    /// The bug this replaced: the step was a fixed sixty seconds, and at the
    /// default scroll speed a minute is about 1700 pixels of waterfall — so on
    /// any ordinary window no boundary was ever on screen and the waterfall
    /// read as having no time axis at all.
    ///
    /// Whatever the speed and however tall the window, some labels have to be
    /// visible, and not so many they bury the waterfall.
    #[test]
    fn the_time_axis_stays_readable_at_every_scroll_speed() {
        // Slow, medium and fast, over short and tall waterfalls.
        for rows_per_sec in [5.0f64, 28.0, 56.0] {
            for height_px in [120.0f64, 300.0, 900.0] {
                let visible = height_px / rows_per_sec;
                let step = time_grid_step_s(visible);
                let lines = visible / step;
                assert!(
                    (1.0..=8.0).contains(&lines),
                    "{rows_per_sec} rows/s over {height_px} px shows {lines:.1} gridlines \
                     ({step} s apart)"
                );
            }
        }
    }

    /// A span reads in the unit its size calls for: a signal's width in Hz or
    /// kHz on the panadapter, a band's in MHz on the full-band strip.
    #[test]
    fn a_span_reads_in_the_unit_its_size_calls_for() {
        assert_eq!(format_bandwidth(300.0), "300 Hz");
        assert_eq!(format_bandwidth(2_400.0), "2.40 kHz");
        assert_eq!(format_bandwidth(48_000.0), "48.0 kHz");
        assert_eq!(format_bandwidth(5_731_100.0), "5.731 MHz");
    }

    /// And the steps stay times a person reads as round.
    #[test]
    fn the_time_axis_steps_are_round_numbers() {
        for visible in [2.0f64, 5.0, 11.0, 60.0, 200.0, 4000.0] {
            let s = time_grid_step_s(visible);
            assert!(
                [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0]
                    .contains(&s),
                "{visible} s of history gave a {s} s step"
            );
        }
        // Past the end of the ladder it saturates rather than vanishing.
        assert_eq!(time_grid_step_s(1e9), 3600.0);
    }

    /// How long a dial released at `v0` points/second keeps turning, and how far
    /// it travels, stepped at a plausible frame rate.
    fn coast(v0: f32) -> (f32, f32) {
        let dt = 1.0 / 60.0;
        let (mut vx, mut t, mut px) = (v0, 0.0, 0.0);
        while vx.abs() >= FLING_STOP {
            vx = fling_decay(vx, dt);
            px += vx * dt;
            t += dt;
            assert!(t < 60.0, "a coast from {v0} px/s never stopped");
        }
        (t, px)
    }

    /// The point of the whole thing: a faster flick spins the dial for longer
    /// and carries it further, the way a weighted VFO knob does.
    #[test]
    fn a_faster_flick_coasts_longer_and_further() {
        let (slow_t, slow_px) = coast(400.0);
        let (mid_t, mid_px) = coast(1500.0);
        let (fast_t, fast_px) = coast(4000.0);
        assert!(slow_t < mid_t && mid_t < fast_t, "{slow_t} {mid_t} {fast_t}");
        assert!(slow_px < mid_px && mid_px < fast_px, "{slow_px} {mid_px} {fast_px}");
        // And it is a dial, not a slingshot: even a hard flick settles in a
        // couple of seconds rather than drifting across the band.
        assert!(fast_t < 3.5, "a hard flick should still settle promptly, took {fast_t}s");
    }

    /// A coast only ever runs down. Reversing direction would read as the dial
    /// bouncing off something, and at a long frame the naive decay does exactly
    /// that.
    #[test]
    fn a_coast_never_spins_backwards() {
        for v0 in [-4000.0f32, -300.0, 300.0, 4000.0] {
            let mut vx = v0;
            for _ in 0..200 {
                // Deliberately long frames: the worst case the clamp allows.
                let next = fling_decay(vx, 0.1);
                assert!(next == 0.0 || next.signum() == v0.signum(), "{v0} reversed to {next}");
                assert!(next.abs() <= vx.abs(), "{v0} sped up: {vx} -> {next}");
                vx = next;
            }
            assert_eq!(vx, 0.0, "a coast from {v0} px/s should have run out");
        }
    }

    /// A view with room to move takes the whole pan and leaves the front end
    /// alone: zoomed in, the window is a resource and panning inside it is free.
    #[test]
    fn a_pan_inside_the_window_asks_the_front_end_for_nothing() {
        let mut view = ViewState { view_lo_hz: 14_100_000.0, view_hi_hz: 14_200_000.0, ..def() };
        let over = pan_view(&mut view, 30_000.0, 14_150_000.0, 2_000_000.0);
        assert_eq!(over, 0.0, "a pan with room to move must not disturb the hardware");
        assert_eq!((view.view_lo_hz, view.view_hi_hz), (14_130_000.0, 14_230_000.0));
    }

    /// Issue #133: fully zoomed out the view *is* the window, so every pixel of
    /// the drag has to go to the front end — otherwise the picture stands still
    /// however far the operator drags it.
    #[test]
    fn a_pan_at_full_zoom_out_goes_entirely_to_the_front_end() {
        let (center, span) = (14_150_000.0, 2_000_000.0);
        let mut view = ViewState { view_lo_hz: 13_150_000.0, view_hi_hz: 15_150_000.0, ..def() };
        for dhz in [50_000.0, -50_000.0] {
            let mut v = view.clone();
            assert_eq!(pan_view(&mut v, dhz, center, span), dhz, "at dhz={dhz}");
        }
        // And the centre goes with it, so the window this frame is clamped to is
        // the one being asked for.
        let mut state = RadioState { center_hz: center, sample_rate: span, ..state() };
        let mut dev_center = center;
        let mut cmds = Vec::new();
        let over = pan_view(&mut view, 50_000.0, dev_center, span);
        pan_center(&mut dev_center, &mut state, over, movable(), &mut cmds);
        assert_eq!(dev_center, 14_200_000.0);
        assert_eq!(state.center_hz, 14_200_000.0, "the echo the clamp reads this frame");
        assert_eq!(cmds, vec![Command::SetCenter(14_200_000.0)]);
        view.clamp_to(dev_center, span);
        assert_eq!(
            (view.view_lo_hz, view.view_hi_hz),
            (13_200_000.0, 15_200_000.0),
            "the pan must survive the clamp, or the drag fights itself"
        );
    }

    /// Several frames of one held drag, run in the order the widget runs them:
    /// the pan bounds are read at the top of the frame, the overshoot goes to
    /// the front end in the middle, and the view is clamped at the bottom.
    ///
    /// The clamp has to be against the window as it is *then*. Against the
    /// frame's opening snapshot it pulls the view back over exactly the
    /// overshoot the centre has just moved by, so the next frame finds the view
    /// outside the window on the *other* side and moves the centre back:
    /// window and view chase each other one frame apart, and the panadapter
    /// jitters sideways for as long as the button is held. Measured on a
    /// 1.5 Msps generator before the fix — a steady 24-step drag reversed
    /// direction on 40 of its 50 frames, travelled 177 kHz of the 298 kHz asked
    /// for, and went on oscillating ±76 kHz with the pointer standing still.
    #[test]
    fn a_held_drag_moves_one_way_only() {
        let (center, span) = (14_150_000.0, 2_000_000.0);
        let step = 25_000.0;
        let mut view =
            ViewState { view_lo_hz: center - span / 2.0, view_hi_hz: center + span / 2.0, ..def() };
        let mut state = RadioState { center_hz: center, sample_rate: span, ..state() };
        let mut dev_center = center;
        let pan = movable();
        let mut sent: Vec<f64> = Vec::new();

        for _ in 0..6 {
            let mut cmds = Vec::new();
            let (zoom_center, zoom_span) = pan.view_bounds(dev_center, span);
            let over = pan_view(&mut view, step, zoom_center, zoom_span);
            pan_center(&mut dev_center, &mut state, over, pan, &mut cmds);
            let (clamp_center, clamp_span) = pan.view_bounds(dev_center, span);
            view.clamp_to(clamp_center, clamp_span);
            sent.extend(cmds.iter().filter_map(|c| match c {
                Command::SetCenter(hz) => Some(*hz),
                _ => None,
            }));
        }

        assert_eq!(sent.len(), 6, "one retune a frame: {sent:?}");
        for (a, b) in sent.iter().zip(sent.iter().skip(1)) {
            assert!(b > a, "the drag reversed on itself: {sent:?}");
        }
        assert!(
            (dev_center - (center + 6.0 * step)).abs() < 0.5,
            "six {step} Hz steps should have travelled {}, got {}",
            6.0 * step,
            dev_center - center,
        );
    }

    /// The same drag on a front end with a full-band lane — a KiwiSDR, a
    /// SpyServer, an RX-888's wide strip.
    ///
    /// There the pan bounds are that lane's own fixed window: they do not move
    /// with the passband, so the clamp reads the same numbers whichever end of
    /// the frame it is taken at and the fix above cannot reach this path. The
    /// drag just slides the view across the wide picture, the receiver is never
    /// asked to move, and it is `follow_view_into_the_band` that brings it over
    /// on the way back in. Measured on the generator with a 30 MHz lane bolted
    /// on: nought retunes, before the fix and after it.
    #[test]
    fn a_drag_inside_a_full_band_lane_never_retunes() {
        let (center, span) = (14_150_000.0, 2_000_000.0);
        let pan = movable().with_outer(Some((15_000_000.0, 30_000_000.0)), span);
        let mut view =
            ViewState { view_lo_hz: center - span / 2.0, view_hi_hz: center + span / 2.0, ..def() };
        let mut state = RadioState { center_hz: center, sample_rate: span, ..state() };
        let mut dev_center = center;
        let mut cmds = Vec::new();

        for _ in 0..6 {
            let (zoom_center, zoom_span) = pan.view_bounds(dev_center, span);
            let over = pan_view(&mut view, 25_000.0, zoom_center, zoom_span);
            pan_center(&mut dev_center, &mut state, over, pan, &mut cmds);
            let (clamp_center, clamp_span) = pan.view_bounds(dev_center, span);
            view.clamp_to(clamp_center, clamp_span);
        }

        assert!(cmds.is_empty(), "a pan inside the wide lane must not move the receiver: {cmds:?}");
        assert_eq!(dev_center, center);
        // And it really did pan — the view has walked six steps up the lane.
        assert!((view.view_lo_hz - (center - span / 2.0 + 150_000.0)).abs() < 0.5);
    }

    /// The transition frame: a zoomed-in view that runs into the edge mid-pan
    /// spends what room it had and hands over only the rest.
    #[test]
    fn a_pan_that_reaches_the_edge_hands_over_only_the_overshoot() {
        let (center, span) = (14_150_000.0, 2_000_000.0);
        // 100 kHz wide, ending 20 kHz short of the top of the window.
        let mut view = ViewState { view_lo_hz: 15_030_000.0, view_hi_hz: 15_130_000.0, ..def() };
        assert_eq!(pan_view(&mut view, 50_000.0, center, span), 30_000.0);
    }

    /// A front end whose centre is its dial is never asked for a centre: the
    /// drag is already turning that one knob, and a second command to the same
    /// place is a second CAT write per frame.
    #[test]
    fn a_dial_centred_front_end_is_left_to_its_dial() {
        let mut state = RadioState { center_hz: 14_150_000.0, ..state() };
        let mut dev_center = 14_150_000.0;
        let mut cmds = Vec::new();
        let pan = WindowPan::of(
            Some(&sdroxide_types::DeviceCaps {
                center_is_dial: true,
                freq_ranges_rx: vec![(30_000.0, 60_000_000.0)],
                ..Default::default()
            }),
            14_150_000.0,
        );
        pan_center(&mut dev_center, &mut state, 50_000.0, pan, &mut cmds);
        assert_eq!(dev_center, 14_150_000.0);
        assert!(cmds.is_empty());
        // And nothing at all is asked of a radio whose capabilities have not
        // arrived yet — the one thing that decides whether a pan may act.
        pan_center(&mut dev_center, &mut state, 50_000.0, WindowPan::of(None, 0.0), &mut cmds);
        assert!(cmds.is_empty());
    }

    /// The pan stops at the end of the range the centre is in rather than
    /// walking into the gap between two of them: an RX-888's HF and VHF ranges
    /// are not one stretch of dial, and asking for the gap is a refusal the
    /// engine would answer sixty times a second.
    #[test]
    fn a_pan_stops_at_the_end_of_its_own_receive_range() {
        let caps = sdroxide_types::DeviceCaps {
            freq_ranges_rx: vec![(0.0, 32_400_000.0), (36_000_000.0, 1_800_000_000.0)],
            ..Default::default()
        };
        let pan = WindowPan::of(Some(&caps), 32_000_000.0);
        assert_eq!(pan.limits, (0.0, 32_400_000.0));
        let mut state = RadioState { center_hz: 32_000_000.0, ..state() };
        let mut dev_center = 32_000_000.0;
        let mut cmds = Vec::new();
        pan_center(&mut dev_center, &mut state, 1_000_000.0, pan, &mut cmds);
        assert_eq!(cmds, vec![Command::SetCenter(32_400_000.0)]);
        // Already against the end: nothing more is asked for.
        cmds.clear();
        pan_center(&mut dev_center, &mut state, 1_000_000.0, pan, &mut cmds);
        assert!(cmds.is_empty(), "a pan pressed against the end must stop asking");
    }

    /// A device that publishes no ranges is taken at its word, exactly as
    /// `DeviceCaps::may_rx_hz` takes it — the SXceiver publishes none.
    #[test]
    fn a_front_end_that_publishes_no_ranges_may_still_be_panned() {
        let caps = sdroxide_types::DeviceCaps::default();
        let pan = WindowPan::of(Some(&caps), 14_150_000.0);
        assert!(pan.movable);
        let mut state = RadioState { center_hz: 14_150_000.0, ..state() };
        let mut dev_center = 14_150_000.0;
        let mut cmds = Vec::new();
        pan_center(&mut dev_center, &mut state, -50_000.0, pan, &mut cmds);
        assert_eq!(cmds, vec![Command::SetCenter(14_100_000.0)]);
    }

    fn def() -> ViewState {
        ViewState::default()
    }

    fn state() -> RadioState {
        RadioState::default()
    }

    /// An ordinary SDR: a centre of its own, and no published range in the way.
    fn movable() -> WindowPan {
        WindowPan::of(Some(&sdroxide_types::DeviceCaps::default()), 0.0)
    }

    /// A slow, careful drag must place the dial exactly where it is released —
    /// the launch threshold is what keeps precise tuning precise.
    #[test]
    fn a_careful_drag_does_not_fling() {
        // A release just under the threshold is suppressed, and the coast it
        // would have had is short enough that nothing visible is lost — which is
        // what makes the threshold safe to set where it is.
        let (t, px) = coast(FLING_MIN_LAUNCH);
        assert!(t < 0.35, "the shortest coast that can launch is {t}s — too long to feel bounded");
        assert!(px < 30.0, "suppressing it costs {px} points of travel");
    }

    /// One frame of a drag, against an engine whose state snapshot arrives
    /// `lag` frames after the `SetVfo` it answers. Returns every dial the drag
    /// asked for, in order.
    fn drag(lag: usize, frames: usize, dhz: f64, carry: bool) -> Vec<f64> {
        let mut sent: Vec<f64> = Vec::new();
        let mut carried: Option<f64> = None;
        for f in 0..frames {
            // What the engine has said by now: the command from `lag` frames
            // ago, or the dial the drag started on.
            let echoed = f.checked_sub(lag).map_or(0.0, |i| sent[i]);
            let hz = gesture_step(carried.filter(|_| carry), echoed, dhz);
            carried = Some(hz);
            sent.push(hz);
        }
        sent
    }

    /// A drag turns the dial by one delta a frame and has to go on doing that
    /// however far behind the engine's echo of it is. The view beside it is
    /// this screen's own and always does, so any shortfall here is the picture
    /// sliding out from under the marker.
    #[test]
    fn a_drag_turns_the_dial_once_a_frame_however_late_the_echo_is() {
        const FRAMES: usize = 12;
        const D: f64 = 100.0;
        for lag in 1..=3 {
            let sent = drag(lag, FRAMES, D, true);
            assert_eq!(
                *sent.last().unwrap(),
                FRAMES as f64 * D,
                "a {lag}-frame echo held the dial back"
            );
            // Evenly, too: a dial that arrived by fits and starts would twitch
            // under the pointer even where it ended up in the right place.
            for pair in sent.windows(2) {
                assert_eq!(pair[1] - pair[0], D, "the dial moved unevenly at lag {lag}");
            }
        }
    }

    /// What that replaced, so this says what it is guarding. Rebuilding the
    /// dial from the engine's snapshot every frame advances it one delta per
    /// *round trip*: at the two-frame lag of an ordinary network client the
    /// waterfall runs at twice the speed of its own marker, and the marker
    /// splits into two interleaved positions — a drift and a twitch.
    #[test]
    fn a_dial_rebuilt_from_a_late_echo_runs_slow_and_doubles() {
        const FRAMES: usize = 12;
        const D: f64 = 100.0;
        let sent = drag(2, FRAMES, D, false);
        assert_eq!(*sent.last().unwrap(), FRAMES as f64 * D / 2.0, "not half speed after all");
        // Two positions: the frames pair up, each pair asking for the same dial.
        for pair in sent.chunks(2) {
            assert_eq!(pair[0], pair[1]);
        }
    }

    /// A front end whose full-band lane is wider than its I/Q may be zoomed out
    /// into it — a KiwiSDR sends 12 kHz of I/Q and a picture of the whole
    /// 0-30 MHz, and before this the panadapter could only ever show the 12 kHz.
    #[test]
    fn a_wide_lane_lets_the_view_zoom_out_past_the_iq() {
        const DEV_C: f64 = 10_000_000.0;
        const DEV_SPAN: f64 = 12_000.0;
        let wide = (15_000_000.0, 30_000_000.0);

        // Without one, the passband is the end of the zoom, as it always was.
        let plain = WindowPan::default();
        assert_eq!(plain.view_bounds(DEV_C, DEV_SPAN), (DEV_C, DEV_SPAN));
        let mut view =
            ViewState { view_lo_hz: 0.0, view_hi_hz: 30_000_000.0, ..Default::default() };
        let (c, span) = plain.view_bounds(DEV_C, DEV_SPAN);
        view.clamp_to(c, span);
        assert!(view.span() <= DEV_SPAN, "clamped back to the passband");

        // With one, the whole band is reachable.
        let wide_pan = WindowPan::default().with_outer(Some(wide), DEV_SPAN);
        assert_eq!(wide_pan.view_bounds(DEV_C, DEV_SPAN), wide);
        let mut view =
            ViewState { view_lo_hz: 0.0, view_hi_hz: 30_000_000.0, ..Default::default() };
        let (c, span) = wide_pan.view_bounds(DEV_C, DEV_SPAN);
        view.clamp_to(c, span);
        assert_eq!((view.view_lo_hz, view.view_hi_hz), (0.0, 30_000_000.0));
    }

    /// Zoom out on a KiwiSDR, then zoom back in somewhere else: the view is
    /// narrow again but the 12 kHz the receiver streams is elsewhere, so the
    /// panadapter would go on drawing from the coarse full-band bins for ever.
    /// The receiver follows instead.
    #[test]
    fn zooming_back_in_off_the_passband_brings_the_receiver_over() {
        const DEV_C: f64 = 10_000_000.0;
        const DEV_SPAN: f64 = 12_000.0;
        let pan = WindowPan::default().with_outer(Some((15e6, 30e6)), DEV_SPAN);
        let state = RadioState { vfo_a_hz: DEV_C, active_vfo: Vfo::A, ..RadioState::default() };

        // Zoomed back in at 15 MHz, five megahertz from what is being received.
        let view =
            ViewState { view_lo_hz: 14_997_000.0, view_hi_hz: 15_003_000.0, ..Default::default() };
        let mut cmds = Vec::new();
        follow_view_into_the_band(&view, &state, DEV_C, DEV_SPAN, pan, &mut cmds);
        assert_eq!(cmds, vec![Command::SetVfo { vfo: Vfo::A, hz: 15_000_000.0 }]);
    }

    /// Still zoomed out: panning about the band view must not retune anything.
    #[test]
    fn panning_while_zoomed_out_never_retunes() {
        const DEV_C: f64 = 10_000_000.0;
        const DEV_SPAN: f64 = 12_000.0;
        let pan = WindowPan::default().with_outer(Some((15e6, 30e6)), DEV_SPAN);
        let state = RadioState { vfo_a_hz: DEV_C, active_vfo: Vfo::A, ..RadioState::default() };
        let view = ViewState { view_lo_hz: 12e6, view_hi_hz: 18e6, ..Default::default() };
        let mut cmds = Vec::new();
        follow_view_into_the_band(&view, &state, DEV_C, DEV_SPAN, pan, &mut cmds);
        assert!(cmds.is_empty(), "a view wider than the passband is not a tuning request");
    }

    /// Asked once, not once a frame: with the dial inside the view the engine
    /// is already bringing the receiver over.
    #[test]
    fn the_request_is_not_repeated_while_it_is_being_answered() {
        const DEV_SPAN: f64 = 12_000.0;
        let pan = WindowPan::default().with_outer(Some((15e6, 30e6)), DEV_SPAN);
        // The dial has moved to the view; the receiver has not caught up yet.
        let state =
            RadioState { vfo_a_hz: 15_000_000.0, active_vfo: Vfo::A, ..RadioState::default() };
        let view =
            ViewState { view_lo_hz: 14_997_000.0, view_hi_hz: 15_003_000.0, ..Default::default() };
        let mut cmds = Vec::new();
        follow_view_into_the_band(&view, &state, 10_000_000.0, DEV_SPAN, pan, &mut cmds);
        assert!(cmds.is_empty());
    }

    /// A front end with no full-band lane is untouched: its view can never
    /// leave its passband in the first place.
    #[test]
    fn a_front_end_without_a_wide_lane_is_never_retuned_by_a_zoom() {
        const DEV_SPAN: f64 = 2_000_000.0;
        let pan = WindowPan::default();
        let state = RadioState { vfo_a_hz: 10e6, active_vfo: Vfo::A, ..RadioState::default() };
        let view =
            ViewState { view_lo_hz: 14_997_000.0, view_hi_hz: 15_003_000.0, ..Default::default() };
        let mut cmds = Vec::new();
        follow_view_into_the_band(&view, &state, 10e6, DEV_SPAN, pan, &mut cmds);
        assert!(cmds.is_empty());
    }

    /// A "wide" lane no wider than the I/Q — an Icom's scope at a narrow span —
    /// is ignored, so nothing about that front end changes.
    #[test]
    fn a_wide_lane_no_wider_than_the_iq_is_ignored() {
        const DEV_C: f64 = 10_000_000.0;
        const DEV_SPAN: f64 = 2_000_000.0;
        let pan = WindowPan::default().with_outer(Some((DEV_C, 100_000.0)), DEV_SPAN);
        assert_eq!(pan.outer, None);
        assert_eq!(pan.view_bounds(DEV_C, DEV_SPAN), (DEV_C, DEV_SPAN));
    }

    /// Before the front end has said what it streams there is nothing to
    /// compare against, so the lane is not adopted on a zero span.
    #[test]
    fn no_outer_window_before_the_passband_is_known() {
        let pan = WindowPan::default().with_outer(Some((1e6, 30e6)), 0.0);
        assert_eq!(pan.outer, None);
    }
}
