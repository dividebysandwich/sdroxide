//! Control-input vocabulary: what an external control *does*, independent of
//! what produced it.
//!
//! A keyboard chord, a mouse wheel detent and a MIDI encoder tick all resolve
//! to the same [`Action`] plus an [`ActionInput`] sample; the client turns that
//! into [`Command`](crate::Command)s (or a purely local view change). Adding a
//! new transport — USB HID, a gamepad, a home-built serial box — means mapping
//! its events onto these two types and nothing else.
//!
//! Everything here is plain data with no I/O, so the browser client compiles
//! and edits the same bindings the native app runs.

use serde::{Deserialize, Serialize};

use crate::{Band, Mode, Vfo};

/// What a control input does.
///
/// The first block is [`ActionKind::Continuous`] — driven by a signed delta
/// (encoder, wheel, arrow key) or an absolute 0..=1 position (fader). The rest
/// are [`ActionKind::Momentary`] — driven by press/release, and interpreted as
/// hold-to-act or flip-on-press according to [`ButtonMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    // ── Continuous ──────────────────────────────────────────────────────────
    Tune,
    RitOffset,
    XitOffset,
    Volume,
    SubVolume,
    Squelch,
    FilterWidth,
    FilterShift,
    AgcMaxGain,
    /// Fixed audio gain, in effect while the AGC is off.
    ManualGain,
    TxDrive,
    TuneDrive,
    MicGain,
    /// Digital-mode transmit tone offset within the passband (Hz).
    DigiAudioFreq,
    SpectrumZoom,
    SpectrumPan,
    SpectrumFloorDb,
    SpectrumCeilDb,

    // ── Momentary / toggle ──────────────────────────────────────────────────
    Ptt,
    TuneCarrier,
    Mute,
    NoiseBlanker,
    /// Advance the noise-reduction strength: Off → Low → Med → High → Off, on
    /// whichever engine is selected. The engine itself is picked from the UI.
    NoiseReductionCycle,
    AutoNotch,
    /// Binaural (pseudo-stereo) audio — see [`crate::Mode::binaural_audio`].
    Binaural,
    AgcCycle,
    SubRx,
    Split,
    RitEnable,
    XitEnable,
    RitClear,
    XitClear,
    VfoSelect(Vfo),
    VfoToggle,
    SwapVfos,
    CopyAtoB,
    BandUp,
    BandDown,
    BandSelect(Band),
    ModeNext,
    ModePrev,
    ModeSelect(Mode),
    MemoryRecall(u32),
    RecordToggle,
    AbortTx,
    /// Hold the bound key as a CW straight key (issue #322). The *key* is the
    /// binding; the CW panel reads it held rather than the input runtime
    /// dispatching it, because a straight key is a held state and not a
    /// press/release pair the runtime can see through the panel's own focus
    /// rules. Only takes effect while the CW panel's KEY toggle is armed.
    CwStraight,
    /// Transmit voice-keyer slot `n` (0-based). Does nothing when the slot is
    /// empty, which is what makes the shipped numpad bindings safe.
    VoicePlay(u8),
    /// Stop a voice-keyer message mid-transmission.
    VoiceStop,
    /// Send the 1750 Hz repeater tone burst. From receive it keys the
    /// transmitter for the length of the burst and lets go again, which is
    /// what makes it worth a button of its own.
    ToneBurst,

    // ── Client-local only: never becomes a Command ──────────────────────────
    FitSpan,
    ZoomIn,
    ZoomOut,
    PeakHold,
    SpectrumCollapse,
    /// Hide/show the waterfall — the twin of [`Action::SpectrumCollapse`],
    /// both of them switches on the SPEC popup's pair of layers.
    WaterfallCollapse,
    WaterfallFlip,
    ToggleHelp,
    ToggleSettings,
    ToggleLogbook,
    ToggleSpots,
    ToggleMemories,
    ToggleVoice,

    // ── Spoken announcements; also client-local only ─────────────────────────
    /// Read the whole radio out: band, frequency, mode, VFO, split, and SWR
    /// while keyed.
    SpeakStatus,
    /// Say the last announcement again — for the word that was masked by a
    /// burst of noise just as it arrived.
    SpeakRepeat,
    /// Stop talking now and drop whatever is queued.
    SpeechSilence,
    /// Announcements on/off, with a spoken confirmation either way.
    SpeechToggle,
}

/// Whether an action is driven by a value or by a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Continuous,
    Momentary,
}

impl Action {
    pub fn kind(self) -> ActionKind {
        use Action::*;
        match self {
            Tune | RitOffset | XitOffset | Volume | SubVolume | Squelch | FilterWidth
            | FilterShift | AgcMaxGain | ManualGain | TxDrive | TuneDrive | MicGain
            | DigiAudioFreq | SpectrumZoom | SpectrumPan | SpectrumFloorDb | SpectrumCeilDb => {
                ActionKind::Continuous
            }
            _ => ActionKind::Momentary,
        }
    }

