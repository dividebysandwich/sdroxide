//! Settings for the transmit voice chain: the processing a microphone goes
//! through between the mic-gain control and the modulator (or, on a rig that
//! modulates its own audio, the sound card / network stream that carries it).
//!
//! ```text
//! mic → mic gain → input gain → [denoise] → Gate → EQ → De-esser → Compressor → Output
//!                                           └──── reorderable (`order`) ────┘   (limiter, trim)
//! ```
//!
//! Input gain is level setting rather than processing: it brings a particular
//! microphone up (or down) to the level the stages' thresholds are set for, so
//! it travels with a profile, stays in under BYPASS, and applies to digital
//! voice whether or not the rest of the chain does.
//!
//! Every stage is off by default and the chain as shipped is unity gain, so a
//! first-time operator transmits exactly what they did before the chain
//! existed. Voice only: digital modes synthesise their audio at the shape it
//! has to reach the air with, a TCI client sets its own audio, and both bypass
//! the chain entirely. Digital voice (FreeDV/RADE) passes through it only when
//! the operator asks for that ([`TxVoiceState::digital_voice`]).
//!
//! Lives here rather than in the DSP crate because the UI edits it and the UI
//! builds for wasm32 alone, without the engine.

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::BiquadCoeffs;

/// `v` forced into `r`, with a NaN (which `clamp` would pass straight through)
/// replaced by `fallback`. Everything in this module that comes off the wire or
/// out of `session.json` goes through here before the DSP sees it.
fn fit(v: f32, r: &RangeInclusive<f32>, fallback: f32) -> f32 {
    if v.is_nan() { fallback } else { v.clamp(*r.start(), *r.end()) }
}

/// One band of [`TxEqState`]: corner/center frequency and gain, plus either Q
/// (the mid peaking band, where higher is narrower) or shelf slope (the
/// low/high shelf bands, the RBJ cookbook's `S`, `0.1..=1.0`; `1.0` is the
/// steepest shelf that stays monotonic).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxEqBand {
    pub freq_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

/// A high- or low-pass edge on the EQ: Butterworth, 12 dB/octave, or
/// 24 dB/octave when `steep`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxEqEdge {
    pub enabled: bool,
    pub freq_hz: f32,
    pub steep: bool,
}

/// Transmit parametric EQ on the microphone audio: an optional high-pass, a low
/// shelf, two peaking bands, a high shelf and an optional low-pass, cascaded in
/// that order. Off by default and flat when turned on, so enabling it changes
/// nothing on the air until a band is actually moved.
///
/// `hpf`, `mid2` and `lpf` arrived with the voice chain; `serde(default)` lets
/// a `session.json` written before them load with both edges off and the second
/// peak flat.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxEqState {
    pub enabled: bool,
    /// Low shelf: cuts/boosts rumble and handling noise.
    pub low: TxEqBand,
    /// Mid peak: presence/proximity shaping.
    pub mid: TxEqBand,
    /// High shelf: brightness/de-ess.
    pub high: TxEqBand,
    /// High-pass ahead of everything else: handling noise, breath pops, hum.
    #[serde(default = "TxEqState::default_hpf")]
    pub hpf: TxEqEdge,
    /// A second peaking band, for the presence lift that usually sits above
    /// the cut `mid` is used for.
    #[serde(default = "TxEqState::default_mid2")]
    pub mid2: TxEqBand,
    /// Low-pass last of all, usually set to the transmit filter's width so the
    /// energy above it is not spent in the modulator only to be filtered off.
    #[serde(default = "TxEqState::default_lpf")]
    pub lpf: TxEqEdge,
}

impl Default for TxEqState {
    fn default() -> Self {
        TxEqState {
            enabled: false,
            low: TxEqBand { freq_hz: 300.0, gain_db: 0.0, q: 0.7 },
            mid: TxEqBand { freq_hz: 1500.0, gain_db: 0.0, q: 1.0 },
            high: TxEqBand { freq_hz: 2800.0, gain_db: 0.0, q: 0.7 },
            hpf: Self::default_hpf(),
            mid2: Self::default_mid2(),
            lpf: Self::default_lpf(),
        }
    }
}

