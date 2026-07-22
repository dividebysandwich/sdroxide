use crate::{
    Command, Decode, DeviceCaps, DigiStatus, MemoryChannel, Meters, QsoRecord, RadioState,
    SkimmerSpot, SpectrumFrame, SstvMode, SstvStatus,
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
    /// A non-fatal, persistent status/warning for the operator (e.g. the radio
    /// audio input was unavailable or a mono card was selected for IQ). `None`
    /// clears it. Native-local only — not forwarded to remote clients.
    Notice(Option<String>),
    /// FT8/FT4 decodes from one receive slot.
    Ft8Decodes(Vec<Decode>),
    /// FT8/FT4 engine status change.
    Ft8Status(DigiStatus),
    /// A completed QSO, appended to the session log.
    Ft8QsoLogged(QsoRecord),
    /// Latest set of skimmer spots (CW etc.).
    SkimmerSpots(Vec<SkimmerSpot>),
    /// SSTV: one freshly decoded scanline `rgb` (3·width bytes) at row `y` of the
    /// image identified by `image_id`. Paints progressively.
    SstvLine { image_id: u32, y: u16, rgb: Vec<u8> },
    /// SSTV: a completed image (PNG bytes) plus its identity and size.
    SstvImage { image_id: u32, mode: SstvMode, w: u16, h: u16, png: Vec<u8> },
    /// SSTV: engine status change (tx/rx active, detected mode, progress).
    SstvStatus(SstvStatus),
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

    /// Whether this build can drive SoapySDR devices (compiled with the `soapy`
    /// feature). The settings UI offers the SoapySDR interface only when true.
    /// Default false (remote clients don't own the server's hardware).
    fn soapy_supported(&self) -> bool {
        false
    }

    /// Serial ports available for CAT control (native local client only).
    fn serial_ports(&self) -> Vec<String> {
        Vec::new()
    }

    /// Scan the LAN for OpenHPSDR devices (native local client only). Blocking —
    /// the settings UI calls this on demand (a "Discover" button), not per frame.
    /// Default empty: the browser/remote client can't scan the server's network.
    fn discover_hpsdr(&self) -> Vec<crate::HpsdrDevice> {
        Vec::new()
    }

    /// Test a TCI server connection at `address` (`host:port`). Blocking — the
    /// settings UI calls this on demand (a "Test connection" button). Returns a
    /// success summary or an error message. Default: unsupported (remote client).
    fn test_tci(&self, _address: &str) -> Result<String, String> {
        Err("not supported on this client".into())
    }

    /// The persisted radio-backend config (SoapySDR vs CAT), or `None` when the
    /// client can't own it (the browser remote client).
    fn radio_config(&self) -> Option<crate::RadioConfig> {
        None
    }

    /// Persist an updated radio-backend config. Most fields only take effect on
    /// restart (the source/engine is rebuilt at startup). No-op where
    /// unsupported.
    fn set_radio_config(&mut self, cfg: crate::RadioConfig) {
        let _ = cfg;
    }
}
