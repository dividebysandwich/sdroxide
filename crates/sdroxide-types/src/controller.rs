use crate::{
    Command, Decode, DeviceCaps, DigiStatus, MemoryChannel, Meters, QsoRecord, RadioState,
    SpectrumFrame,
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
}