impl TxEqState {
    /// UI range for every band's gain.
    pub const GAIN_DB_RANGE: RangeInclusive<f32> = -15.0..=15.0;
    pub const LOW_FREQ_HZ_RANGE: RangeInclusive<f32> = 100.0..=1000.0;
    pub const MID_FREQ_HZ_RANGE: RangeInclusive<f32> = 300.0..=3000.0;
    pub const MID2_FREQ_HZ_RANGE: RangeInclusive<f32> = 500.0..=5000.0;
    pub const HIGH_FREQ_HZ_RANGE: RangeInclusive<f32> = 1000.0..=4000.0;
    pub const HPF_HZ_RANGE: RangeInclusive<f32> = 50.0..=500.0;
    pub const LPF_HZ_RANGE: RangeInclusive<f32> = 2000.0..=8000.0;
    /// Q range for the peaking bands.
    pub const MID_Q_RANGE: RangeInclusive<f32> = 0.3..=5.0;
    /// Shelf-slope range for the low/high (shelving) bands.
    pub const SHELF_SLOPE_RANGE: RangeInclusive<f32> = 0.1..=1.0;

    /// Every section of this EQ at `rate`, in the order the audio meets them:
    /// high-pass (two sections), low shelf, mid, mid2, high shelf, low-pass
    /// (two sections). Ignores `enabled`: these are what the EQ does when on.
    pub fn sections(&self, rate: f64) -> [BiquadCoeffs; 8] {
        let band = |b: &TxEqBand| (b.freq_hz as f64, b.gain_db as f64, b.q as f64);
        let (lf, lg, lq) = band(&self.low);
        let (mf, mg, mq) = band(&self.mid);
        let (m2f, m2g, m2q) = band(&self.mid2);
        let (hf, hg, hq) = band(&self.high);
        let [h0, h1] = edge_coeffs(&self.hpf, rate, BiquadCoeffs::highpass);
        let [l0, l1] = edge_coeffs(&self.lpf, rate, BiquadCoeffs::lowpass);
        [
            h0,
            h1,
            BiquadCoeffs::low_shelf(lf, lg, lq, rate),
            BiquadCoeffs::peaking(mf, mg, mq, rate),
            BiquadCoeffs::peaking(m2f, m2g, m2q, rate),
            BiquadCoeffs::high_shelf(hf, hg, hq, rate),
            l0,
            l1,
        ]
    }

    /// The EQ's magnitude response in dB at `freq_hz` — for the TX Audio
    /// window's curve, which is therefore the response the audio really gets.
    pub fn response_db(&self, rate: f64, freq_hz: f64) -> f64 {
        let mag: f64 = self.sections(rate).iter().map(|c| c.magnitude(freq_hz, rate)).product();
        20.0 * mag.max(1e-9).log10()
    }

    fn default_hpf() -> TxEqEdge {
        TxEqEdge { enabled: false, freq_hz: 100.0, steep: false }
    }

    fn default_mid2() -> TxEqBand {
        TxEqBand { freq_hz: 2500.0, gain_db: 0.0, q: 1.0 }
    }

    fn default_lpf() -> TxEqEdge {
        TxEqEdge { enabled: false, freq_hz: 3000.0, steep: false }
    }

    /// This state with every field forced into the range the UI offers.
    ///
    /// The controls cannot leave these ranges, but two other doors into this
    /// type can: a command from a WebSocket client, and the `session.json`
    /// restored at startup, which is a file an operator may have edited. Both
    /// go through here, so the coefficient maths in `sdroxide-dsp` only ever
    /// sees corner frequencies inside the voice band and a Q it was designed
    /// for.
    pub fn clamped(self) -> Self {
        let band = |b: TxEqBand, d: TxEqBand, freq: RangeInclusive<f32>, q: RangeInclusive<f32>| {
            TxEqBand {
                freq_hz: fit(b.freq_hz, &freq, d.freq_hz),
                gain_db: fit(b.gain_db, &Self::GAIN_DB_RANGE, 0.0),
                q: fit(b.q, &q, d.q),
            }
        };
        let edge = |e: TxEqEdge, d: TxEqEdge, freq: RangeInclusive<f32>| TxEqEdge {
            enabled: e.enabled,
            freq_hz: fit(e.freq_hz, &freq, d.freq_hz),
            steep: e.steep,
        };
        let d = Self::default();
        TxEqState {
            enabled: self.enabled,
            low: band(self.low, d.low, Self::LOW_FREQ_HZ_RANGE, Self::SHELF_SLOPE_RANGE),
            mid: band(self.mid, d.mid, Self::MID_FREQ_HZ_RANGE, Self::MID_Q_RANGE),
            high: band(self.high, d.high, Self::HIGH_FREQ_HZ_RANGE, Self::SHELF_SLOPE_RANGE),
            hpf: edge(self.hpf, d.hpf, Self::HPF_HZ_RANGE),
            mid2: band(self.mid2, d.mid2, Self::MID2_FREQ_HZ_RANGE, Self::MID_Q_RANGE),
            lpf: edge(self.lpf, d.lpf, Self::LPF_HZ_RANGE),
        }
    }
}

