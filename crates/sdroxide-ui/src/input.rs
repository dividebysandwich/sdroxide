//! Control inputs: the runtime that turns keyboard chords, panadapter mouse
//! gestures and MIDI messages into [`Action`]s, and the resolver that turns an
//! `Action` into [`Command`]s or a local view change.
//!
//! This lives with the *client*, not the engine, so a knob plugged into the
//! operator's laptop works whether the radio is in the same process or on the
//! far end of `--connect`. Relative controls are resolved against the local
//! [`RadioState`] and echoed optimistically — the same trick drag-tuning on the
//! panadapter already uses — so a fast spin doesn't wait on a round trip.

use std::collections::HashMap;

use sdroxide_types::{
    Action, ActionInput, ActionKind, BindingTuning, ButtonMode, Command, InputSettings, KeyChord,
    MAX_MANUAL_GAIN_DB, MouseButton, RadioState, RxId, SQUELCH_OPEN_DB, Vfo,
};

use crate::view::ViewState;

/// Gap after which a control counts as newly grabbed, so acceleration starts
/// from rest instead of inheriting the last burst's rate.
const ACCEL_IDLE_S: f64 = 0.2;

/// Ceiling for RIT/XIT offsets, matching the rig-like range reported to
/// external control clients.
const MAX_OFFSET_HZ: f32 = 9999.0;
/// Narrowest a filter may be squeezed by a knob.
const MIN_FILTER_HZ: f32 = 50.0;

/// Side effects an action can have that are purely local to this client and
/// never become a [`Command`]. The app lends the flags it owns.
pub(crate) struct UiSink<'a> {
    pub view: &'a mut ViewState,
    pub help: &'a mut bool,
    pub settings: &'a mut bool,
    pub logbook: &'a mut bool,
    pub spots: &'a mut bool,
    pub memories: &'a mut bool,
    pub voice: &'a mut bool,
    /// Speech actions triggered this frame, for the caller to act on.
    ///
    /// Collected rather than applied here because answering them needs the
    /// meters and the frame clock, neither of which this module has any other
    /// reason to know about — and because the announcer is borrowed from the
    /// same `self` this sink already has torn apart.
    pub speech: &'a mut Vec<Action>,
    /// Whether the SQL action drives the *radio's* squelch rather than the
    /// engine's own gate — `DeviceCaps::commands_squelch`, which this module
    /// has no other reason to know about. A bound knob has to reach the same
    /// control the on-screen rail does, or it moves a threshold the audio never
    /// passes through (issue #192).
    pub rig_squelch: bool,
    /// The window a zoom-out may reach, `(centre, span)` — the front end's
    /// full-band lane where it has one, else its passband. Precomputed by the
    /// caller for the same reason `rig_squelch` is: it comes off the spectrum
    /// lane, which this module has no other reason to know about, and off the
    /// same `self` this sink has already taken apart.
    pub zoom_out: (f64, f64),
}

/// Which binding table an in-flight momentary press came from. Held state is
/// tracked by index so the release path never has to re-match the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeldSource {
    Key(usize),
    Mouse(usize),
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Midi(usize),
}

#[derive(Debug, Clone, Copy)]
struct Held {
    src: HeldSource,
    action: Action,
    mode: ButtonMode,
    since: f64,
}

/// Rolling tick-rate estimate for one action, driving speed-sensitive steps.
#[derive(Debug, Clone, Copy, Default)]
struct TickAccum {
    last_t: f64,
    rate_hz: f32,
}

impl TickAccum {
    /// Record `detents` ticks at `now` and return the smoothed rate.
    fn tick(&mut self, now: f64, detents: f32) -> f32 {
        let dt = now - self.last_t;
        self.last_t = now;
        if dt <= 0.0 || dt > ACCEL_IDLE_S {
            // Start of a fresh turn: no history to smooth against.
            self.rate_hz = 0.0;
            return 0.0;
        }
        let instant = detents.abs() / dt as f32;
        self.rate_hz = self.rate_hz * 0.6 + instant * 0.4;
        self.rate_hz
    }
}

/// The 0..=1 range an absolute control (a fader) spans for this action, or
/// `None` where an absolute position is meaningless (tuning, panning).
///
/// `rig_squelch` is [`UiSink::rig_squelch`]: the SQL action has two scales, and
/// which one a fader spans has to match the one the arm below writes.
fn absolute_range(act: Action, state: &RadioState, rig_squelch: bool) -> Option<(f32, f32)> {
    use Action::*;
    Some(match act {
        Volume | SubVolume | TxDrive | TuneDrive | MicGain => (0.0, 1.0),
        // dBFS for the engine's own gate, and the rig's own `0..1` where the
        // radio is the one squelching.
        Squelch if rig_squelch => (0.0, 1.0),
        Squelch => (SQUELCH_OPEN_DB, 0.0),
        AgcMaxGain => (0.0, 120.0),
        ManualGain => (0.0, MAX_MANUAL_GAIN_DB),
        RitOffset | XitOffset => (-MAX_OFFSET_HZ, MAX_OFFSET_HZ),
        DigiAudioFreq => (100.0, 3500.0),
        SpectrumFloorDb => (-160.0, 0.0),
        SpectrumCeilDb => (-120.0, 40.0),
        FilterWidth => (MIN_FILTER_HZ, state.rx[0].mode.max_filter_hz()),
        _ => return None,
    })
}