    /// Which section of the bindings editor this belongs under.
    pub fn group(self) -> &'static str {
        use Action::*;
        match self {
            Tune | RitOffset | XitOffset | RitEnable | XitEnable | RitClear | XitClear
            | VfoSelect(_) | VfoToggle | SwapVfos | CopyAtoB | Split | BandUp | BandDown
            | BandSelect(_) | MemoryRecall(_) => "Tuning",
            Volume | SubVolume | Squelch | FilterWidth | FilterShift | AgcMaxGain | ManualGain
            | Mute | NoiseBlanker | NoiseReductionCycle | AutoNotch | Binaural | AgcCycle
            | SubRx | ModeNext | ModePrev | ModeSelect(_) | RecordToggle => "Receive",
            Ptt | TuneCarrier | TxDrive | TuneDrive | MicGain | DigiAudioFreq | AbortTx
            | CwStraight | VoicePlay(_) | VoiceStop | ToneBurst => "Transmit",
            SpectrumZoom | SpectrumPan | SpectrumFloorDb | SpectrumCeilDb | FitSpan | ZoomIn
            | ZoomOut | PeakHold | SpectrumCollapse | WaterfallCollapse | WaterfallFlip => {
                "Display"
            }
            ToggleHelp | ToggleSettings | ToggleLogbook | ToggleSpots | ToggleMemories
            | ToggleVoice => "Windows",
            SpeakStatus | SpeakRepeat | SpeechSilence | SpeechToggle => "Speech",
        }
    }

    pub fn label(self) -> String {
        use Action::*;
        let fixed = match self {
            Tune => "Tune VFO",
            RitOffset => "RIT offset",
            XitOffset => "XIT offset",
            Volume => "Volume",
            SubVolume => "Sub-RX volume",
            Squelch => "Squelch",
            FilterWidth => "Filter width",
            FilterShift => "Filter shift",
            AgcMaxGain => "AGC max gain",
            ManualGain => "Manual gain (AGC off)",
            TxDrive => "TX drive",
            TuneDrive => "Tune drive",
            MicGain => "Mic gain",
            DigiAudioFreq => "Digi audio offset",
            SpectrumZoom => "Spectrum zoom",
            SpectrumPan => "Spectrum pan",
            SpectrumFloorDb => "Spectrum floor",
            SpectrumCeilDb => "Spectrum ceiling",
            Ptt => "PTT",
            TuneCarrier => "TUNE carrier",
            Mute => "Mute",
            NoiseBlanker => "Noise blanker",
            NoiseReductionCycle => "Noise reduction (cycle)",
            AutoNotch => "Auto notch",
            Binaural => "Binaural audio",
            AgcCycle => "AGC (cycle)",
            SubRx => "Sub receiver",
            Split => "Split",
            RitEnable => "RIT on/off",
            XitEnable => "XIT on/off",
            RitClear => "RIT clear",
            XitClear => "XIT clear",
            VfoToggle => "VFO A/B toggle",
            SwapVfos => "Swap A↔B",
            CopyAtoB => "Copy A→B",
            BandUp => "Band up",
            BandDown => "Band down",
            ModeNext => "Mode next",
            ModePrev => "Mode previous",
            RecordToggle => "Record on/off",
            AbortTx => "Abort transmit",
            CwStraight => "CW straight key",
            VoiceStop => "Voice keyer stop",
            ToneBurst => "1750 Hz tone burst",
            FitSpan => "Fit span",
            ZoomIn => "Zoom in",
            ZoomOut => "Zoom out",
            PeakHold => "Peak hold",
            SpectrumCollapse => "Collapse spectrum",
            WaterfallCollapse => "Collapse waterfall",
            WaterfallFlip => "Flip waterfall",
            ToggleHelp => "Manual window",
            ToggleSettings => "Settings window",
            ToggleLogbook => "Logbook window",
            ToggleSpots => "Spots window",
            ToggleMemories => "Memories window",
            ToggleVoice => "Voice keyer window",
            SpeakStatus => "Speak status",
            SpeakRepeat => "Repeat last announcement",
            SpeechSilence => "Stop speaking",
            SpeechToggle => "Announcements on/off",
            VfoSelect(v) => return format!("Select VFO {}", if v == Vfo::A { "A" } else { "B" }),
            BandSelect(b) => return format!("Band {}", b.label()),
            ModeSelect(m) => return format!("Mode {}", m.label()),
            MemoryRecall(n) => return format!("Recall memory {n}"),
            VoicePlay(n) => return format!("Voice keyer slot {}", n + 1),
        };
        fixed.to_string()
    }

    /// Sensible [`BindingTuning::step`] when this action is freshly bound —
    /// Hz for frequencies, dB for levels, a 0..1 fraction for gains.
    pub fn default_step(self) -> f32 {
        use Action::*;
        match self {
            Tune => 100.0,
            RitOffset | XitOffset | DigiAudioFreq => 10.0,
            FilterWidth | FilterShift => 50.0,
            Volume | SubVolume | TxDrive | TuneDrive | MicGain => 0.02,
            Squelch | AgcMaxGain | ManualGain | SpectrumFloorDb | SpectrumCeilDb => 1.0,
            SpectrumZoom => 0.05,
            SpectrumPan => 0.02,
            _ => 1.0,
        }
    }

    /// Every action offered by the bindings editor, minus [`Action::MemoryRecall`]
    /// (the editor appends one entry per stored memory channel).
    pub fn all() -> Vec<Action> {
        use Action::*;
        let mut v = vec![
            Tune,
            RitOffset,
            XitOffset,
            Volume,
            SubVolume,
            Squelch,
            FilterWidth,
            FilterShift,
            AgcMaxGain,
            ManualGain,
            TxDrive,
            TuneDrive,
            MicGain,
            DigiAudioFreq,
            SpectrumZoom,
            SpectrumPan,
            SpectrumFloorDb,
            SpectrumCeilDb,
            Ptt,
            TuneCarrier,
            Mute,
            NoiseBlanker,
            NoiseReductionCycle,
            AutoNotch,
            Binaural,
            AgcCycle,
            SubRx,
            Split,
            RitEnable,
            XitEnable,
            RitClear,
            XitClear,
            VfoSelect(Vfo::A),
            VfoSelect(Vfo::B),
            VfoToggle,
            SwapVfos,
            CopyAtoB,
            BandUp,
            BandDown,
        ];
        v.extend(Band::ALL.iter().map(|b| BandSelect(*b)));
        v.extend([ModeNext, ModePrev]);
        v.extend(Mode::ALL.iter().map(|m| ModeSelect(*m)));
        v.extend([RecordToggle, AbortTx, CwStraight, VoiceStop, ToneBurst]);
        v.extend((0..crate::VOICE_SLOTS as u8).map(VoicePlay));
        v.extend([
            FitSpan,
            ZoomIn,
            ZoomOut,
            PeakHold,
            SpectrumCollapse,
            WaterfallCollapse,
            WaterfallFlip,
            ToggleHelp,
            ToggleSettings,
            ToggleLogbook,
            ToggleSpots,
            ToggleMemories,
            ToggleVoice,
            SpeakStatus,
            SpeakRepeat,
            SpeechSilence,
            SpeechToggle,
        ]);
        v
    }
}

