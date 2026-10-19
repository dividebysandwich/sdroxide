//! Following a slotted mode's frames by ear.
//!
//! The slot bar used to read the wall clock, shifted by the DT the stations we
//! hear show. That is the time the frame is *on the air*; what the operator
//! hears is that time plus the whole receive path — the radio, the network, the
//! sound card's buffer — and the bar drifted off the sound by however long that
//! chain was. It took three frames of listening to notice, which is why nobody
//! could say where it came from.
//!
//! This is the other half of a bar that follows the sound: a [`FrameGate`]
//! listens to the audio on its way to the speakers and reports when a frame
//! starts and when it stops, and a [`SlotFollower`] turns those edges into a
//! place on the bar. Both are pure, so the engine, the remote client and the
//! browser run the same code.
//!
//! The gate looks at **level only**, and a level can lie: QRM keeps the band
//! loud through the gap, a fade makes a hole in the middle of a frame. So the
//! follower never takes an edge on its own say-so. It asks the slot clock
//! whether an edge is at a plausible place in the slot, and when the sound
//! stops giving it edges it hands the bar back to the clock instead of
//! freezing — see [`SlotFollower::view`].

use std::collections::VecDeque;

use crate::SlotTiming;

/// An edge of a frame, as the operator hears it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeardEdge {
    /// `true` when a frame starts being heard, `false` when it stops.
    pub present: bool,
    /// Unix seconds at which the edge comes out of the speakers — the time the
    /// audio was handed over plus everything it still has to travel through.
    pub at: f64,
}

/// An edge in the audio just handed to [`FrameGate::process`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateEdge {
    /// `true` when a frame starts, `false` when it stops.
    pub present: bool,
    /// How long before the end of that audio the edge happened, in seconds.
    pub ago_s: f64,
}

/// How often the level is read, and the unit every time below is counted in.
const HOP_S: f64 = 0.05;
/// How much of the previous level each reading carries (about 100 ms).
const SMOOTHING: f32 = 0.5;
/// How much of the past defines "the level of a frame", and which part of it.
const REFERENCE_S: f64 = 8.0;
const REFERENCE_PERCENTILE: f32 = 0.85;
/// A reference needs this much listening before it is worth comparing with.
const REFERENCE_MIN_S: f64 = 2.0;
/// How far under the frame level counts as the gap between two frames.
///
/// Measured on an off-air FT8 recording: the gap between frames is only
/// 1.5–2 s long and 3–15 dB deep, with edges 0.1 s wide. At 2–3 dB the band's
/// own noise makes false gaps; 4 dB is the shallowest that did not.
const DROP_DB: f32 = 4.0;
/// How long it has to stay down to be a gap rather than a flutter.
const HOLD_S: f64 = 0.25;
/// How close to the frame level it has to come back to be a frame again. Closer
/// than the drop is deep, so a level sitting on the line does not chatter.
const RECOVER_DB: f32 = 1.5;
/// A gap that lasts this long is not a gap but a level that moved (AGC, a new
/// station on the dial): forget the reference and learn it again.
const MAX_GAP_S: f64 = 8.0;
/// The speech band the level is measured in: what a frame's tones fill, and
/// not what a hum, a carrier or an ultrasonic whine put outside it.
const BAND_LOW_HZ: f64 = 300.0;
const BAND_HIGH_HZ: f64 = 2_800.0;

/// One second-order section, direct form I.
#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    x: [f64; 2],
    y: [f64; 2],
}

impl Biquad {
    /// Butterworth high- or low-pass (RBJ cookbook, Q = 1/√2).
    fn new(low_pass: bool, hz: f64, rate: f64) -> Self {
        let w = std::f64::consts::TAU * hz / rate;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let (b0, b1) = if low_pass {
            ((1.0 - cos) / 2.0, 1.0 - cos)
        } else {
            ((1.0 + cos) / 2.0, -(1.0 + cos))
        };
        Biquad {
            b: [b0 / a0, b1 / a0, b0 / a0],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            ..Default::default()
        }
    }