/// Apply one input sample to one action.
///
/// Relative moves are resolved against `state` and written back to it before
/// the [`Command`] is queued, so the readout tracks the knob at frame rate
/// instead of waiting for the engine's echo. The engine still has the last
/// word: its reply overwrites `state` on the next snapshot, which is how an
/// out-of-range or ham-band-locked tune snaps back.
pub(crate) fn apply_action(
    act: Action,
    input: ActionInput,
    button: ButtonMode,
    state: &mut RadioState,
    ui: &mut UiSink<'_>,
    cmds: &mut Vec<Command>,
) {
    use Action::*;

    // ── Continuous ──────────────────────────────────────────────────────────
    if act.kind() == ActionKind::Continuous {
        // `delta` is in the action's own units; `abs` is a 0..=1 fader position.
        let (delta, grid, abs) = match input {
            ActionInput::Delta { d, step } => (d, step.abs() as f64, None),
            ActionInput::Absolute(p) => (0.0, 0.0, Some(p.clamp(0.0, 1.0))),
            // A button bound to a continuous action does nothing.
            ActionInput::Press | ActionInput::Release => return,
        };
        // Resolve an absolute position into this action's range up front, so
        // each arm below only has to deal with "here is the new value".
        let target = abs.and_then(|p| {
            absolute_range(act, state, ui.rig_squelch).map(|(lo, hi)| lo + p * (hi - lo))
        });
        if abs.is_some() && target.is_none() {
            return;
        }
        let rx = RxId::Main;
        match act {
            Tune => {
                // Land on the step grid, not one step from wherever the dial
                // happened to sit. Panadapter dragging tunes continuously and
                // leaves a fraction of a step behind it; without this, every
                // later step carries that fraction along for good, and the
                // readout never reaches a round number again.
                let d = delta as f64;
                // A binding may be set to move by less than its own step (a
                // half-detent encoder); then that smaller move is the grid.
                let grid = if d != 0.0 { grid.min(d.abs()) } else { grid };
                let hz = if grid > 0.0 {
                    // Whole grid units, so an accelerated spin stays on it too.
                    (state.active_freq_hz() / grid).round() * grid + (d / grid).round() * grid
                } else {
                    state.active_freq_hz() + d
                };
                let hz = hz.max(0.0);
                match state.active_vfo {
                    Vfo::A => state.vfo_a_hz = hz,
                    Vfo::B => state.vfo_b_hz = hz,
                }
                cmds.push(Command::SetVfo { vfo: state.active_vfo, hz });
            }
            RitOffset => {
                let hz = target
                    .unwrap_or(state.rit.hz as f32 + delta)
                    .clamp(-MAX_OFFSET_HZ, MAX_OFFSET_HZ) as i32;
                state.rit.hz = hz;
                cmds.push(Command::SetRit { enabled: state.rit.enabled, hz });
            }
            XitOffset => {
                let hz = target
                    .unwrap_or(state.xit.hz as f32 + delta)
                    .clamp(-MAX_OFFSET_HZ, MAX_OFFSET_HZ) as i32;
                state.xit.hz = hz;
                cmds.push(Command::SetXit { enabled: state.xit.enabled, hz });
            }
            Volume | SubVolume => {
                let rx = if act == Volume { RxId::Main } else { RxId::Sub };
                let cur = state.rx[rx.index()].volume;
                let v = target.unwrap_or(cur + delta).clamp(0.0, 1.0);
                state.rx[rx.index()].volume = v;
                cmds.push(Command::SetVolume { rx, v });
            }
            Squelch if ui.rig_squelch => {
                // The rig's own scale, `0`..`1`, not dBFS — see `UiSink`, and
                // `absolute_range`, which answers on the same scale so a fader
                // lands where this does.
                //
                // A *relative* step arrives in the action's declared units,
                // and those are dB: `Action::default_step` gives the SQL knob
                // 1.0 without knowing which radio is on the other end, and a
                // step of one on a rail that spans one would be the whole
                // range per detent. One unit is read as one percentage point
                // here, which is what the readout beside it shows.
                let cur = state.rig_squelch;
                let frac = target.unwrap_or(cur + delta / 100.0).clamp(0.0, 1.0);
                state.rig_squelch = frac;
                cmds.push(Command::SetRigSquelch { frac });
            }
            Squelch => {
                let cur = state.rx[0].squelch_db;
                let db = target.unwrap_or(cur + delta).clamp(SQUELCH_OPEN_DB, 0.0);
                state.rx[0].squelch_db = db;
                cmds.push(Command::SetSquelch { rx, db });
            }
            FilterWidth => {
                let r = &state.rx[0];
                let (lo, hi) = (r.filter_lo, r.filter_hi);
                let max = r.mode.max_filter_hz();
                let centre = (lo + hi) / 2.0;
                let half = match target {
                    Some(w) => w / 2.0,
                    None => (hi - lo) / 2.0 + delta / 2.0,
                };
                let half = half.clamp(MIN_FILTER_HZ / 2.0, max);
                let (lo, hi) = (centre - half, centre + half);
                let (lo, hi) = (lo.clamp(-max, max), hi.clamp(-max, max));
                state.rx[0].filter_lo = lo;
                state.rx[0].filter_hi = hi;
                cmds.push(Command::SetFilter { rx, lo, hi });
            }
            FilterShift => {
                let r = &state.rx[0];
                let max = r.mode.max_filter_hz();
                let width = r.filter_hi - r.filter_lo;
                let lo = (r.filter_lo + delta).clamp(-max, max - width);
                let hi = lo + width;
                state.rx[0].filter_lo = lo;
                state.rx[0].filter_hi = hi;
                cmds.push(Command::SetFilter { rx, lo, hi });
            }
            ManualGain => {
                let cur = state.rx[0].manual_gain_db;
                let db = target.unwrap_or(cur + delta).clamp(0.0, MAX_MANUAL_GAIN_DB);
                state.rx[0].manual_gain_db = db;
                cmds.push(Command::SetManualGain { rx, db });
            }
            AgcMaxGain => {
                let cur = state.rx[0].agc_max_gain_db;
                let db = target.unwrap_or(cur + delta).clamp(0.0, 120.0);
                state.rx[0].agc_max_gain_db = db;
                cmds.push(Command::SetAgcMaxGain { rx, db });
            }
            TxDrive => {
                let v = target.unwrap_or(state.tx.drive + delta).clamp(0.0, 1.0);
                state.tx.drive = v;
                cmds.push(Command::SetTxDrive(v));
            }
            TuneDrive => {
                let v = target.unwrap_or(state.tx.tune_drive + delta).clamp(0.0, 1.0);
                state.tx.tune_drive = v;
                cmds.push(Command::SetTuneDrive(v));
            }
            MicGain => {
                let v = target.unwrap_or(state.tx.mic_gain + delta).clamp(0.0, 1.0);
                state.tx.mic_gain = v;
                cmds.push(Command::SetMicGain(v));
            }
            DigiAudioFreq => {
                // The engine owns the current offset, so a relative nudge needs
                // a reference: use the passband centre when there is none.
                let base = target.unwrap_or_else(|| {
                    (state.rx[0].filter_lo + state.rx[0].filter_hi) / 2.0 + delta
                });
                cmds.push(Command::SetDigiAudioFreq(base.clamp(100.0, 3500.0)));
            }
            SpectrumZoom => {
                let span = ui.view.span();
                if span > 0.0 {
                    let centre = (ui.view.view_lo_hz + ui.view.view_hi_hz) / 2.0;
                    let factor = (1.0 - delta as f64).clamp(0.05, 20.0);
                    let half = (span * factor / 2.0).max(50.0);
                    ui.view.view_lo_hz = centre - half;
                    ui.view.view_hi_hz = centre + half;
                    ui.view.clamp_to(ui.zoom_out.0, ui.zoom_out.1);
                }
            }
            SpectrumPan => {
                let shift = ui.view.span() * delta as f64;
                ui.view.view_lo_hz += shift;
                ui.view.view_hi_hz += shift;
                ui.view.clamp_to(ui.zoom_out.0, ui.zoom_out.1);
            }
            SpectrumFloorDb => {
                let v = target.unwrap_or(ui.view.db_floor + delta);
                ui.view.db_floor = v.clamp(-160.0, ui.view.db_ceil - 5.0);
            }
            SpectrumCeilDb => {
                let v = target.unwrap_or(ui.view.db_ceil + delta);
                ui.view.db_ceil = v.clamp(ui.view.db_floor + 5.0, 40.0);
            }
            _ => {}
        }
        return;
    }

    // ── Momentary / toggle ──────────────────────────────────────────────────
    // `Momentary` mirrors the physical control; `Toggle` flips on press and
    // ignores release. Everything else fires once, on press.
    let on = match (button, input) {
        (ButtonMode::Momentary, ActionInput::Press) => true,
        (ButtonMode::Momentary, ActionInput::Release) => false,
        (ButtonMode::Toggle, ActionInput::Press) => true,
        _ => return,
    };
    let latching = matches!(act, Ptt | TuneCarrier);
    if !latching && !on {
        // A one-shot action fires on press only; its release is a no-op.
        return;
    }

    let rx = RxId::Main;
    match act {
        Ptt => {
            let want = if button == ButtonMode::Toggle { !state.tx.ptt } else { on };
            cmds.push(Command::SetPtt(want));
        }
        TuneCarrier => {
            let want = if button == ButtonMode::Toggle { !state.tx.tune } else { on };
            cmds.push(Command::SetTune(want));
        }
        Mute => cmds.push(Command::SetMute { rx, muted: !state.rx[0].muted }),
        NoiseBlanker => cmds.push(Command::SetNoiseBlanker(!state.noise_blanker)),
        NoiseReductionCycle => {
            cmds.push(Command::SetNoiseReduction { rx, level: state.rx[0].noise_reduction.next() })
        }
        AutoNotch => cmds.push(Command::SetAutoNotch { rx, on: !state.rx[0].auto_notch }),
        // Only where the mode has it: in every other mode the chip is not
        // drawn, and a binding that toggled a hidden setting would change what
        // CW came back to without saying so — the same rule the AGC follows
        // just below.
        Binaural => {
            if state.rx[0].mode.binaural_audio() {
                cmds.push(Command::SetBinaural { rx, on: !state.rx[0].binaural });
            }
        }
        AgcCycle => {
            // In FM the chain bypasses the AGC and the chip is hidden; cycling
            // here would invisibly change what the next mode comes back to.
            if state.rx[0].mode.audio_agc() {
                let all = sdroxide_types::AgcMode::ALL;
                let i = all.iter().position(|a| *a == state.rx[0].agc).unwrap_or(0);
                cmds.push(Command::SetAgc { rx, agc: all[(i + 1) % all.len()] });
            }
        }
        SubRx => cmds.push(Command::SetSubRx(!state.sub_rx_enabled)),
        Split => cmds.push(Command::SetSplit(!state.split)),
        RitEnable => cmds.push(Command::SetRit { enabled: !state.rit.enabled, hz: state.rit.hz }),
        XitEnable => cmds.push(Command::SetXit { enabled: !state.xit.enabled, hz: state.xit.hz }),
        RitClear => cmds.push(Command::SetRit { enabled: state.rit.enabled, hz: 0 }),
        XitClear => cmds.push(Command::SetXit { enabled: state.xit.enabled, hz: 0 }),
        VfoSelect(v) => cmds.push(Command::SelectVfo(v)),
        VfoToggle => cmds.push(Command::SelectVfo(match state.active_vfo {
            Vfo::A => Vfo::B,
            Vfo::B => Vfo::A,
        })),
        SwapVfos => cmds.push(Command::SwapVfos),
        CopyAtoB => cmds.push(Command::CopyAtoB),
        BandUp | BandDown => {
            if let Some(b) = step_band(state.band, act == BandUp) {
                cmds.push(Command::SetBand(b));
            }
        }
        BandSelect(b) => cmds.push(Command::SetBand(b)),
        ModeNext | ModePrev => {
            let all = sdroxide_types::Mode::ALL;
            let i = all.iter().position(|m| *m == state.rx[0].mode).unwrap_or(0);
            let n = all.len();
            let i = if act == ModeNext { (i + 1) % n } else { (i + n - 1) % n };
            cmds.push(Command::SetMode { rx, mode: all[i] });
        }
        ModeSelect(mode) => cmds.push(Command::SetMode { rx, mode }),
        MemoryRecall(n) => cmds.push(Command::RecallMemory(n)),
        RecordToggle => cmds.push(Command::SetRecording(!state.recording)),
        AbortTx => cmds.push(Command::DigiAbortTx),
        // The engine decides whether this can transmit: an empty slot, a
        // digital mode other than RADE, or TUNE in progress all make it a
        // no-op there, which is where the keyer's state actually lives.
        VoicePlay(slot) => cmds.push(Command::VoicePlay(Some(slot))),
        VoiceStop => cmds.push(Command::VoicePlay(None)),
        // The engine decides this one too: outside NFM it says so rather than
        // transmitting, and a key-down from receive goes through every
        // transmit rail on the way.
        ToneBurst => cmds.push(Command::ToneBurst),
        FitSpan => ui.view.fit(state.center_hz, state.sample_rate),
        ZoomIn | ZoomOut => {
            let span = ui.view.span();
            if span > 0.0 {
                let centre = (ui.view.view_lo_hz + ui.view.view_hi_hz) / 2.0;
                let half = (span * if act == ZoomIn { 0.5 } else { 2.0 } / 2.0).max(50.0);
                ui.view.view_lo_hz = centre - half;
                ui.view.view_hi_hz = centre + half;
                ui.view.clamp_to(ui.zoom_out.0, ui.zoom_out.1);
            }
        }
        PeakHold => ui.view.peak_hold = !ui.view.peak_hold,
        // The two panadapter layers, switched independently — with both off
        // the panadapter is not drawn at all (see `app::frame`).
        SpectrumCollapse => {
            let on = ui.view.spectrum_visible();
            ui.view.set_spectrum_visible(!on);
        }
        WaterfallCollapse => {
            let on = ui.view.waterfall_visible();
            ui.view.set_waterfall_visible(!on);
        }
        WaterfallFlip => ui.view.waterfall_flip = !ui.view.waterfall_flip,
        ToggleHelp => *ui.help = !*ui.help,
        ToggleSettings => *ui.settings = !*ui.settings,
        ToggleLogbook => *ui.logbook = !*ui.logbook,
        ToggleSpots => *ui.spots = !*ui.spots,
        ToggleMemories => *ui.memories = !*ui.memories,
        ToggleVoice => *ui.voice = !*ui.voice,
        SpeakStatus | SpeakRepeat | SpeechSilence | SpeechToggle => ui.speech.push(act),
        _ => {}
    }
}