/// One input sample, whatever produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionInput {
    /// Signed move `d` in the action's own units, already summed for this
    /// frame. Acceleration is applied by the resolver, not by the transport.
    ///
    /// `step` is the binding's own step — the grid a stepped action lands on,
    /// so that a dial left off it by a continuous gesture (a panadapter drag)
    /// is put back on it rather than carrying the offset forever. 0 asks for
    /// no grid at all.
    Delta {
        d: f32,
        step: f32,
    },
    /// Absolute position, 0.0..=1.0 (fader, absolute CC, pitch bend).
    Absolute(f32),
    Press,
    Release,
}

/// Per-binding scaling, shared by every transport.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BindingTuning {
    /// Units per detent — see [`Action::default_step`].
    pub step: f32,
    /// Speed sensitivity. 0 is linear; the effective step is
    /// `step * (1 + accel * min(ticks_per_sec / ACCEL_REF_HZ, ACCEL_MAX))`.
    pub accel: f32,
    pub invert: bool,
}

impl Default for BindingTuning {
    fn default() -> Self {
        BindingTuning { step: 1.0, accel: 0.0, invert: false }
    }
}

impl BindingTuning {
    /// Tick rate at which the acceleration term reaches its nominal weight.
    pub const ACCEL_REF_HZ: f32 = 50.0;
    /// Ceiling on the acceleration multiplier, so a fast spin stays bounded.
    pub const ACCEL_MAX: f32 = 8.0;

    pub fn with_step(step: f32) -> Self {
        BindingTuning { step, ..BindingTuning::default() }
    }

    /// Effective step for a control currently ticking at `rate_hz`.
    pub fn effective_step(&self, rate_hz: f32) -> f32 {
        let boost = if self.accel > 0.0 {
            1.0 + self.accel * (rate_hz / Self::ACCEL_REF_HZ).min(Self::ACCEL_MAX)
        } else {
            1.0
        };
        self.step * boost * if self.invert { -1.0 } else { 1.0 }
    }
}

/// How a button-shaped input drives a momentary action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ButtonMode {
    /// Hold-to-act: press asserts, release de-asserts. Required for PTT.
    #[default]
    Momentary,
    /// Press flips the current state; release is ignored.
    Toggle,
}

impl ButtonMode {
    pub const ALL: [ButtonMode; 2] = [ButtonMode::Momentary, ButtonMode::Toggle];

    pub fn label(self) -> &'static str {
        match self {
            ButtonMode::Momentary => "Hold",
            ButtonMode::Toggle => "Toggle",
        }
    }
}