    fn step(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// Listens to audio on its way to the speakers and reports where the frames
/// of a slotted mode start and stop.
///
/// What it compares is the level in 300–2800 Hz against the level the last
/// eight seconds of frame had: a drop of [`DROP_DB`] that lasts [`HOLD_S`] is
/// the end of a frame, and a return to within [`RECOVER_DB`] of the old level
/// is the start of the next. It has no idea what a frame *is* — a band of
/// nothing but noise gives it no edges, and QRM that fills the gap hides it —
/// which is why [`SlotFollower`] sits between it and the bar.
#[derive(Debug, Clone)]
pub struct FrameGate {
    rate: f64,
    high_pass: Biquad,
    low_pass: Option<Biquad>,
    hop_len: usize,
    in_hop: usize,
    energy: f64,
    /// Samples taken since the gate started or was reset: the clock the edges
    /// are placed on.
    samples: u64,
    hops: u64,
    smoothed: Option<f32>,
    /// The smoothed level of the hops that were not part of a gap.
    window: VecDeque<f32>,
    /// The hop the level first went under the line, while it is still deciding
    /// whether this is a gap.
    candidate: Option<u64>,
    /// The hop a confirmed gap started at, while one is open.
    gap_since: Option<u64>,
}

impl FrameGate {
    /// A gate for audio at `rate` Hz, mono.
    pub fn new(rate: f64) -> Self {
        let rate = rate.max(8_000.0);
        FrameGate {
            rate,
            high_pass: Biquad::new(false, BAND_LOW_HZ, rate),
            // A low rate has no room above the band to filter into.
            low_pass: (BAND_HIGH_HZ < rate * 0.45).then(|| Biquad::new(true, BAND_HIGH_HZ, rate)),
            hop_len: ((rate * HOP_S) as usize).max(1),
            in_hop: 0,
            energy: 0.0,
            samples: 0,
            hops: 0,
            smoothed: None,
            window: VecDeque::new(),
            candidate: None,
            gap_since: None,
        }
    }

    /// The rate this gate was built for.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Forget everything heard so far. For a turn of our own transmission, a
    /// change of dial or mode: the level that follows is not the level before,
    /// and a reference learnt on the old one would call the new one a gap.
    pub fn reset(&mut self) {
        *self = FrameGate::new(self.rate);
    }

    /// Take the next block of audio, and append what it contained to `out`.
    pub fn process(&mut self, block: &[f32], out: &mut Vec<GateEdge>) {
        let mut found: Vec<(bool, u64)> = Vec::new();
        for &s in block {
            let mut y = self.high_pass.step(f64::from(s));
            if let Some(lp) = self.low_pass.as_mut() {
                y = lp.step(y);
            }
            self.energy += y * y;
            self.in_hop += 1;
            self.samples += 1;
            if self.in_hop == self.hop_len {
                self.hop_done(&mut found);
            }
        }
        for (present, at_sample) in found {
            let ago = self.samples.saturating_sub(at_sample) as f64 / self.rate;
            out.push(GateEdge { present, ago_s: ago });
        }
    }

    fn reference(&self, fallback: f32) -> f32 {
        if (self.window.len() as f64) * HOP_S < REFERENCE_MIN_S {
            return fallback;
        }
        let mut v: Vec<f32> = self.window.iter().copied().collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v[((v.len() - 1) as f32 * REFERENCE_PERCENTILE) as usize]
    }

    fn hop_done(&mut self, found: &mut Vec<(bool, u64)>) {
        let level = (10.0 * (self.energy / self.hop_len as f64 + 1e-12).log10()) as f32;
        self.energy = 0.0;
        self.in_hop = 0;
        let smoothed = match self.smoothed {
            None => level,
            Some(s) => s + SMOOTHING * (level - s),
        };
        self.smoothed = Some(smoothed);
        let hop = self.hops;
        self.hops += 1;
        let hop_start = |h: u64| h * self.hop_len as u64;
        let reference = self.reference(smoothed);

        match self.gap_since {
            None => {
                self.window.push_back(smoothed);
                while (self.window.len() as f64) * HOP_S > REFERENCE_S {
                    self.window.pop_front();
                }
                if smoothed < reference - DROP_DB {
                    let first = *self.candidate.get_or_insert(hop);
                    if (hop - first) as f64 * HOP_S >= HOLD_S {
                        self.gap_since = Some(hop);
                        // The edge is where it went under the line, not where
                        // it was confirmed: the hold is a quarter second the
                        // bar must not be late by.
                        found.push((false, hop_start(first)));
                    }
                } else {
                    self.candidate = None;
                }
            }
            Some(since) => {
                if smoothed > reference - RECOVER_DB {
                    self.gap_since = None;
                    self.candidate = None;
                    found.push((true, hop_start(hop)));
                } else if (hop - since) as f64 * HOP_S > MAX_GAP_S {
                    self.gap_since = None;
                    self.candidate = None;
                    self.window.clear();
                }
            }
        }
    }
}

/// What the follower makes of the sound: where on the bar to be, and which
/// stretch of it that is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Followed {
    /// Where the bar has got to, as a fraction of its width.
    pub frac: f32,
    /// Where the bar changes from the stretch a frame fills to the one that
    /// follows it, as a fraction of its width. The burst's share of the slot.
    pub split: f32,
    /// Past the split: the frame is over and the next has not arrived yet.
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Heard {
    /// Nothing accepted yet.
    #[default]
    Nothing,
    /// A frame has been sounding since this instant.
    Signal(f64),
    /// The frame ended at this instant and nothing has taken its place.
    Quiet(f64),
}

/// Turns the edges a [`FrameGate`] found into a place on the slot bar, and
/// decides how far to trust them.
///
/// An edge counts only where the slot clock says an edge could be: a start
/// near the start of the slot, an end near the end of the burst. That is the
/// whole defence against QRM and fades — a hole in the middle of a frame is a
/// perfectly good drop in level and an impossible end of frame, and the clock
/// is what knows the difference. The clock is only asked *whether*, never
/// *when*: a station running a second early or late, or a path a second long,
/// is inside the window and the bar goes where the sound is.
#[derive(Debug, Clone, Default)]
pub struct SlotFollower {
    heard: Heard,
    /// The slot length the edges were taken against. A different mode — FT8
    /// to FT4 — is a different slot, and what was heard in one says nothing
    /// about the other.
    slot_s: f64,
}

impl SlotFollower {
    /// Forget what was heard: an over of our own, a dial or mode change. The
    /// bar goes back to the clock until the sound has given it something new.
    pub fn reset(&mut self) {
        self.heard = Heard::Nothing;
    }

    /// How far either side of its expected place an edge may fall and still be
    /// taken, in seconds.
    fn tolerance(t: &SlotTiming) -> f64 {
        (0.2 * t.slot_s).min(3.0)
    }

    /// How far past the end of a burst the bar waits for the sound before it
    /// stops believing a frame is still on.
    fn grace(t: &SlotTiming) -> f64 {
        (0.1 * t.slot_s).max(1.0)
    }

    /// The signed distance from `expected` to `at` in the slot, the short way
    /// round, in seconds.
    fn offset_in_slot(t: &SlotTiming, at: f64, expected: f64) -> f64 {
        let d = (at - expected).rem_euclid(t.slot_s);
        if d > t.slot_s / 2.0 { d - t.slot_s } else { d }
    }

    /// Offer an edge the gate found.
    pub fn edge(&mut self, t: &SlotTiming, edge: HeardEdge) {
        if self.slot_s != t.slot_s {
            self.heard = Heard::Nothing;
            self.slot_s = t.slot_s;
        }
        if edge.present {
            let off = Self::offset_in_slot(t, edge.at, t.tx_offset_s);
            if off.abs() > Self::tolerance(t) {
                return;
            }
            // The same frame twice: a start seen again before there was room
            // for a whole new slot.
            if let Heard::Signal(since) = self.heard {
                if edge.at - since < t.slot_s * 0.5 {
                    return;
                }
            }
            self.heard = Heard::Signal(edge.at);
        } else {
            let off = Self::offset_in_slot(t, edge.at, t.tx_offset_s + t.burst_s);
            // Not far before the burst's end — that is a fade — and not
            // unboundedly after it either.
            if off < -0.1 * t.slot_s || off > Self::tolerance(t) {
                return;
            }
            if let Heard::Quiet(since) = self.heard {
                if edge.at - since < t.slot_s * 0.5 {
                    return;
                }
            }
            self.heard = Heard::Quiet(edge.at);
        }
    }

    /// Where the bar is at `now`, or `None` when it should be the clock's.
    ///
    /// `None` is not an error: it is every band that has not given the gate
    /// anything to find, and every stretch where what was found has stopped
    /// being believable — a frame that has sounded longer than a burst can, a
    /// quiet that has outlasted the gap — and the bar must carry on rather
    /// than stand still on a frame that ended a minute ago.
    pub fn view(&self, t: &SlotTiming, now: f64) -> Option<Followed> {
        let split = (t.burst_s / t.slot_s) as f32;
        if self.slot_s != t.slot_s {
            return None;
        }
        match self.heard {
            Heard::Nothing => None,
            Heard::Signal(since) => {
                let on = now - since;
                if on > t.burst_s + Self::grace(t) {
                    return None;
                }
                let p = (on / t.burst_s).clamp(0.0, 1.0) as f32;
                Some(Followed { frac: split * p, split, quiet: false })
            }
            Heard::Quiet(since) => {
                let gap = (t.slot_s - t.burst_s).max(0.1);
                let off = now - since;
                if off > gap + Self::tolerance(t) {
                    return None;
                }
                let p = (off / gap).clamp(0.0, 1.0) as f32;
                Some(Followed { frac: split + (1.0 - split) * p, split, quiet: true })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;
    const FT8: SlotTiming = SlotTiming { slot_s: 15.0, tx_offset_s: 0.5, burst_s: 12.64 };

    /// A deterministic noise source: the tests need the same band every run.
    struct Noise(u32);
    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
        }
    }

    /// A stretch of band: noise at `noise`, plus tones at `tones` (all in the
    /// speech band), for `seconds`. Levels are linear amplitudes.
    fn stretch(seconds: f64, noise: f32, tones: f32, rng: &mut Noise, phase: &mut f64) -> Vec<f32> {
        (0..(seconds * RATE) as usize)
            .map(|_| {
                *phase += 1.0;
                let t = *phase / RATE;
                let tone = [900.0, 1_100.0, 1_300.0, 1_500.0]
                    .iter()
                    .map(|f| (std::f64::consts::TAU * f * t).sin() as f32)
                    .sum::<f32>();
                noise * rng.next() + tones * tone * 0.25
            })
            .collect()
    }

    /// Feed `audio` in 20 ms blocks the way the engine does, and return every
    /// edge placed on the stream's own timeline, in seconds.
    fn edges_of(gate: &mut FrameGate, audio: &[f32]) -> Vec<(bool, f64)> {
        let mut out = Vec::new();
        let block = (RATE * 0.02) as usize;
        for (n, chunk) in audio.chunks(block).enumerate() {
            let mut found = Vec::new();
            gate.process(chunk, &mut found);
            let end = ((n * block + chunk.len()) as f64) / RATE;
            out.extend(found.into_iter().map(|e| (e.present, end - e.ago_s)));
        }
        out
    }

    /// Four FT8-shaped frames: 12.6 s of tones over band noise, then a 2.4 s
    /// gap that leaves only the noise.
    fn four_frames() -> Vec<f32> {
        let mut rng = Noise(7);
        let mut ph = 0.0;
        let mut a = stretch(2.4, 0.02, 0.0, &mut rng, &mut ph);
        for _ in 0..4 {
            a.extend(stretch(12.6, 0.02, 0.3, &mut rng, &mut ph));
            a.extend(stretch(2.4, 0.02, 0.0, &mut rng, &mut ph));
        }
        a
    }

    #[test]
    fn the_end_and_start_of_every_frame_are_found_where_the_sound_changes() {
        let mut gate = FrameGate::new(RATE);
        let got = edges_of(&mut gate, &four_frames());
        let stops: Vec<f64> = got.iter().filter(|e| !e.0).map(|e| e.1).collect();
        let starts: Vec<f64> = got.iter().filter(|e| e.0).map(|e| e.1).collect();
        // Frames end at 15, 30, 45 and 60 s and start at 2.4, 17.4, 32.4, 47.4.
        // The first stop is the end of the lead-in silence's frame — there was
        // none — so three gaps are heard between four frames, plus the last.
        let want_stops = [15.0, 30.0, 45.0, 60.0];
        assert_eq!(stops.len(), want_stops.len(), "stops {stops:?} edges {got:?}");
        for (g, w) in stops.iter().zip(want_stops) {
            assert!((g - w).abs() < 0.25, "a frame ended at {w} s and was heard to end at {g} s");
        }
        for (g, w) in starts.iter().zip([17.4, 32.4, 47.4]) {
            assert!((g - w).abs() < 0.25, "a frame began at {w} s and was heard to begin at {g} s");
        }
    }

    #[test]
    fn the_edge_is_where_the_level_fell_not_where_it_was_confirmed() {
        // The gap is confirmed a quarter second after the level falls. A gate
        // that stamped the confirmation would turn the bar green late by that
        // much, which is the very thing this exists to remove.
        let mut gate = FrameGate::new(RATE);
        let got = edges_of(&mut gate, &four_frames());
        let first_stop = got.iter().find(|e| !e.0).expect("no stop").1;
        assert!(first_stop < 15.15, "stamped {first_stop} s for a fall at 15.0 s");
    }

    #[test]
    fn a_band_of_nothing_but_noise_gives_no_edges() {
        let mut rng = Noise(3);
        let mut ph = 0.0;
        let audio = stretch(60.0, 0.05, 0.0, &mut rng, &mut ph);
        let got = edges_of(&mut FrameGate::new(RATE), &audio);
        assert!(got.is_empty(), "noise alone produced {got:?}");
    }

    #[test]
    fn a_shallow_fade_is_not_the_end_of_a_frame() {
        // 3 dB for 0.7 s inside a frame — the fade that fooled a 3 dB gate on
        // the real recording.
        let mut rng = Noise(11);
        let mut ph = 0.0;
        let mut a = stretch(4.0, 0.02, 0.3, &mut rng, &mut ph);
        a.extend(stretch(0.7, 0.02, 0.3 * 0.70, &mut rng, &mut ph));
        a.extend(stretch(8.0, 0.02, 0.3, &mut rng, &mut ph));
        let got = edges_of(&mut FrameGate::new(RATE), &a);
        assert!(got.is_empty(), "a 3 dB fade was taken for an edge: {got:?}");
    }

    #[test]
    fn a_level_that_moved_for_good_is_learnt_again() {
        // A frame, then the band settles 12 dB lower and stays there: the gate
        // calls it a gap, and must not stay in that gap forever.
        let mut rng = Noise(5);
        let mut ph = 0.0;
        let mut a = stretch(8.0, 0.02, 0.3, &mut rng, &mut ph);
        a.extend(stretch(14.0, 0.02, 0.07, &mut rng, &mut ph));
        a.extend(stretch(4.0, 0.02, 0.07, &mut rng, &mut ph));
        let mut gate = FrameGate::new(RATE);
        let got = edges_of(&mut gate, &a);
        assert_eq!(got.iter().filter(|e| !e.0).count(), 1, "{got:?}");
        // After the reference is rebuilt on the new level, a later gap in it is
        // found again.
        let mut b = stretch(3.0, 0.02, 0.0, &mut rng, &mut ph);
        b.extend(stretch(6.0, 0.02, 0.07, &mut rng, &mut ph));
        let after = edges_of(&mut gate, &b);
        assert!(after.iter().any(|e| e.0), "the gate never came back from its gap: {after:?}");
    }

    #[test]
    fn a_reset_forgets_the_level_it_had_learnt() {
        let mut gate = FrameGate::new(RATE);
        let _ = edges_of(&mut gate, &four_frames());
        gate.reset();
        assert!(gate.window.is_empty() && gate.gap_since.is_none() && gate.samples == 0);
    }

    // ── the follower ───────────────────────────────────────────────

    /// A slot boundary, so slot arithmetic in the tests is in whole seconds.
    const T0: f64 = 1_000_000_005.0;

    fn edge(present: bool, at: f64) -> HeardEdge {
        HeardEdge { present, at }
    }

    #[test]
    fn the_bar_starts_when_the_frame_is_heard_not_when_the_clock_says() {
        // The clock is 0.1 s into the slot, so a clock-driven bar would be all
        // but empty. The frame was heard to begin 1.5 s ago: the bar is there.
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5 - 0.6));
        let v = f.view(&FT8, T0 + 0.9).unwrap();
        assert!(!v.quiet);
        assert!((v.frac - v.split * (1.0 / 12.64) as f32).abs() < 0.01, "{v:?}");
    }

    #[test]
    fn it_turns_to_the_quiet_stretch_the_moment_the_sound_stops() {
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        f.edge(&FT8, edge(false, T0 + 12.9));
        // Half a second before the clock thinks the burst is over, and long
        // before a clock-driven bar would have left the white stretch.
        let v = f.view(&FT8, T0 + 13.0).unwrap();
        assert!(v.quiet, "{v:?}");
        assert!(v.frac > v.split);
    }

    #[test]
    fn the_next_frame_starts_the_bar_again_at_once() {
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        f.edge(&FT8, edge(false, T0 + 12.9));
        f.edge(&FT8, edge(true, T0 + 14.85));
        let v = f.view(&FT8, T0 + 14.9).unwrap();
        assert!(!v.quiet);
        assert!(v.frac < 0.01, "the bar did not start over: {v:?}");
    }

    #[test]
    fn a_fade_in_the_middle_of_a_frame_does_not_end_it() {
        // QSB or QRM makes a hole 6 s into a frame. The level really did fall,
        // and the gate will say so; the clock knows no frame ends there.
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        f.edge(&FT8, edge(false, T0 + 6.5));
        f.edge(&FT8, edge(true, T0 + 7.2));
        let v = f.view(&FT8, T0 + 8.0).unwrap();
        assert!(!v.quiet, "a mid-frame fade ended the frame: {v:?}");
        assert!((v.frac - v.split * (7.5 / 12.64) as f32).abs() < 0.01, "{v:?}");
    }

    #[test]
    fn a_start_far_from_the_start_of_a_slot_is_not_a_frame() {
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 7.0));
        assert_eq!(f.view(&FT8, T0 + 8.0), None);
    }

    #[test]
    fn a_frame_that_never_ends_hands_the_bar_back_to_the_clock() {
        // QRM keeps the band up through the gap, so no end of frame comes. The
        // bar must not stand at full for the rest of the evening.
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        assert!(f.view(&FT8, T0 + 12.0).is_some());
        assert_eq!(f.view(&FT8, T0 + 12.64 + 1.6 + 0.5), None);
    }

    #[test]
    fn a_quiet_that_outlasts_the_gap_hands_the_bar_back_to_the_clock() {
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(false, T0 + 13.0));
        assert!(f.view(&FT8, T0 + 14.0).is_some());
        assert_eq!(f.view(&FT8, T0 + 13.0 + 2.36 + 3.0 + 0.1), None);
    }

    #[test]
    fn nothing_heard_is_the_clocks_bar() {
        assert_eq!(SlotFollower::default().view(&FT8, T0 + 5.0), None);
    }

    #[test]
    fn a_reset_goes_back_to_the_clock() {
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        f.reset();
        assert_eq!(f.view(&FT8, T0 + 2.0), None);
    }

    #[test]
    fn the_sound_ahead_of_the_clock_by_the_playout_does_not_matter() {
        // The edge is stamped when it comes out of the speakers, which is later
        // than the engine saw it. A bar asked a moment before that sits at the
        // start rather than before it.
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5 + 0.3));
        let v = f.view(&FT8, T0 + 0.5).unwrap();
        assert_eq!(v.frac, 0.0);
    }

    #[test]
    fn what_was_heard_in_one_mode_says_nothing_about_another() {
        let ft4 = SlotTiming { slot_s: 7.5, tx_offset_s: 0.5, burst_s: 4.48 };
        let mut f = SlotFollower::default();
        f.edge(&FT8, edge(true, T0 + 0.5));
        assert!(f.view(&FT8, T0 + 2.0).is_some());
        assert_eq!(f.view(&ft4, T0 + 2.0), None);
        f.edge(&ft4, edge(true, T0 + 7.5 + 0.5));
        assert_eq!(f.view(&FT8, T0 + 9.0), None, "FT4's frame was taken for FT8's");
    }

    #[test]
    fn ft4_and_ft2_follow_on_their_own_proportions() {
        let ft4 = SlotTiming { slot_s: 7.5, tx_offset_s: 0.5, burst_s: 4.48 };
        let ft2 = SlotTiming { slot_s: 3.75, tx_offset_s: 0.1, burst_s: 2.52 };
        for t in [ft4, ft2] {
            let mut f = SlotFollower::default();
            f.edge(&t, edge(true, T0 + t.tx_offset_s));
            f.edge(&t, edge(false, T0 + t.tx_offset_s + t.burst_s));
            let v = f.view(&t, T0 + t.tx_offset_s + t.burst_s + 0.2).unwrap();
            assert!(v.quiet, "{t:?}: {v:?}");
            assert!((v.split - (t.burst_s / t.slot_s) as f32).abs() < 1e-6);
        }
    }

    /// Off-air check, run by hand like the decoders' own: point
    /// `SDROXIDE_FRAMEGATE_SAMPLE` at a mono or stereo 16-bit WAV of FT8 and it
    /// must find a stop and a start for every gap, at the times in
    /// `SDROXIDE_FRAMEGATE_STOPS` / `_STARTS` (comma-separated seconds).
    #[test]
    #[ignore = "needs an off-air recording"]
    fn an_off_air_recording_is_followed() {
        let Ok(path) = std::env::var("SDROXIDE_FRAMEGATE_SAMPLE") else { return };
        let bytes = std::fs::read(path).expect("read the recording");
        // A RIFF walk, enough for PCM16: find `fmt ` and `data`.
        let (mut channels, mut rate, mut data) = (1usize, RATE as u32, &bytes[..0]);
        let mut i = 12;
        while i + 8 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
            let body = &bytes[i + 8..(i + 8 + size).min(bytes.len())];
            match &bytes[i..i + 4] {
                b"fmt " => {
                    channels = u16::from_le_bytes([body[2], body[3]]) as usize;
                    rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                }
                b"data" => data = body,
                _ => {}
            }
            i += 8 + size + (size & 1);
        }
        let mono: Vec<f32> = data
            .chunks_exact(2 * channels)
            .map(|f| {
                (0..channels)
                    .map(|c| i16::from_le_bytes([f[2 * c], f[2 * c + 1]]) as f32 / 32768.0)
                    .sum::<f32>()
                    / channels as f32
            })
            .collect();
        let list = |k: &str| -> Vec<f64> {
            std::env::var(k)
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect()
        };
        let mut gate = FrameGate::new(f64::from(rate));
        let mut got = Vec::new();
        let block = (f64::from(rate) * 0.02) as usize;
        for (n, chunk) in mono.chunks(block).enumerate() {
            let mut found = Vec::new();
            gate.process(chunk, &mut found);
            let end = (n * block + chunk.len()) as f64 / f64::from(rate);
            got.extend(found.into_iter().map(|e| (e.present, end - e.ago_s)));
        }
        eprintln!("{got:?}");
        // Every edge the follower is offered from this recording lands inside
        // its windows: QRM-free air must not be second-guessed.
        let mut f = SlotFollower::default();
        for &(present, at) in &got {
            f.edge(&FT8, edge(present, T0 + at));
        }
        let last = got.last().expect("no edges").1;
        let v = f.view(&FT8, T0 + last + 0.2).expect("the follower lost the recording");
        assert!(!v.quiet, "the last frame had started: {v:?}");
        for (present, key) in
            [(false, "SDROXIDE_FRAMEGATE_STOPS"), (true, "SDROXIDE_FRAMEGATE_STARTS")]
        {
            let heard: Vec<f64> = got.iter().filter(|e| e.0 == present).map(|e| e.1).collect();
            let want = list(key);
            assert_eq!(heard.len(), want.len(), "{key}: heard {heard:?}, expected {want:?}");
            for (h, w) in heard.iter().zip(&want) {
                assert!((h - w).abs() < 0.3, "{key}: heard {h}, expected {w}");
            }
        }
    }
}