/// Next/previous amateur band, skipping the general-coverage pseudo-band and
/// any band the station's own band plan does not give this region — stepping
/// onto Region 1's 4 m in the Americas would be stepping out of band.
fn step_band(cur: sdroxide_types::Band, up: bool) -> Option<sdroxide_types::Band> {
    use sdroxide_types::Band;
    let ham: Vec<Band> = Band::ALL.iter().copied().filter(|b| b.edges().is_some()).collect();
    let i = ham.iter().position(|b| *b == cur)?;
    let n = ham.len();
    Some(if up { ham[(i + 1) % n] } else { ham[(i + n - 1) % n] })
}

/// Does this chord's modifier set match what is physically held?
///
/// `KeyChord::ctrl` means "the command modifier" — Ctrl on Windows/Linux, ⌘ on
/// macOS — so one saved binding behaves the same everywhere. The extra `ctrl`
/// term rejects macOS Ctrl (which is not ⌘) rather than letting it pass as an
/// unmodified press.
fn chord_matches(m: eframe::egui::Modifiers, c: &KeyChord) -> bool {
    m.alt == c.alt && m.shift == c.shift && m.command == c.ctrl && m.ctrl == (c.ctrl && !m.mac_cmd)
}

fn egui_button(b: MouseButton) -> eframe::egui::PointerButton {
    use eframe::egui::PointerButton;
    match b {
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Extra1 => PointerButton::Extra1,
        MouseButton::Extra2 => PointerButton::Extra2,
    }
}