/// A keyboard chord.
///
/// `key` is an `egui::Key::name()` string ("ArrowRight", "F", "Space"), so this
/// type stays free of any egui dependency and round-trips through
/// `egui::Key::from_name`. `ctrl` means Ctrl, or ⌘ on macOS — one binding works
/// on every platform.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyChord {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyChord {
    pub fn plain(key: &str) -> Self {
        KeyChord { key: key.to_string(), ..KeyChord::default() }
    }

    pub fn shift(key: &str) -> Self {
        KeyChord { key: key.to_string(), shift: true, ..KeyChord::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Human-readable form, e.g. `Shift+ArrowRight`.
    pub fn label(&self) -> String {
        if self.key.is_empty() {
            return "—".to_string();
        }
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(&self.key);
        s
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBinding {
    pub chord: KeyChord,
    pub action: Action,
    /// Delta emitted per press for a continuous action; the sign is the
    /// direction. Ignored for momentary actions.
    pub value: f32,
    pub tuning: BindingTuning,
    pub button: ButtonMode,
    pub enabled: bool,
}

impl Default for KeyBinding {
    fn default() -> Self {
        KeyBinding {
            chord: KeyChord::default(),
            action: Action::Tune,
            value: 1.0,
            tuning: BindingTuning::with_step(Action::Tune.default_step()),
            button: ButtonMode::Momentary,
            enabled: true,
        }
    }
}

impl KeyBinding {
    fn tune(chord: KeyChord, value: f32, step: f32) -> Self {
        KeyBinding {
            chord,
            action: Action::Tune,
            value,
            tuning: BindingTuning::with_step(step),
            button: ButtonMode::Momentary,
            enabled: true,
        }
    }

    fn toggle(chord: KeyChord, action: Action) -> Self {
        KeyBinding {
            chord,
            action,
            value: 1.0,
            tuning: BindingTuning::with_step(action.default_step()),
            button: ButtonMode::Toggle,
            enabled: true,
        }
    }

    /// A momentary binding — hold to act, release to stop. Used by the CW
    /// straight key, which is the one binding whose *held* state matters rather
    /// than its edges.
    fn momentary(chord: KeyChord, action: Action) -> Self {
        KeyBinding {
            chord,
            action,
            value: 1.0,
            tuning: BindingTuning::with_step(action.default_step()),
            button: ButtonMode::Momentary,
            enabled: true,
        }
    }

    /// The shipped defaults. These reproduce the shortcuts sdroxide had before
    /// bindings were configurable, so an operator who never opens the editor
    /// sees no change.
    ///
    /// PTT is deliberately *not* bound: an accidentally keyed transmitter is
    /// the worst failure mode here, so the operator opts in.
    ///
    /// The voice-keyer digits *are* bound, because they can only ever transmit
    /// something the operator recorded on purpose: an empty slot does nothing,
    /// and a fresh installation has ten of them. Note that the numpad digits
    /// are not distinguishable from the top-row ones here — the platform layer
    /// reports both as plain digits — so either row fires a slot.
    pub fn defaults() -> Vec<KeyBinding> {
        let mut v = vec![
            KeyBinding::tune(KeyChord::plain("ArrowRight"), 1.0, 100.0),
            KeyBinding::tune(KeyChord::plain("ArrowLeft"), -1.0, 100.0),
            KeyBinding::tune(KeyChord::shift("ArrowRight"), 1.0, 10.0),
            KeyBinding::tune(KeyChord::shift("ArrowLeft"), -1.0, 10.0),
            KeyBinding::tune(KeyChord::plain("ArrowUp"), 1.0, 1_000.0),
            KeyBinding::tune(KeyChord::plain("ArrowDown"), -1.0, 1_000.0),
            KeyBinding::tune(KeyChord::plain("PageUp"), 1.0, 10_000.0),
            KeyBinding::tune(KeyChord::plain("PageDown"), -1.0, 10_000.0),
            KeyBinding::toggle(KeyChord::plain("M"), Action::Mute),
            KeyBinding::toggle(KeyChord::plain("N"), Action::NoiseBlanker),
            KeyBinding::toggle(KeyChord::plain("F"), Action::FitSpan),
            KeyBinding::toggle(KeyChord::plain("V"), Action::WaterfallFlip),
            // The CW straight key keeps its historical Space bar, now as a
            // binding so any key can be chosen instead (the space bar's travel
            // is long for keying). Only armed by the CW panel's KEY toggle, so
            // it types a space everywhere else.
            KeyBinding::momentary(KeyChord::plain("Space"), Action::CwStraight),
        ];
        // Numpad 1–9 then 0 play slots 1–10; numpad "−" stops a message early.
        for slot in 0..crate::VOICE_SLOTS as u8 {
            let digit = (slot + 1) % 10; // slot 10 sits on the 0 key
            v.push(KeyBinding::toggle(
                KeyChord::plain(&digit.to_string()),
                Action::VoicePlay(slot),
            ));
        }
        v.push(KeyBinding::toggle(KeyChord::plain("Minus"), Action::VoiceStop));
        v
    }
}

/// What the wheel does over the panadapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WheelAction {
    Zoom,
    Tune,
    None,
}

impl WheelAction {
    pub const ALL: [WheelAction; 3] = [WheelAction::Zoom, WheelAction::Tune, WheelAction::None];

    pub fn label(self) -> &'static str {
        match self {
            WheelAction::Zoom => "Zoom",
            WheelAction::Tune => "Tune",
            WheelAction::None => "Nothing",
        }
    }
}

/// Panadapter pointer behaviour. `Copy`, so it can be handed to the spectrum
/// widget without an allocation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WheelSettings {
    pub wheel: WheelAction,
    /// What the wheel does with Shift held — usually the other behaviour.
    pub wheel_shift: WheelAction,
    pub invert: bool,
    /// Hz per wheel detent in [`WheelAction::Tune`].
    pub tune_step_hz: f64,
    /// Zoom rate multiplier; 1.0 is the shipped feel.
    pub zoom_rate: f32,
    /// Rounding applied by click-to-tune.
    pub click_tune_step_hz: f64,
    /// Left-drag on the panadapter drags the tuning along with the view.
    pub drag_tunes: bool,
    /// Per-digit scroll stepping on the frequency readout.
    pub digit_wheel: bool,
}

impl Default for WheelSettings {
    fn default() -> Self {
        WheelSettings {
            wheel: WheelAction::Zoom,
            wheel_shift: WheelAction::Tune,
            invert: false,
            tune_step_hz: 100.0,
            zoom_rate: 1.0,
            click_tune_step_hz: 10.0,
            drag_tunes: true,
            digit_wheel: true,
        }
    }
}

/// Mouse buttons that can carry a binding. The primary and secondary buttons
/// are reserved for tuning and panning the panadapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Middle,
    Extra1,
    Extra2,
}

impl MouseButton {
    pub const ALL: [MouseButton; 3] =
        [MouseButton::Middle, MouseButton::Extra1, MouseButton::Extra2];

    pub fn label(self) -> &'static str {
        match self {
            MouseButton::Middle => "Middle",
            MouseButton::Extra1 => "Back (extra 1)",
            MouseButton::Extra2 => "Forward (extra 2)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseButtonBinding {
    pub button: MouseButton,
    pub action: Action,
    pub tuning: BindingTuning,
    pub button_mode: ButtonMode,
    pub enabled: bool,
}

impl Default for MouseButtonBinding {
    fn default() -> Self {
        MouseButtonBinding {
            button: MouseButton::Middle,
            action: Action::Ptt,
            tuning: BindingTuning::default(),
            button_mode: ButtonMode::Momentary,
            enabled: true,
        }
    }
}

// ── MIDI ────────────────────────────────────────────────────────────────────

/// The MIDI message class a binding matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MidiMsgKind {
    Note,
    Cc,
    PitchBend,
    ProgramChange,
}

