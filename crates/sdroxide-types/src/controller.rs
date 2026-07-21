use crate::{
    Command, Decode, DeviceCaps, DigiStatus, MemoryChannel, Meters, QsoRecord, RadioState,
    SkimmerSpot, SpectrumFrame,
};

/// Events flowing engine → UI.
#[derive(Debug, Clone, PartialEq)]
pub enum RadioEvent {
    Capabilities(DeviceCaps),
    /// Full state snapshot on any change (latest-wins).
    State(RadioState),
    Spectrum(SpectrumFrame),
    Meters(Meters),
    Memories(Vec<MemoryChannel>),
    ConnectionLost(String),
    /// FT8/FT4 decodes from one receive slot.
    Ft8Decodes(Vec<Decode>),
    /// FT8/FT4 engine status change.
    Ft8Status(DigiStatus),
    /// A completed QSO, appended to the session log.
    Ft8QsoLogged(QsoRecord),
    /// Latest set of skimmer spots (CW etc.).
    SkimmerSpots(Vec<SkimmerSpot>),
}

/// Snapshot of the frontend's switchable sound devices (native clients).
/// `selected_*` of `None` means "system default".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioDevices {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
    pub selected_output: Option<String>,
    pub selected_input: Option<String>,
}

/// The seam that lets the same UI run against an in-process radio engine
/// (native GUI) or a WebSocket session (WASM remote client).
pub trait RadioController {
    fn send(&mut self, cmd: Command);

    /// Non-blocking; the UI drains this each frame until `None`.
    fn poll_event(&mut self) -> Option<RadioEvent>;

    /// Microphone audio, 48 kHz mono. The local implementation is a no-op
    /// because cpal feeds the engine directly.
    fn send_mic(&mut self, _pcm_48k_mono: &[f32]) {}

    /// Whether the UI should schedule a repaint soon (fresh data pending).
    fn wants_repaint_soon(&self) -> bool {
        false
    }

    /// The frontend's switchable audio devices, or `None` when the platform
    /// has none to offer (e.g. the browser client, where the browser owns
    /// device routing). Enumeration may be slow — call on demand, not per
    /// frame.
    fn audio_devices(&self) -> Option<AudioDevices> {
        None
    }

    /// Switch the audio output (`output = true`) or input device by name;
    /// `None` selects the system default. No-op on platforms without
    /// switchable audio.
    fn set_audio_device(&mut self, output: bool, name: Option<String>) {
        let _ = (output, name);
    }
}