/// Butterworth Q for one second-order section, and the two that make up a
/// fourth-order Butterworth when cascaded.
const BUTTERWORTH_Q2: f64 = std::f64::consts::FRAC_1_SQRT_2;
const BUTTERWORTH_Q4: [f64; 2] = [0.541_196_100_146_197, 1.306_562_964_876_376_6];

/// The (up to two) sections of a high- or low-pass edge. A disabled edge is
/// two identities; a 12 dB/octave one leaves its second section an identity.
fn edge_coeffs(
    edge: &TxEqEdge,
    rate: f64,
    design: fn(f64, f64, f64) -> BiquadCoeffs,
) -> [BiquadCoeffs; 2] {
    if !edge.enabled {
        return [BiquadCoeffs::identity(); 2];
    }
    let f = edge.freq_hz as f64;
    if edge.steep {
        [design(f, BUTTERWORTH_Q4[0], rate), design(f, BUTTERWORTH_Q4[1], rate)]
    } else {
        [design(f, BUTTERWORTH_Q2, rate), BiquadCoeffs::identity()]
    }
}

/// The stages whose order the operator chooses. The denoiser always runs first
/// (noise removed before anything can amplify it) and the output stage always
/// last (its limiter is the ceiling), so neither is in here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TxVoiceStage {
    Gate,
    Eq,
    DeEss,
    Comp,
}

impl TxVoiceStage {
    pub const ALL: [TxVoiceStage; 4] =
        [TxVoiceStage::Gate, TxVoiceStage::Eq, TxVoiceStage::DeEss, TxVoiceStage::Comp];

    pub fn label(self) -> &'static str {
        match self {
            TxVoiceStage::Gate => "Gate",
            TxVoiceStage::Eq => "EQ",
            TxVoiceStage::DeEss => "De-esser",
            TxVoiceStage::Comp => "Compressor",
        }
    }
}

/// Downward expander / noise gate. Knocks down room noise, fan rumble and mic
/// hiss in the pauses.
///
/// Below `threshold_db` the level is pushed further down at `ratio`:1, never by
/// more than `floor_db`. `return_db` is hysteresis — once open, the gate only
/// starts to close below `threshold_db - return_db`, so a voice hovering at the
/// threshold does not make it flutter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxGateState {
    pub enabled: bool,
    pub threshold_db: f32,
    pub return_db: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
    pub floor_db: f32,
    pub ratio: f32,
    /// How far ahead the detector looks, so a gate opening on a word's first
    /// consonant opens before it rather than clipping it. Adds this much delay
    /// to the whole transmit path while the gate is on.
    pub lookahead_ms: f32,
}

impl Default for TxGateState {
    fn default() -> Self {
        TxGateState {
            enabled: false,
            threshold_db: -45.0,
            return_db: 3.0,
            hold_ms: 20.0,
            release_ms: 80.0,
            floor_db: -15.0,
            ratio: 2.0,
            lookahead_ms: 1.5,
        }
    }
}

impl TxGateState {
    pub const THRESHOLD_DB_RANGE: RangeInclusive<f32> = -80.0..=0.0;
    pub const RETURN_DB_RANGE: RangeInclusive<f32> = 0.0..=20.0;
    pub const HOLD_MS_RANGE: RangeInclusive<f32> = 0.0..=500.0;
    pub const RELEASE_MS_RANGE: RangeInclusive<f32> = 5.0..=2000.0;
    pub const FLOOR_DB_RANGE: RangeInclusive<f32> = -80.0..=0.0;
    pub const RATIO_RANGE: RangeInclusive<f32> = 1.0..=10.0;
    pub const LOOKAHEAD_MS_RANGE: RangeInclusive<f32> = 0.0..=5.0;