/// What the MIDI LEARN button is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiLearn {
    /// Row in `cfg.midi.bindings` to write the captured control into.
    pub row: usize,
}

/// Client-side control-input state: the bindings, what is currently held, and
/// (on native) the live MIDI connection.
pub struct InputRuntime {
    pub cfg: InputSettings,
    held: Vec<Held>,
    accum: HashMap<Action, TickAccum>,
    /// Settings dialog: index of the key binding waiting to capture a chord.
    pub key_capture: Option<usize>,
    /// Settings dialog: the MIDI binding row waiting for a control to move.
    pub midi_learn: Option<MidiLearn>,
    /// Values seen since LEARN started, used to guess the encoder's encoding.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    learn_values: Vec<u8>,
    /// Last control that spoke, shown in the settings dialog so an operator can
    /// identify a knob before binding it.
    pub last_midi: Option<(sdroxide_types::MidiMsg, u8)>,
    #[cfg(not(target_arch = "wasm32"))]
    midi: Option<sdroxide_midi::MidiHandle>,
    /// The config the MIDI worker was last told about, so an unrelated edit
    /// (a step change, say) doesn't tear the connection down.
    #[cfg(not(target_arch = "wasm32"))]
    midi_sent: sdroxide_midi::MidiConfig,
}

impl InputRuntime {
    /// `ctx` is cloned into the MIDI wake closure: a knob tick is a user
    /// gesture, and without an immediate repaint it would wait out the app's
    /// 250 ms idle poll and feel broken.
    pub fn new(storage: Option<&dyn eframe::Storage>, ctx: &eframe::egui::Context) -> Self {
        let cfg = load_input_settings(storage);
        #[cfg(not(target_arch = "wasm32"))]
        let (midi, midi_sent) = {
            let want = midi_config(&cfg);
            let ctx = ctx.clone();
            (Some(sdroxide_midi::spawn(want.clone(), move || crate::repaint::animate(&ctx))), want)
        };
        #[cfg(target_arch = "wasm32")]
        let _ = ctx;
        InputRuntime {
            cfg,
            held: Vec::new(),
            accum: HashMap::new(),
            key_capture: None,
            midi_learn: None,
            learn_values: Vec::new(),
            last_midi: None,
            #[cfg(not(target_arch = "wasm32"))]
            midi,
            #[cfg(not(target_arch = "wasm32"))]
            midi_sent,
        }
    }

    pub fn persist(&self) {
        persist_input_settings(&self.cfg);
    }