impl MidiMsgKind {
    pub fn label(self) -> &'static str {
        match self {
            MidiMsgKind::Note => "Note",
            MidiMsgKind::Cc => "CC",
            MidiMsgKind::PitchBend => "Pitch bend",
            MidiMsgKind::ProgramChange => "Program",
        }
    }
}

/// The identity of one physical control on a MIDI surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiMsg {
    pub kind: MidiMsgKind,
    /// 0..=15, or [`MidiMsg::ANY_CHANNEL`] to match every channel (omni).
    pub channel: u8,
    /// Note number or CC number. Unused for pitch bend.
    pub data1: u8,
}

impl Default for MidiMsg {
    fn default() -> Self {
        MidiMsg { kind: MidiMsgKind::Cc, channel: MidiMsg::ANY_CHANNEL, data1: 0 }
    }
}

impl MidiMsg {
    pub const ANY_CHANNEL: u8 = 0xFF;

    /// Whether an incoming control matches this binding's identity. Pitch bend
    /// has no controller number, so `data1` is not compared for it.
    pub fn matches(&self, other: &MidiMsg) -> bool {
        if self.kind != other.kind {
            return false;
        }
        if self.channel != MidiMsg::ANY_CHANNEL
            && other.channel != MidiMsg::ANY_CHANNEL
            && self.channel != other.channel
        {
            return false;
        }
        self.kind == MidiMsgKind::PitchBend || self.data1 == other.data1
    }

    pub fn label(&self) -> String {
        let ch = if self.channel == MidiMsg::ANY_CHANNEL {
            "any".to_string()
        } else {
            (self.channel + 1).to_string()
        };
        match self.kind {
            MidiMsgKind::PitchBend => format!("Pitch bend  ch {ch}"),
            k => format!("{} {}  ch {ch}", k.label(), self.data1),
        }
    }
}

/// How a 7-bit value from an endless/relative encoder is decoded.
///
/// Controllers disagree on this and rarely document it. The three relative
/// encodings below are the ones in the wild; DJ/DAW surfaces label them
/// "Rel-1/2/3". The Behringer CMD PL-1 jog wheel ships in [`Self::BinaryOffset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RelativeMode {
    /// Absolute fader or knob: 0..=127 maps to 0.0..=1.0.
    #[default]
    Absolute,
    /// "Rel-1", signed bit: `0x01..=0x3F` = +n, `0x41..=0x7F` = −(v − 0x40).
    SignMagnitude,
    /// "Rel-2", two's complement: 1..=63 = +n, 127..=65 = −(128 − v).
    TwosComplement,
    /// "Rel-3", binary offset centred on 64: v − 64.
    BinaryOffset,
}