    /// A hard gate: steep, and closing all the way down.
    pub fn as_gate(self) -> Self {
        TxGateState { ratio: 10.0, floor_db: -40.0, ..self }
    }

    /// A gentle expander: takes the edge off the pauses without the room
    /// going dead.
    pub fn as_expander(self) -> Self {
        TxGateState { ratio: 2.0, floor_db: -15.0, ..self }
    }

    pub fn clamped(self) -> Self {
        let d = Self::default();
        TxGateState {
            enabled: self.enabled,
            threshold_db: fit(self.threshold_db, &Self::THRESHOLD_DB_RANGE, d.threshold_db),
            return_db: fit(self.return_db, &Self::RETURN_DB_RANGE, d.return_db),
            hold_ms: fit(self.hold_ms, &Self::HOLD_MS_RANGE, d.hold_ms),
            release_ms: fit(self.release_ms, &Self::RELEASE_MS_RANGE, d.release_ms),
            floor_db: fit(self.floor_db, &Self::FLOOR_DB_RANGE, d.floor_db),
            ratio: fit(self.ratio, &Self::RATIO_RANGE, d.ratio),
            lookahead_ms: fit(self.lookahead_ms, &Self::LOOKAHEAD_MS_RANGE, d.lookahead_ms),
        }
    }
}

/// Split-band de-esser: when the sibilant band around `freq_hz` crosses
/// `threshold_db`, only that band is turned down — by at most `amount_db` — so
/// the rest of the voice keeps its level and the average power does not drop
/// with it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxDeEssState {
    pub enabled: bool,
    pub freq_hz: f32,
    pub q: f32,
    pub threshold_db: f32,
    /// Most the band is ever turned down, `<= 0`.
    pub amount_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

impl Default for TxDeEssState {
    fn default() -> Self {
        TxDeEssState {
            enabled: false,
            freq_hz: 6000.0,
            q: 2.0,
            threshold_db: -30.0,
            amount_db: -6.0,
            attack_ms: 1.0,
            release_ms: 60.0,
        }
    }
}

impl TxDeEssState {
    pub const FREQ_HZ_RANGE: RangeInclusive<f32> = 2000.0..=12000.0;
    pub const Q_RANGE: RangeInclusive<f32> = 0.5..=5.0;
    pub const THRESHOLD_DB_RANGE: RangeInclusive<f32> = -60.0..=0.0;
    pub const AMOUNT_DB_RANGE: RangeInclusive<f32> = -24.0..=0.0;
    pub const ATTACK_MS_RANGE: RangeInclusive<f32> = 0.1..=30.0;
    pub const RELEASE_MS_RANGE: RangeInclusive<f32> = 10.0..=500.0;

    pub fn clamped(self) -> Self {
        let d = Self::default();
        TxDeEssState {
            enabled: self.enabled,
            freq_hz: fit(self.freq_hz, &Self::FREQ_HZ_RANGE, d.freq_hz),
            q: fit(self.q, &Self::Q_RANGE, d.q),
            threshold_db: fit(self.threshold_db, &Self::THRESHOLD_DB_RANGE, d.threshold_db),
            amount_db: fit(self.amount_db, &Self::AMOUNT_DB_RANGE, d.amount_db),
            attack_ms: fit(self.attack_ms, &Self::ATTACK_MS_RANGE, d.attack_ms),
            release_ms: fit(self.release_ms, &Self::RELEASE_MS_RANGE, d.release_ms),
        }
    }
}

/// Feed-forward compressor with a soft knee.
///
/// `drive_db` and `phase_stages` are the peak-to-average controls. Drive is
/// gain ahead of the detector, so more of the voice crosses the threshold;
/// the phase rotator is a cascade of all-pass sections that evens out the
/// lopsided peaks a human voice has, so the same peak level carries more
/// average power. Neither changes the level of a voice that never reaches the
/// threshold — that is what `makeup_db` is for.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxCompState {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_db: f32,
    pub drive_db: f32,
    /// All-pass sections in the phase rotator, `0` = off.
    pub phase_stages: u8,
}