    /// Throw away any queued MIDI events. A radio tab that is not focused
    /// does not act on the control surface, and its backlog must not fire as
    /// a burst of stale knob turns the moment it is focused.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn discard_midi(&mut self) {
        if let Some(midi) = self.midi.as_mut() {
            let _ = midi.poll();
        }
    }

    /// Effective step for `act` given `detents` ticks arriving now.
    fn scaled(&mut self, act: Action, tuning: &BindingTuning, now: f64, detents: f32) -> f32 {
        let rate = self.accum.entry(act).or_default().tick(now, detents);
        detents * tuning.effective_step(rate)
    }

    /// Whether anything momentary is currently asserted (used by the UI to show
    /// that an external control is holding a control down).
    pub fn any_held(&self) -> bool {
        !self.held.is_empty()
    }

    fn press(&mut self, src: HeldSource, action: Action, mode: ButtonMode, now: f64) {
        if mode == ButtonMode::Momentary && !self.held.iter().any(|h| h.src == src) {
            self.held.push(Held { src, action, mode, since: now });
        }
    }

    /// Release a held control, if it is held. Returns the action to de-assert.
    fn release(&mut self, src: HeldSource) -> Option<Action> {
        let i = self.held.iter().position(|h| h.src == src)?;
        Some(self.held.remove(i).action)
    }

    /// De-assert every held control. The only correct response to losing
    /// focus, losing the device, or shutting down: a stranded PTT is the worst
    /// failure this subsystem can produce.
    pub(crate) fn release_all(
        &mut self,
        state: &mut RadioState,
        ui: &mut UiSink<'_>,
        cmds: &mut Vec<Command>,
    ) {
        for h in std::mem::take(&mut self.held) {
            apply_action(h.action, ActionInput::Release, h.mode, state, ui, cmds);
        }
    }

    /// Keyboard and mouse-button dispatch for one frame.
    pub(crate) fn poll_pointer_and_keys(
        &mut self,
        ctx: &eframe::egui::Context,
        state: &mut RadioState,
        ui: &mut UiSink<'_>,
        cmds: &mut Vec<Command>,
    ) {
        use eframe::egui::{Event, Key};

        let now = ctx.input(|i| i.time);

        // Rebind capture swallows the next keypress rather than acting on it.
        if let Some(row) = self.key_capture {
            let captured = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                    _ => None,
                })
            });
            if let Some((key, m)) = captured {
                self.key_capture = None;
                if key != Key::Escape
                    && let Some(b) = self.cfg.keys.get_mut(row)
                {
                    b.chord = KeyChord {
                        key: key.name().to_string(),
                        ctrl: m.command,
                        shift: m.shift,
                        alt: m.alt,
                    };
                }
            }
            return;
        }

        let (focused, modifiers) = ctx.input(|i| (i.focused, i.modifiers));
        // A focused widget also counts as typing: without that, a Space bound
        // to PTT would key the rig every time the operator nudges a button.
        let typing = ctx.egui_wants_keyboard_input() || ctx.memory(|m| m.focused()).is_some();

        // ── Safety unkey, before the typing early-return ─────────────────────
        // A control that is held must be released when the key physically comes
        // up, when the window loses focus, when a text field steals the
        // keyboard, or when the hold timeout expires. Modifiers are
        // deliberately *not* re-checked: releasing Shift before Space must not
        // strand the transmitter.
        let timeout = self.cfg.ptt_hold_timeout_s;
        let stale: Vec<HeldSource> = {
            let keys = &self.cfg.keys;
            let mouse = &self.cfg.mouse_buttons;
            ctx.input(|i| {
                self.held
                    .iter()
                    .filter(|h| {
                        if !focused || typing {
                            return true;
                        }
                        if timeout > 0.0 && (now - h.since) as f32 > timeout {
                            return true;
                        }
                        match h.src {
                            HeldSource::Key(ix) => keys
                                .get(ix)
                                .and_then(|b| Key::from_name(&b.chord.key))
                                .map(|k| !i.key_down(k))
                                .unwrap_or(true),
                            HeldSource::Mouse(ix) => mouse
                                .get(ix)
                                .map(|b| !i.pointer.button_down(egui_button(b.button)))
                                .unwrap_or(true),
                            // MIDI releases arrive as messages, not as polled
                            // state; only the global conditions above end them.
                            HeldSource::Midi(_) => false,
                        }
                    })
                    .map(|h| h.src)
                    .collect()
            })
        };
        for src in stale {
            if let Some(action) = self.release(src) {
                apply_action(action, ActionInput::Release, ButtonMode::Momentary, state, ui, cmds);
            }
        }

        if typing {
            return;
        }

        // ── Key bindings ────────────────────────────────────────────────────
        // Collected first so the `ctx.input` borrow is released before the
        // resolver runs (it may itself read view/context state).
        #[derive(Clone, Copy)]
        enum Fire {
            /// A continuous action moved by this many raw detents.
            Value(HeldSource, f32),
            Button(HeldSource, bool),
        }
        let mut fired: Vec<Fire> = Vec::new();
        ctx.input(|i| {
            for (ix, b) in self.cfg.keys.iter().enumerate() {
                if !b.enabled || b.chord.is_empty() || !chord_matches(modifiers, &b.chord) {
                    continue;
                }
                let Some(key) = Key::from_name(&b.chord.key) else { continue };
                let src = HeldSource::Key(ix);
                match b.action.kind() {
                    ActionKind::Continuous => {
                        // `num_presses` counts auto-repeat too, which is what
                        // gives a held arrow key its familiar run-on feel.
                        let n = i.num_presses(key);
                        if n > 0 {
                            fired.push(Fire::Value(src, b.value * n as f32));
                        }
                    }
                    ActionKind::Momentary => {
                        if i.key_pressed(key) {
                            fired.push(Fire::Button(src, true));
                        }
                        if b.button == ButtonMode::Momentary && i.key_released(key) {
                            fired.push(Fire::Button(src, false));
                        }
                    }
                }
            }
            for (ix, b) in self.cfg.mouse_buttons.iter().enumerate() {
                if !b.enabled {
                    continue;
                }
                let (src, btn) = (HeldSource::Mouse(ix), egui_button(b.button));
                if i.pointer.button_pressed(btn) {
                    fired.push(Fire::Button(src, true));
                }
                if b.button_mode == ButtonMode::Momentary && i.pointer.button_released(btn) {
                    fired.push(Fire::Button(src, false));
                }
            }
        });

        for f in fired {
            match f {
                Fire::Value(src, detents) => {
                    let HeldSource::Key(ix) = src else { continue };
                    let Some(b) = self.cfg.keys.get(ix).cloned() else { continue };
                    let d = self.scaled(b.action, &b.tuning, now, detents);
                    let input = ActionInput::Delta { d, step: b.tuning.step };
                    apply_action(b.action, input, b.button, state, ui, cmds);
                }
                Fire::Button(src, down) => {
                    let Some((action, mode)) = self.binding_button(src) else { continue };
                    if down {
                        self.press(src, action, mode, now);
                    } else if self.release(src).is_none() {
                        continue;
                    }
                    let sample = if down { ActionInput::Press } else { ActionInput::Release };
                    apply_action(action, sample, mode, state, ui, cmds);
                }
            }
        }
    }

    /// Drain the MIDI worker and apply everything it produced.
    ///
    /// Deltas are summed per binding before being resolved, so a fast spin
    /// still yields exactly one command per frame — the same rate drag-tuning
    /// already produces, which is what makes this safe over `--connect`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn poll_midi(
        &mut self,
        ctx: &eframe::egui::Context,
        state: &mut RadioState,
        ui: &mut UiSink<'_>,
        cmds: &mut Vec<Command>,
    ) {
        use sdroxide_types::RelativeMode;

        self.sync_midi();
        let Some(midi) = self.midi.as_mut() else { return };
        let events = midi.poll();
        if events.is_empty() {
            self.send_midi_feedback(state, ui);
            return;
        }
        let now = ctx.input(|i| i.time);

        // (binding index -> summed detents) and (binding index -> last fader
        // position). Buttons keep their arrival order.
        let mut deltas: Vec<(usize, f32)> = Vec::new();
        let mut absolutes: Vec<(usize, f32)> = Vec::new();
        let mut buttons: Vec<(usize, bool)> = Vec::new();

        for ev in events {
            let c = match ev {
                sdroxide_midi::MidiEvent::Control(c) => c,
                sdroxide_midi::MidiEvent::Connected(_) => continue,
                sdroxide_midi::MidiEvent::Disconnected => {
                    // A surface that vanished can't release what it was
                    // holding, so do it for it.
                    self.release_midi_holds(state, ui, cmds);
                    continue;
                }
            };
            self.last_midi = Some((c.msg, c.value));

            if let Some(learn) = self.midi_learn {
                self.learn_capture(learn, c);
                continue;
            }

            for (ix, b) in self.cfg.midi.bindings.iter().enumerate() {
                if !b.enabled || !b.msg.matches(&c.msg) {
                    continue;
                }
                match b.action.kind() {
                    ActionKind::Continuous => match b.relative.decode(c.value) {
                        Some(detents) => {
                            let d = detents as f32;
                            match deltas.iter_mut().find(|(i, _)| *i == ix) {
                                Some(e) => e.1 += d,
                                None => deltas.push((ix, d)),
                            }
                        }
                        None => {
                            let p = c.value as f32 / 127.0;
                            match absolutes.iter_mut().find(|(i, _)| *i == ix) {
                                Some(e) => e.1 = p,
                                None => absolutes.push((ix, p)),
                            }
                        }
                    },
                    ActionKind::Momentary => {
                        // A relative encoder bound to a button action fires on
                        // any movement; a real button uses its note/CC value.
                        let down = if b.relative == RelativeMode::Absolute {
                            !c.off && c.value > 0
                        } else {
                            true
                        };
                        buttons.push((ix, down));
                    }
                }
            }
        }

        for (ix, down) in buttons {
            let Some(b) = self.cfg.midi.bindings.get(ix) else { continue };
            let (action, mode) = (b.action, b.button_mode);
            let src = HeldSource::Midi(ix);
            if down {
                self.press(src, action, mode, now);
            } else if self.release(src).is_none() && mode == ButtonMode::Momentary {
                continue;
            }
            let sample = if down { ActionInput::Press } else { ActionInput::Release };
            apply_action(action, sample, mode, state, ui, cmds);
        }
        for (ix, detents) in deltas {
            let Some(b) = self.cfg.midi.bindings.get(ix).cloned() else { continue };
            let d = self.scaled(b.action, &b.tuning, now, detents);
            let input = ActionInput::Delta { d, step: b.tuning.step };
            apply_action(b.action, input, b.button_mode, state, ui, cmds);
        }
        for (ix, p) in absolutes {
            let Some(b) = self.cfg.midi.bindings.get(ix).cloned() else { continue };
            let p = if b.tuning.invert { 1.0 - p } else { p };
            apply_action(b.action, ActionInput::Absolute(p), b.button_mode, state, ui, cmds);
        }
        self.send_midi_feedback(state, ui);
    }

    /// Record a captured control into the LEARN target row.
    #[cfg(not(target_arch = "wasm32"))]
    fn learn_capture(&mut self, learn: MidiLearn, c: sdroxide_midi::MidiControl) {
        let Some(b) = self.cfg.midi.bindings.get_mut(learn.row) else {
            self.midi_learn = None;
            return;
        };
        if b.msg != c.msg {
            // A different control took over the capture: start its sample set
            // fresh rather than mixing two encoders' values.
            self.learn_values.clear();
            b.msg = c.msg;
        }
        self.learn_values.push(c.value);
        if b.action.kind() == ActionKind::Continuous {
            b.relative = sdroxide_types::RelativeMode::guess(&self.learn_values);
        }
        // Enough of a turn to have classified the encoding; a button needs one.
        let done = if b.action.kind() == ActionKind::Continuous {
            self.learn_values.len() >= 6
        } else {
            true
        };
        if done {
            self.midi_learn = None;
            self.learn_values.clear();
        }
    }

    /// Release everything a MIDI surface was holding.
    #[cfg(not(target_arch = "wasm32"))]
    fn release_midi_holds(
        &mut self,
        state: &mut RadioState,
        ui: &mut UiSink<'_>,
        cmds: &mut Vec<Command>,
    ) {
        let stale: Vec<HeldSource> = self
            .held
            .iter()
            .filter(|h| matches!(h.src, HeldSource::Midi(_)))
            .map(|h| h.src)
            .collect();
        for src in stale {
            if let Some(action) = self.release(src) {
                apply_action(action, ActionInput::Release, ButtonMode::Momentary, state, ui, cmds);
            }
        }
    }

    /// Push the current value of every feedback-enabled binding back to the
    /// controller, so an LED tracks PTT and a motor fader tracks the volume.
    #[cfg(not(target_arch = "wasm32"))]
    fn send_midi_feedback(&mut self, state: &RadioState, ui: &UiSink<'_>) {
        let Some(midi) = self.midi.as_ref() else { return };
        let msgs: Vec<sdroxide_midi::Feedback> = self
            .cfg
            .midi
            .bindings
            .iter()
            .filter(|b| b.enabled && b.feedback)
            .filter_map(|b| {
                indicator(b.action, state, ui.view)
                    .map(|value| sdroxide_midi::Feedback { msg: b.msg, value })
            })
            .collect();
        midi.feedback(&msgs);
    }

    /// Tell the worker about a port change. Called every frame; it only sends
    /// when the ports actually differ, so editing a step or an action never
    /// interrupts a live connection.
    #[cfg(not(target_arch = "wasm32"))]
    fn sync_midi(&mut self) {
        let want = midi_config(&self.cfg);
        if want != self.midi_sent {
            if let Some(m) = self.midi.as_ref() {
                m.set_config(want.clone());
            }
            self.midi_sent = want;
        }
    }

    /// Live MIDI connection state, for the settings dialog.
    pub fn midi_status(&self) -> MidiStatusView {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match self.midi.as_ref().map(|m| m.status()) {
                Some(s) => MidiStatusView {
                    supported: true,
                    connected: s.connected,
                    port: s.port,
                    out_connected: s.out_connected,
                    error: s.error,
                },
                None => MidiStatusView { supported: true, ..MidiStatusView::default() },
            }
        }
        #[cfg(target_arch = "wasm32")]
        MidiStatusView::default()
    }

    /// MIDI input ports available right now. Enumeration touches the host MIDI
    /// stack, so the settings dialog calls this on open, not per frame.
    pub fn midi_ports(&self) -> (Vec<MidiPort>, Vec<MidiPort>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let map =
                |v: Vec<sdroxide_midi::PortInfo>| v.into_iter().map(|p| (p.id, p.name)).collect();
            (map(sdroxide_midi::input_ports()), map(sdroxide_midi::output_ports()))
        }
        #[cfg(target_arch = "wasm32")]
        (Vec::new(), Vec::new())
    }

    /// The action and button mode behind a held source, if the binding is still
    /// there (the settings dialog can delete a row mid-press).
    fn binding_button(&self, src: HeldSource) -> Option<(Action, ButtonMode)> {
        match src {
            HeldSource::Key(ix) => self.cfg.keys.get(ix).map(|b| (b.action, b.button)),
            HeldSource::Mouse(ix) => {
                self.cfg.mouse_buttons.get(ix).map(|b| (b.action, b.button_mode))
            }
            HeldSource::Midi(ix) => {
                self.cfg.midi.bindings.get(ix).map(|b| (b.action, b.button_mode))
            }
        }
    }
}