impl RelativeMode {
    pub const ALL: [RelativeMode; 4] = [
        RelativeMode::Absolute,
        RelativeMode::SignMagnitude,
        RelativeMode::TwosComplement,
        RelativeMode::BinaryOffset,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RelativeMode::Absolute => "Absolute (fader)",
            RelativeMode::SignMagnitude => "Relative: sign/magnitude",
            RelativeMode::TwosComplement => "Relative: two's complement",
            RelativeMode::BinaryOffset => "Relative: 64-centred",
        }
    }

    pub fn is_relative(self) -> bool {
        !matches!(self, RelativeMode::Absolute)
    }

    /// Signed detent count for a relative encoding. `None` for
    /// [`Self::Absolute`], where the caller uses `v as f32 / 127.0` instead.
    pub fn decode(self, v: u8) -> Option<i8> {
        let v = v & 0x7F;
        Some(match self {
            RelativeMode::Absolute => return None,
            RelativeMode::SignMagnitude => match v {
                0x01..=0x3F => v as i8,
                0x41..=0x7F => -((v - 0x40) as i8),
                _ => 0,
            },
            RelativeMode::TwosComplement => match v {
                1..=63 => v as i8,
                65..=127 => -((128 - v) as i8),
                _ => 0,
            },
            RelativeMode::BinaryOffset => v as i8 - 64,
        })
    }

    /// Best guess at the encoding behind a burst of values captured while the
    /// operator turned a control during LEARN.
    ///
    /// [`Self::SignMagnitude`] and [`Self::BinaryOffset`] are **not**
    /// distinguishable from small-magnitude values alone — sign/magnitude −1 is
    /// `0x41` = 65, and binary-offset +1 is also 65 — so the two are mutually
    /// inverted there. The learn UI resolves it by asking for a clockwise turn
    /// and offering an invert toggle.
    pub fn guess(values: &[u8]) -> RelativeMode {
        if values.len() < 2 {
            return RelativeMode::Absolute;
        }
        let count =
            |r: std::ops::RangeInclusive<u8>| values.iter().filter(|v| r.contains(v)).count();
        let small_pos = count(1..=8);
        let high_wrap = count(120..=127);
        let low_neg = count(65..=72);
        // Only a 64-centred encoder emits small deltas just *below* 64.
        let near_below = count(56..=63);

        if near_below > 0 && values.iter().all(|v| v.abs_diff(64) <= 8) {
            return RelativeMode::BinaryOffset;
        }
        if high_wrap > 0 && small_pos + high_wrap == values.len() {
            return RelativeMode::TwosComplement;
        }
        if small_pos > 0 && small_pos + low_neg == values.len() {
            return RelativeMode::SignMagnitude;
        }
        // Nothing but 65..=72: either binary-offset +1..+8 or sign/magnitude
        // −1..−8, and no way to tell. LEARN asked for a clockwise turn, so
        // pick the reading that makes clockwise positive.
        if low_neg == values.len() {
            return RelativeMode::BinaryOffset;
        }
        RelativeMode::Absolute
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiBinding {
    pub msg: MidiMsg,
    pub action: Action,
    pub relative: RelativeMode,
    pub tuning: BindingTuning,
    pub button_mode: ButtonMode,
    /// Echo the resulting state back to the controller, so an LED lights or a
    /// motor fader follows. Opt-in: not every surface tolerates it.
    pub feedback: bool,
    pub enabled: bool,
}

impl Default for MidiBinding {
    fn default() -> Self {
        MidiBinding {
            msg: MidiMsg::default(),
            action: Action::Tune,
            relative: RelativeMode::default(),
            tuning: BindingTuning::with_step(Action::Tune.default_step()),
            button_mode: ButtonMode::Momentary,
            feedback: false,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MidiSettings {
    pub enabled: bool,
    /// Stable midir port id. The name is the human label, and the fallback when
    /// the id changes across a replug.
    pub in_port_id: String,
    pub in_port_name: String,
    pub out_port_id: String,
    pub out_port_name: String,
    pub bindings: Vec<MidiBinding>,
}

// ── Icom RC-28 ──────────────────────────────────────────────────────────────

/// What one of the RC-28's three buttons does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rc28Button {
    pub action: Action,
    pub button_mode: ButtonMode,
    /// Light the LED over the button while its action is on (PTT keyed, split
    /// set). Only meaningful for actions with a state to show.
    pub led: bool,
    pub enabled: bool,
}

impl Default for Rc28Button {
    fn default() -> Self {
        Rc28Button::toggle(Action::VfoToggle)
    }
}

impl Rc28Button {
    fn toggle(action: Action) -> Self {
        Rc28Button { action, button_mode: ButtonMode::Toggle, led: true, enabled: true }
    }
}

/// The Icom RC-28 remote encoder: a knob and three buttons, each bound to one
/// action.
///
/// Fixed controls rather than a binding table like MIDI's, because the device
/// is: there is nothing to learn, and a row per physical control is the whole
/// editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rc28Settings {
    /// Off until the operator turns it on, for the reason PTT is never a
    /// default key binding: TRANSMIT keys the rig.
    pub enabled: bool,
    pub knob: Action,
    /// Per detent — see `sdroxide_rc28::proto::COUNTS_PER_DETENT`, about 67 to
    /// the turn.
    pub knob_tuning: BindingTuning,
    pub transmit: Rc28Button,
    pub f1: Rc28Button,
    pub f2: Rc28Button,
}

impl Default for Rc28Settings {
    fn default() -> Self {
        Rc28Settings {
            enabled: false,
            knob: Action::Tune,
            // 10 Hz a detent is about 670 Hz a turn, a transceiver's main dial,
            // with enough acceleration that a spin crosses a band.
            knob_tuning: BindingTuning { step: 10.0, accel: 1.0, invert: false },
            // Hold to talk: what the button is labelled, and the mode that
            // cannot leave the transmitter keyed when it is let go.
            transmit: Rc28Button {
                action: Action::Ptt,
                button_mode: ButtonMode::Momentary,
                led: true,
                enabled: true,
            },
            f1: Rc28Button::toggle(Action::VfoToggle),
            f2: Rc28Button::toggle(Action::Split),
        }
    }
}

/// Everything client-local input control needs.
///
/// Persisted as `input.json` on native and in eframe storage on wasm. It lives
/// with the *client*, not the engine, so a knob plugged into the operator's
/// laptop works against a remote engine too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputSettings {
    pub keys: Vec<KeyBinding>,
    pub wheel: WheelSettings,
    pub mouse_buttons: Vec<MouseButtonBinding>,
    /// Unkey a held momentary PTT after this long, as a stuck-key /
    /// stuck-controller backstop. 0 disables the timeout.
    pub ptt_hold_timeout_s: f32,
    pub midi: MidiSettings,
    pub rc28: Rc28Settings,
    /// Which set of shipped defaults this file has already seen. See
    /// [`InputSettings::SCHEMA`] and [`InputSettings::migrate`].
    ///
    /// The field default is spelled out rather than left to the struct's
    /// `#[serde(default)]`: that one fills a missing field from
    /// [`InputSettings::default`], which carries the *current* schema — so a
    /// file written before the stamp existed would read as already migrated
    /// and the migration would never run on the only files that need it.
    #[serde(default = "schema_before_stamps")]
    pub schema: u32,
}

/// What [`InputSettings::schema`] reads as in a file written before the field
/// existed: nothing has been migrated into it yet.
fn schema_before_stamps() -> u32 {
    0
}

impl Default for InputSettings {
    fn default() -> Self {
        InputSettings {
            keys: KeyBinding::defaults(),
            wheel: WheelSettings::default(),
            mouse_buttons: Vec::new(),
            ptt_hold_timeout_s: 300.0,
            midi: MidiSettings::default(),
            rc28: Rc28Settings::default(),
            schema: InputSettings::SCHEMA,
        }
    }
}

impl InputSettings {
    /// The current shipped-defaults generation. Bumped whenever a release adds
    /// a *new* default binding — see [`Self::migrate`] for why that is not
    /// free.
    pub const SCHEMA: u32 = 1;

    /// Bindings introduced at each schema step, so a saved file picks up a new
    /// action's default instead of silently losing the key.
    ///
    /// The keys are stored as a plain list, so a new entry in
    /// [`KeyBinding::defaults`] simply does not exist in a file written before
    /// it was added — and the feature it drives is then dead with nothing on
    /// screen to say why. That is what happened to the CW straight key: it had
    /// been hard-wired to the space bar, became [`Action::CwStraight`] so any
    /// key could drive it, and every operator who had ever opened the Controls
    /// tab found the space bar doing nothing.
    ///
    /// Only ever *adds*, and only where the operator has no binding for that
    /// action at all — a binding they moved, disabled or deleted after the
    /// migration ran is theirs, and the stamped `schema` is what stops this
    /// putting it back.
    const ADDED: &'static [(u32, Action)] = &[(1, Action::CwStraight)];

    /// Bring a loaded file up to [`Self::SCHEMA`], reporting whether it had to
    /// be touched — and so whether it is worth writing back. The stamp is
    /// written even when no binding was added, because it is the stamp that
    /// keeps a later deletion deleted.
    pub fn migrate(&mut self) -> bool {
        if self.schema >= Self::SCHEMA {
            return false;
        }
        let shipped = KeyBinding::defaults();
        for &(at, action) in Self::ADDED {
            if self.schema >= at || self.keys.iter().any(|b| b.action == action) {
                continue;
            }
            if let Some(b) = shipped.iter().find(|b| b.action == action) {
                self.keys.push(b.clone());
            }
        }
        self.schema = Self::SCHEMA;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_has_no_detents() {
        assert_eq!(RelativeMode::Absolute.decode(64), None);
    }

    /// A settings file saved before `CwStraight` existed picks the binding up,
    /// so the space bar still keys a straight key after the upgrade instead of
    /// quietly doing nothing (issue #495 follow-up).
    #[test]
    fn an_old_file_gains_the_bindings_added_since() {
        let mut old = InputSettings {
            keys: KeyBinding::defaults()
                .into_iter()
                .filter(|b| b.action != Action::CwStraight)
                .collect(),
            schema: 0,
            ..InputSettings::default()
        };
        assert!(old.migrate(), "an old file has to be written back");
        let straight: Vec<_> = old.keys.iter().filter(|b| b.action == Action::CwStraight).collect();
        assert_eq!(straight.len(), 1, "exactly one straight-key binding");
        assert_eq!(straight[0].chord, KeyChord::plain("Space"));
        assert!(straight[0].enabled);
        assert_eq!(old.schema, InputSettings::SCHEMA);
    }

    /// A settings file from before the RC-28 was supported loads with it off,
    /// and with TRANSMIT as hold-to-talk for when it is turned on.
    #[test]
    fn a_file_without_the_rc28_leaves_it_off() {
        let mut v: serde_json::Value = serde_json::to_value(InputSettings::default()).unwrap();
        v.as_object_mut().unwrap().remove("rc28");
        let cfg: InputSettings = serde_json::from_value(v).unwrap();
        assert!(!cfg.rc28.enabled);
        assert_eq!(cfg.rc28.transmit.action, Action::Ptt);
        assert_eq!(cfg.rc28.transmit.button_mode, ButtonMode::Momentary);
    }

    /// The real path: a file on disk with no `schema` key at all.
    #[test]
    fn a_file_written_before_the_stamp_existed_reads_as_schema_zero() {
        let mut cfg = InputSettings::default();
        cfg.keys.retain(|b| b.action != Action::CwStraight);
        let mut v: serde_json::Value = serde_json::to_value(&cfg).unwrap();
        v.as_object_mut().unwrap().remove("schema");
        let mut back: InputSettings = serde_json::from_value(v).unwrap();
        assert_eq!(back.schema, 0, "a file with no stamp must read as 0, not as current");
        assert!(back.migrate());
        assert!(back.keys.iter().any(|b| b.action == Action::CwStraight));
    }

    /// ...and only once. An operator who deletes the binding afterwards keeps
    /// it deleted: the stamp says this file has already seen that default.
    #[test]
    fn a_migrated_file_does_not_get_it_back() {
        let mut cfg = InputSettings::default();
        cfg.keys.retain(|b| b.action != Action::CwStraight);
        assert!(!cfg.migrate(), "a current file needs no write-back");
        assert!(!cfg.keys.iter().any(|b| b.action == Action::CwStraight));
    }

    /// A binding the operator moved to another key is theirs, and is left
    /// alone rather than joined by a second one on the shipped default.
    #[test]
    fn a_rebound_action_is_not_given_the_default_as_well() {
        let mut old = InputSettings { schema: 0, ..InputSettings::default() };
        for b in old.keys.iter_mut().filter(|b| b.action == Action::CwStraight) {
            b.chord = KeyChord::plain("Z");
        }
        old.migrate();
        let straight: Vec<_> = old.keys.iter().filter(|b| b.action == Action::CwStraight).collect();
        assert_eq!(straight.len(), 1);
        assert_eq!(straight[0].chord, KeyChord::plain("Z"));
    }

    #[test]
    fn sign_magnitude_round_trip() {
        let m = RelativeMode::SignMagnitude;
        assert_eq!(m.decode(0x01), Some(1));
        assert_eq!(m.decode(0x03), Some(3));
        assert_eq!(m.decode(0x3F), Some(63));
        assert_eq!(m.decode(0x41), Some(-1));
        assert_eq!(m.decode(0x43), Some(-3));
        assert_eq!(m.decode(0x7F), Some(-63));
        assert_eq!(m.decode(0x00), Some(0));
        assert_eq!(m.decode(0x40), Some(0));
    }

    #[test]
    fn twos_complement_round_trip() {
        let m = RelativeMode::TwosComplement;
        assert_eq!(m.decode(1), Some(1));
        assert_eq!(m.decode(63), Some(63));
        assert_eq!(m.decode(127), Some(-1));
        assert_eq!(m.decode(65), Some(-63));
        assert_eq!(m.decode(0), Some(0));
        assert_eq!(m.decode(64), Some(0));
    }

    #[test]
    fn binary_offset_is_centred_on_64() {
        let m = RelativeMode::BinaryOffset;
        assert_eq!(m.decode(64), Some(0));
        assert_eq!(m.decode(65), Some(1));
        assert_eq!(m.decode(63), Some(-1));
        assert_eq!(m.decode(127), Some(63));
        assert_eq!(m.decode(0), Some(-64));
    }

    /// The overlap that makes LEARN ambiguous: 65 is +1 one way and −1 the
    /// other. The guesser must not pretend it can tell them apart.
    #[test]
    fn sixty_five_is_ambiguous() {
        assert_eq!(RelativeMode::BinaryOffset.decode(65), Some(1));
        assert_eq!(RelativeMode::SignMagnitude.decode(65), Some(-1));
    }

    #[test]
    fn guess_encodings() {
        assert_eq!(RelativeMode::guess(&[63, 65, 64, 65, 63]), RelativeMode::BinaryOffset);
        assert_eq!(RelativeMode::guess(&[1, 1, 127, 126, 2]), RelativeMode::TwosComplement);
        assert_eq!(RelativeMode::guess(&[1, 2, 66, 67]), RelativeMode::SignMagnitude);
        assert_eq!(RelativeMode::guess(&[0, 20, 40, 90, 127]), RelativeMode::Absolute);
        assert_eq!(RelativeMode::guess(&[5]), RelativeMode::Absolute);
        // Clockwise-only on an ambiguous encoder: read it as positive.
        assert_eq!(RelativeMode::guess(&[65, 66, 65]), RelativeMode::BinaryOffset);
    }

    #[test]
    fn omni_channel_matches_anything() {
        let omni = MidiMsg { kind: MidiMsgKind::Cc, channel: MidiMsg::ANY_CHANNEL, data1: 16 };
        let ch3 = MidiMsg { kind: MidiMsgKind::Cc, channel: 2, data1: 16 };
        assert!(omni.matches(&ch3));
        assert!(!omni.matches(&MidiMsg { data1: 17, ..ch3 }));
        assert!(!omni.matches(&MidiMsg { kind: MidiMsgKind::Note, ..ch3 }));
    }

    #[test]
    fn pitch_bend_ignores_controller_number() {
        let a = MidiMsg { kind: MidiMsgKind::PitchBend, channel: 0, data1: 0 };
        let b = MidiMsg { kind: MidiMsgKind::PitchBend, channel: 0, data1: 99 };
        assert!(a.matches(&b));
    }

    /// The no-regression contract: the shipped bindings must be exactly the
    /// shortcuts sdroxide had before they became configurable.
    #[test]
    fn defaults_reproduce_the_legacy_shortcuts() {
        let d = KeyBinding::defaults();
        let find = |label: &str| {
            d.iter().find(|b| b.chord.label() == label).unwrap_or_else(|| panic!("missing {label}"))
        };
        assert_eq!(find("ArrowRight").tuning.step, 100.0);
        assert_eq!(find("ArrowRight").value, 1.0);
        assert_eq!(find("ArrowLeft").value, -1.0);
        assert_eq!(find("Shift+ArrowRight").tuning.step, 10.0);
        assert_eq!(find("ArrowUp").tuning.step, 1_000.0);
        assert_eq!(find("PageUp").tuning.step, 10_000.0);
        assert_eq!(find("M").action, Action::Mute);
        assert_eq!(find("N").action, Action::NoiseBlanker);
        assert_eq!(find("F").action, Action::FitSpan);
        // PTT must never ship bound.
        assert!(!d.iter().any(|b| b.action == Action::Ptt));
    }

    /// The keypad plays slot 1 on "1" … slot 10 on "0", and every slot has
    /// exactly one shipped key.
    #[test]
    fn voice_slots_ship_on_the_numpad() {
        let d = KeyBinding::defaults();
        let key_for = |slot: u8| {
            d.iter()
                .find(|b| b.action == Action::VoicePlay(slot))
                .map(|b| b.chord.key.clone())
                .unwrap_or_else(|| panic!("slot {slot} unbound"))
        };
        assert_eq!(key_for(0), "1");
        assert_eq!(key_for(8), "9");
        assert_eq!(key_for(9), "0");
        for slot in 0..crate::VOICE_SLOTS as u8 {
            let n = d.iter().filter(|b| b.action == Action::VoicePlay(slot)).count();
            assert_eq!(n, 1, "slot {slot} bound {n} times");
        }
        assert!(d.iter().any(|b| b.action == Action::VoiceStop));
    }

    #[test]
    fn every_action_has_a_label_and_a_group() {
        for a in Action::all() {
            assert!(!a.label().is_empty());
            assert!(!a.group().is_empty());
        }
    }

    #[test]
    fn acceleration_is_bounded_and_signed() {
        let t = BindingTuning { step: 10.0, accel: 1.0, invert: false };
        assert_eq!(t.effective_step(0.0), 10.0);
        // Ceiling: ACCEL_MAX = 8 -> 10 * (1 + 8).
        assert_eq!(t.effective_step(10_000.0), 90.0);
        let inv = BindingTuning { invert: true, ..t };
        assert_eq!(inv.effective_step(0.0), -10.0);
    }
}