impl Default for TxCompState {
    fn default() -> Self {
        TxCompState {
            enabled: false,
            threshold_db: -20.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 150.0,
            knee_db: 6.0,
            makeup_db: 0.0,
            drive_db: 0.0,
            phase_stages: 0,
        }
    }
}

impl TxCompState {
    pub const THRESHOLD_DB_RANGE: RangeInclusive<f32> = -60.0..=0.0;
    pub const RATIO_RANGE: RangeInclusive<f32> = 1.0..=20.0;
    pub const ATTACK_MS_RANGE: RangeInclusive<f32> = 0.1..=300.0;
    pub const RELEASE_MS_RANGE: RangeInclusive<f32> = 5.0..=2000.0;
    pub const KNEE_DB_RANGE: RangeInclusive<f32> = 0.0..=24.0;
    pub const MAKEUP_DB_RANGE: RangeInclusive<f32> = -12.0..=24.0;
    pub const DRIVE_DB_RANGE: RangeInclusive<f32> = 0.0..=18.0;
    pub const PHASE_STAGES_MAX: u8 = 6;

    /// The static curve: gain in dB, `<= 0`, for a detector level of
    /// `level_db` *after* drive. Soft knee per Giannoulis, Massberg & Reiss,
    /// "Digital Dynamic Range Compressor Design" (JAES 2012). The DSP runs
    /// exactly this, and the UI draws it.
    pub fn static_gain_db(&self, level_db: f32) -> f32 {
        let slope = 1.0 / self.ratio.max(1.0) - 1.0;
        let over = level_db - self.threshold_db;
        let knee = self.knee_db;
        if 2.0 * over < -knee {
            0.0
        } else if knee > 0.0 && 2.0 * over.abs() <= knee {
            slope * (over + knee / 2.0).powi(2) / (2.0 * knee)
        } else {
            slope * over
        }
    }

    pub fn clamped(self) -> Self {
        let d = Self::default();
        TxCompState {
            enabled: self.enabled,
            threshold_db: fit(self.threshold_db, &Self::THRESHOLD_DB_RANGE, d.threshold_db),
            ratio: fit(self.ratio, &Self::RATIO_RANGE, d.ratio),
            attack_ms: fit(self.attack_ms, &Self::ATTACK_MS_RANGE, d.attack_ms),
            release_ms: fit(self.release_ms, &Self::RELEASE_MS_RANGE, d.release_ms),
            knee_db: fit(self.knee_db, &Self::KNEE_DB_RANGE, d.knee_db),
            makeup_db: fit(self.makeup_db, &Self::MAKEUP_DB_RANGE, d.makeup_db),
            drive_db: fit(self.drive_db, &Self::DRIVE_DB_RANGE, d.drive_db),
            phase_stages: self.phase_stages.min(Self::PHASE_STAGES_MAX),
        }
    }
}

/// The last stage before the audio leaves the chain: an output trim and a
/// look-ahead brickwall limiter.
///
/// The limiter is off by default for the same reason every other stage is: a
/// chain that is switched on but untouched must not change the level an
/// operator has already set the drive for. The engine still clips at digital
/// full scale behind it; the limiter's job is to get there without the
/// splatter that hard clipping makes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxOutputState {
    pub limiter: bool,
    pub ceiling_db: f32,
    pub trim_db: f32,
}

impl Default for TxOutputState {
    fn default() -> Self {
        TxOutputState { limiter: false, ceiling_db: -1.0, trim_db: 0.0 }
    }
}

impl TxOutputState {
    pub const CEILING_DB_RANGE: RangeInclusive<f32> = -24.0..=0.0;
    pub const TRIM_DB_RANGE: RangeInclusive<f32> = -12.0..=12.0;

    pub fn clamped(self) -> Self {
        let d = Self::default();
        TxOutputState {
            limiter: self.limiter,
            ceiling_db: fit(self.ceiling_db, &Self::CEILING_DB_RANGE, d.ceiling_db),
            trim_db: fit(self.trim_db, &Self::TRIM_DB_RANGE, d.trim_db),
        }
    }
}