/// A MIDI port as the settings dialog lists it: the stable id it reconnects
/// by, and the name it shows.
pub type MidiPort = (String, String);

/// MIDI connection state as the settings dialog needs it, without exposing the
/// native-only crate's types to the wasm build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MidiStatusView {
    /// False on wasm, where the browser client has no MIDI runtime.
    pub supported: bool,
    pub connected: bool,
    pub port: String,
    pub out_connected: bool,
    pub error: Option<String>,
}

/// The port selection, split out of the full settings blob so a binding edit
/// never looks like a port change.
#[cfg(not(target_arch = "wasm32"))]
fn midi_config(cfg: &InputSettings) -> sdroxide_midi::MidiConfig {
    sdroxide_midi::MidiConfig {
        enabled: cfg.midi.enabled,
        in_port_id: cfg.midi.in_port_id.clone(),
        in_port_name: cfg.midi.in_port_name.clone(),
        out_port_id: cfg.midi.out_port_id.clone(),
        out_port_name: cfg.midi.out_port_name.clone(),
    }
}

/// The 0..=127 value to echo back to a controller for this action, or `None`
/// where there is nothing meaningful to show.
#[cfg(not(target_arch = "wasm32"))]
fn indicator(act: Action, state: &RadioState, view: &ViewState) -> Option<u8> {
    use Action::*;
    let on = |b: bool| Some(if b { 127u8 } else { 0 });
    let frac = |v: f32| Some((v.clamp(0.0, 1.0) * 127.0).round() as u8);
    match act {
        Ptt => on(state.tx.ptt),
        TuneCarrier => on(state.tx.tune),
        Mute => on(state.rx[0].muted),
        NoiseBlanker => on(state.noise_blanker),
        NoiseReductionCycle => on(state.rx[0].noise_reduction.is_on()),
        AutoNotch => on(state.rx[0].auto_notch),
        Binaural => on(state.rx[0].binaural),
        SubRx => on(state.sub_rx_enabled),
        Split => on(state.split),
        RitEnable => on(state.rit.enabled),
        XitEnable => on(state.xit.enabled),
        RecordToggle => on(state.recording),
        PeakHold => on(view.peak_hold),
        SpectrumCollapse => on(!view.spectrum_visible()),
        WaterfallCollapse => on(!view.waterfall_visible()),
        WaterfallFlip => on(view.waterfall_flip),
        VfoSelect(v) => on(state.active_vfo == v),
        Volume => frac(state.rx[0].volume),
        SubVolume => frac(state.rx[1].volume),
        TxDrive => frac(state.tx.drive),
        TuneDrive => frac(state.tx.tune_drive),
        MicGain => frac(state.tx.mic_gain),
        _ => None,
    }
}