/// The whole transmit voice chain — what a profile stores.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TxVoiceState {
    /// Every stage switched out at once, denoiser included, without losing
    /// which ones were on — the quickest A/B of what the chain is doing.
    pub bypass: bool,
    /// RNNoise on the microphone, ahead of every other stage.
    pub denoise: bool,
    /// The order the audio meets the reorderable stages in. Always a
    /// permutation of [`TxVoiceStage::ALL`]; [`Self::clamped`] restores the
    /// default if it is not.
    pub order: [TxVoiceStage; 4],
    pub gate: TxGateState,
    pub eq: TxEqState,
    pub deess: TxDeEssState,
    pub comp: TxCompState,
    pub output: TxOutputState,
    /// Run digital voice (FreeDV, RADE) through the chain as well. Off by
    /// default: a vocoder is trained on natural speech and may well decode
    /// worse from a heavily processed voice, so this is the operator's call.
    pub digital_voice: bool,
    /// Gain in dB between the mic-gain control and the first stage. The Mic
    /// slider is a shared, MIDI-mappable control with only 6 dB of headroom;
    /// this is what the profile for a given microphone needs on top of it so
    /// the gate and compressor thresholds sit where they were set. Applied
    /// with mic gain, outside [`Self::active`] — see the module docs.
    pub input_gain_db: f32,
}

impl Default for TxVoiceState {
    fn default() -> Self {
        TxVoiceState {
            bypass: false,
            denoise: false,
            order: TxVoiceStage::ALL,
            gate: TxGateState::default(),
            eq: TxEqState::default(),
            deess: TxDeEssState::default(),
            comp: TxCompState::default(),
            output: TxOutputState::default(),
            digital_voice: false,
            input_gain_db: 0.0,
        }
    }
}

impl TxVoiceState {
    pub const INPUT_GAIN_DB_RANGE: RangeInclusive<f32> = -12.0..=24.0;

    /// [`Self::input_gain_db`] as a linear factor, for multiplying into mic
    /// gain.
    pub fn input_gain(&self) -> f32 {
        10f32.powf(self.input_gain_db / 20.0)
    }

    /// Whether any stage would touch the audio. False means the chain is a
    /// straight wire and the engine can skip it.
    pub fn active(&self) -> bool {
        !self.bypass
            && (self.denoise
                || self.gate.enabled
                || self.eq.enabled
                || self.deess.enabled
                || self.comp.enabled
                || self.output.limiter
                || self.output.trim_db != 0.0)
    }

    /// Whether `stage` is switched on.
    pub fn stage_enabled(&self, stage: TxVoiceStage) -> bool {
        match stage {
            TxVoiceStage::Gate => self.gate.enabled,
            TxVoiceStage::Eq => self.eq.enabled,
            TxVoiceStage::DeEss => self.deess.enabled,
            TxVoiceStage::Comp => self.comp.enabled,
        }
    }

    pub fn set_stage_enabled(&mut self, stage: TxVoiceStage, on: bool) {
        match stage {
            TxVoiceStage::Gate => self.gate.enabled = on,
            TxVoiceStage::Eq => self.eq.enabled = on,
            TxVoiceStage::DeEss => self.deess.enabled = on,
            TxVoiceStage::Comp => self.comp.enabled = on,
        }
    }

    /// This state with every field forced into its range — see
    /// [`TxEqState::clamped`] for why that matters.
    pub fn clamped(self) -> Self {
        let permutation = TxVoiceStage::ALL.iter().all(|s| self.order.contains(s));
        TxVoiceState {
            bypass: self.bypass,
            denoise: self.denoise,
            order: if permutation { self.order } else { TxVoiceStage::ALL },
            gate: self.gate.clamped(),
            eq: self.eq.clamped(),
            deess: self.deess.clamped(),
            comp: self.comp.clamped(),
            output: self.output.clamped(),
            digital_voice: self.digital_voice,
            input_gain_db: fit(self.input_gain_db, &Self::INPUT_GAIN_DB_RANGE, 0.0),
        }
    }
}

/// A named [`TxVoiceState`] in the operator's profile library — the whole
/// chain: every stage's settings, which are on, and their order.
///
/// A client-side preference, like the speech and alert settings: the library
/// is what this operator keeps for their own microphones and voice, and
/// loading one simply sends the engine a whole new chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxVoiceProfile {
    pub name: String,
    pub voice: TxVoiceState,
}

/// Where the TX Audio window's test recording stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TxVoiceTest {
    #[default]
    Idle,
    /// Capturing the microphone.
    Recording,
    /// Playing the phrase through the chain to the speakers.
    Playing,
    /// The live microphone through the chain to the speakers, with the
    /// meters running.
    Monitoring,
    /// The live microphone through the chain for the meters alone — no
    /// audio, so no monitoring delay to talk against and nothing to feed back.
    Metering,
}

impl TxVoiceTest {
    /// Whether the live microphone is going through the test chain.
    pub fn is_live(self) -> bool {
        matches!(self, TxVoiceTest::Monitoring | TxVoiceTest::Metering)
    }
}

/// What [`crate::Command::TxVoiceTestLive`] asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxVoiceLive {
    Off,
    /// Meters only.
    Meters,
    /// Meters, and the processed voice on the speakers.
    Audio,
}

/// The longest test phrase kept, in seconds.
pub const TX_VOICE_TEST_MAX_S: f32 = 30.0;

/// What the voice chain did over the last meter window, for the TX Audio
/// window's meters. Levels are peaks in dBFS; gain reductions are the deepest
/// reached, in dB, `<= 0`. A stage that is off reports `0.0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxVoiceMeters {
    pub input_db: f32,
    pub output_db: f32,
    pub gate_gr_db: f32,
    pub deess_gr_db: f32,
    pub comp_gr_db: f32,
    pub limiter_gr_db: f32,
}

impl Default for TxVoiceMeters {
    fn default() -> Self {
        TxVoiceMeters {
            input_db: f32::NEG_INFINITY,
            output_db: f32::NEG_INFINITY,
            gate_gr_db: 0.0,
            deess_gr_db: 0.0,
            comp_gr_db: 0.0,
            limiter_gr_db: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chain_does_nothing() {
        assert!(!TxVoiceState::default().active());
    }

    #[test]
    fn a_broken_order_is_restored() {
        let mut s = TxVoiceState { order: [TxVoiceStage::Gate; 4], ..Default::default() };
        assert_eq!(s.clamped().order, TxVoiceStage::ALL);
        s.order = [TxVoiceStage::Comp, TxVoiceStage::Eq, TxVoiceStage::Gate, TxVoiceStage::DeEss];
        assert_eq!(s.clamped().order, s.order);
    }

    #[test]
    fn nan_falls_back_to_the_default() {
        let mut s = TxVoiceState::default();
        s.comp.ratio = f32::NAN;
        s.gate.threshold_db = -500.0;
        let c = s.clamped();
        assert_eq!(c.comp.ratio, TxCompState::default().ratio);
        assert_eq!(c.gate.threshold_db, -80.0);
    }

    #[test]
    fn input_gain_is_clamped_and_kept_out_of_active() {
        let s = TxVoiceState { input_gain_db: 40.0, ..Default::default() }.clamped();
        assert_eq!(s.input_gain_db, 24.0);
        // Level setting, not processing: the chain stays a straight wire.
        assert!(!s.active());
        assert!(
            (TxVoiceState { input_gain_db: 6.0, ..Default::default() }.input_gain() - 1.995).abs()
                < 1e-3
        );
    }

    /// A chain saved before input gain existed loads at unity.
    #[test]
    fn a_chain_without_input_gain_loads_at_unity() {
        let mut v = serde_json::to_value(TxVoiceState::default()).unwrap();
        v.as_object_mut().unwrap().remove("input_gain_db");
        let s: TxVoiceState = serde_json::from_value(v).unwrap();
        assert_eq!(s.input_gain_db, 0.0);
    }

    /// A `session.json` written before the voice chain held only the three
    /// original EQ bands; it has to load, with the new edges off.
    #[test]
    fn an_old_eq_still_loads() {
        let json = r#"{"enabled":true,
            "low":{"freq_hz":200.0,"gain_db":-3.0,"q":0.7},
            "mid":{"freq_hz":1500.0,"gain_db":2.0,"q":1.0},
            "high":{"freq_hz":2800.0,"gain_db":0.0,"q":0.7}}"#;
        let eq: TxEqState = serde_json::from_str(json).unwrap();
        assert!(eq.enabled);
        assert_eq!(eq.low.gain_db, -3.0);
        assert!(!eq.hpf.enabled && !eq.lpf.enabled);
        assert_eq!(eq.mid2.gain_db, 0.0);
    }
}