// ── Persistence (native: config-dir JSON; wasm: eframe storage) ──────────────
#[cfg(not(target_arch = "wasm32"))]
fn load_input_settings(_storage: Option<&dyn eframe::Storage>) -> InputSettings {
    sdroxide_config::load_input_settings()
}
#[cfg(target_arch = "wasm32")]
fn load_input_settings(storage: Option<&dyn eframe::Storage>) -> InputSettings {
    storage.and_then(|s| eframe::get_value(s, "input")).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_input_settings(cfg: &InputSettings) {
    if let Err(e) = sdroxide_config::save_input_settings(cfg) {
        eprintln!("failed to save control-input settings: {e}");
    }
}
#[cfg(target_arch = "wasm32")]
fn persist_input_settings(_cfg: &InputSettings) {
    // Written by eframe's periodic `save()` into localStorage.
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::{Band, Mode};

    fn sink<'a>(
        view: &'a mut ViewState,
        flags: &'a mut [bool; 6],
        speech: &'a mut Vec<Action>,
    ) -> UiSink<'a> {
        let (help, rest) = flags.split_at_mut(1);
        let (settings, rest) = rest.split_at_mut(1);
        let (logbook, rest) = rest.split_at_mut(1);
        let (spots, rest) = rest.split_at_mut(1);
        let (memories, voice) = rest.split_at_mut(1);
        UiSink {
            view,
            help: &mut help[0],
            settings: &mut settings[0],
            logbook: &mut logbook[0],
            spots: &mut spots[0],
            memories: &mut memories[0],
            voice: &mut voice[0],
            speech,
            rig_squelch: false,
            // The clamp these tests exercise is a no-op at zero span, which is
            // what a default `RadioState` carries — the same thing the caller
            // used to read straight off it. A test about the clamp itself sets
            // this after building the sink.
            zoom_out: (0.0, 0.0),
        }
    }

    /// A relative tune must move the local state immediately (the optimistic
    /// echo) *and* queue the absolute command the engine expects.
    #[test]
    fn tune_echoes_locally_and_commands_absolutely() {
        let mut state =
            RadioState { vfo_a_hz: 14_074_000.0, active_vfo: Vfo::A, ..RadioState::default() };
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Tune,
            ActionInput::Delta { d: 300.0, step: 100.0 },
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(state.vfo_a_hz, 14_074_300.0);
        assert_eq!(cmds, vec![Command::SetVfo { vfo: Vfo::A, hz: 14_074_300.0 }]);
    }

    /// Issue #136: a step lands on the step *grid*. Panadapter dragging tunes
    /// continuously, so it leaves the dial a fraction of a step off; the next
    /// step has to put it back rather than carry the offset along forever.
    #[test]
    fn tuning_snaps_onto_the_step_grid() {
        let tune = |hz: f64, d: f32, step: f32| {
            let mut state =
                RadioState { vfo_a_hz: hz, active_vfo: Vfo::A, ..RadioState::default() };
            let mut view = ViewState::default();
            let mut flags = [false; 6];
            let mut speech_acts = Vec::new();
            let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
            let mut cmds = Vec::new();
            apply_action(
                Action::Tune,
                ActionInput::Delta { d, step },
                ButtonMode::Momentary,
                &mut state,
                &mut ui,
                &mut cmds,
            );
            state.vfo_a_hz
        };
        // Left behind 37.4 Hz off the grid by a drag, and stepped either way.
        assert_eq!(tune(14_074_037.4, 100.0, 100.0), 14_074_100.0);
        assert_eq!(tune(14_074_037.4, -100.0, 100.0), 14_073_900.0);
        // A finer binding keeps its own grid, and a coarser one its own.
        assert_eq!(tune(14_074_037.4, 10.0, 10.0), 14_074_050.0);
        assert_eq!(tune(14_074_037.4, 1_000.0, 1_000.0), 14_075_000.0);
        // Acceleration moves by whole steps rather than off the grid.
        assert_eq!(tune(14_074_000.0, 137.0, 100.0), 14_074_100.0);
        assert_eq!(tune(14_074_000.0, 262.0, 100.0), 14_074_300.0);
        // A binding set to move by less than its step tunes by that instead,
        // on the grid its own move makes.
        assert_eq!(tune(14_074_037.4, 50.0, 100.0), 14_074_100.0);
        // No step at all is the old free-running behaviour.
        assert_eq!(tune(14_074_037.4, 100.0, 0.0), 14_074_137.4);
    }

    #[test]
    fn tuning_never_goes_negative() {
        let mut state = RadioState { vfo_a_hz: 100.0, ..RadioState::default() };
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Tune,
            ActionInput::Delta { d: -5_000.0, step: 100.0 },
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(state.vfo_a_hz, 0.0);
    }

    #[test]
    fn momentary_ptt_follows_the_button() {
        let mut state = RadioState::default();
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Ptt,
            ActionInput::Press,
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        apply_action(
            Action::Ptt,
            ActionInput::Release,
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(cmds, vec![Command::SetPtt(true), Command::SetPtt(false)]);
    }

    /// A toggle PTT flips the *engine's* reported state, never a local latch,
    /// so the on-screen chip and the external control can never disagree.
    #[test]
    fn toggle_ptt_flips_reported_state() {
        let mut state = RadioState::default();
        state.tx.ptt = true;

        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Ptt,
            ActionInput::Press,
            ButtonMode::Toggle,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(cmds, vec![Command::SetPtt(false)]);
        // Release on a toggle does nothing.
        cmds.clear();
        apply_action(
            Action::Ptt,
            ActionInput::Release,
            ButtonMode::Toggle,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert!(cmds.is_empty());
    }

    /// One-shots must not fire twice for one physical press.
    #[test]
    fn one_shot_actions_ignore_release() {
        let mut state = RadioState::default();
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::SwapVfos,
            ActionInput::Press,
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        apply_action(
            Action::SwapVfos,
            ActionInput::Release,
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(cmds, vec![Command::SwapVfos]);
    }

    #[test]
    fn absolute_fader_maps_into_range() {
        let mut state = RadioState::default();
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Volume,
            ActionInput::Absolute(0.25),
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert_eq!(cmds, vec![Command::SetVolume { rx: RxId::Main, v: 0.25 }]);
        assert_eq!(state.rx[0].volume, 0.25);
    }

    /// Tuning has no meaningful absolute position; a fader bound to it must be
    /// ignored rather than jumping the VFO to a fraction of nothing.
    #[test]
    fn absolute_on_tune_is_ignored() {
        let mut state = RadioState::default();
        let before = state.vfo_a_hz;
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        apply_action(
            Action::Tune,
            ActionInput::Absolute(0.5),
            ButtonMode::Momentary,
            &mut state,
            &mut ui,
            &mut cmds,
        );
        assert!(cmds.is_empty());
        assert_eq!(state.vfo_a_hz, before);
    }

    #[test]
    fn filter_width_stays_inside_the_mode_limits() {
        let mut state = RadioState::default();
        state.rx[0] = sdroxide_types::RxState::with_mode(Mode::Usb);

        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut speech_acts = Vec::new();
        let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
        let mut cmds = Vec::new();
        // Squeeze far past zero width.
        for _ in 0..200 {
            apply_action(
                Action::FilterWidth,
                ActionInput::Delta { d: -500.0, step: 50.0 },
                ButtonMode::Momentary,
                &mut state,
                &mut ui,
                &mut cmds,
            );
        }
        assert!(state.rx[0].filter_hi - state.rx[0].filter_lo >= MIN_FILTER_HZ - 0.001);
    }

    #[test]
    fn band_stepping_skips_general_coverage() {
        assert_eq!(step_band(Band::M20, true), Some(Band::M17));
        assert_eq!(step_band(Band::M20, false), Some(Band::M30));
        // 4 m sits between 6 m and 2 m, in the region that has it — which is
        // Region 1, the default these tests run under.
        assert_eq!(step_band(Band::M6, true), Some(Band::M4));
        assert_eq!(step_band(Band::M4, true), Some(Band::M2));
        // 1.25 m and 33 cm are Region 2's alone, so in Region 1 the step from
        // 2 m goes straight to 70 cm and from there to 23 cm.
        assert_eq!(step_band(Band::M2, true), Some(Band::M70));
        assert_eq!(step_band(Band::M70, true), Some(Band::Cm23));
        assert_eq!(step_band(Band::Cm23, false), Some(Band::M70));
        // Wraps within the ham bands only, from the highest to the lowest —
        // 3 cm being the highest since the IC-905 got its own 10 GHz band
        // (issue #326).
        assert_eq!(step_band(Band::Cm6, true), Some(Band::Cm3));
        assert_eq!(step_band(Band::Cm3, true), Some(Band::M160));
        assert_eq!(step_band(Band::Cm3, false), Some(Band::Cm6));
        assert_eq!(step_band(Band::Gen, true), None);
    }

    #[test]
    fn ui_only_actions_emit_no_commands() {
        let mut state = RadioState::default();
        let mut view = ViewState::default();
        let mut flags = [false; 6];
        let mut cmds = Vec::new();
        {
            let mut speech_acts = Vec::new();
            let mut ui = sink(&mut view, &mut flags, &mut speech_acts);
            apply_action(
                Action::FitSpan,
                ActionInput::Press,
                ButtonMode::Toggle,
                &mut state,
                &mut ui,
                &mut cmds,
            );
            apply_action(
                Action::ToggleHelp,
                ActionInput::Press,
                ButtonMode::Toggle,
                &mut state,
                &mut ui,
                &mut cmds,
            );
        }
        assert!(cmds.is_empty());
        assert!(flags[0], "help window should have toggled open");
        assert!(view.span() > 0.0, "fit should have set a span");
    }

    #[test]
    fn acceleration_resets_after_an_idle_gap() {
        let mut a = TickAccum::default();
        assert_eq!(a.tick(1.0, 1.0), 0.0);
        let fast = a.tick(1.01, 1.0);
        assert!(fast > 0.0);
        // A long pause means a new grab, not a continuation.
        assert_eq!(a.tick(5.0, 1.0), 0.0);
    }
}
