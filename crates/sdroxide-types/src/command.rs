use serde::{Deserialize, Serialize};

use crate::{
    AgcMode, Band, DigiConfig, Direction, Mode, NrLevel, RxId, SpectrumConfig, SstvMode, Vfo,
};

/// The single control vocabulary. The GUI, the WebSocket protocol, and the
/// future TCI server all speak `Command`; the DSP engine is its only consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // VFO / tuning
    SetVfo { vfo: Vfo, hz: f64 },
    SelectVfo(Vfo),
    SwapVfos,
    CopyAtoB,
    SetSplit(bool),
    SetCenter(f64),
    SetSampleRate(f64),
    /// Engine applies band-stack recall (or the band default entry).
    SetBand(Band),

    // Receiver settings
    SetMode { rx: RxId, mode: Mode },
    SetFilter { rx: RxId, lo: f32, hi: f32 },
    SetAgc { rx: RxId, agc: AgcMode },
    SetAgcMaxGain { rx: RxId, db: f32 },
    SetVolume { rx: RxId, v: f32 },
    SetMute { rx: RxId, muted: bool },
    /// Squelch threshold in dBFS ([`crate::SQUELCH_OPEN_DB`] = open).
    SetSquelch { rx: RxId, db: f32 },
    SetNoiseBlanker(bool),
    /// Spectral audio noise-reduction intensity for a receiver.
    SetNoiseReduction { rx: RxId, level: NrLevel },
    /// Adaptive auto-notch (constant-tone canceller) for a receiver.
    SetAutoNotch { rx: RxId, on: bool },
    SetSubRx(bool),
    SetRit { enabled: bool, hz: i32 },
    SetXit { enabled: bool, hz: i32 },

    // Transmit
    SetPtt(bool),
    SetTune(bool),
    SetTxDrive(f32),
    SetTuneDrive(f32),
    SetMicGain(f32),

    // Hardware
    SetGain { dir: Direction, element: String, db: f64 },
    SetAntenna { dir: Direction, name: String },

    // Memories
    StoreMemory { name: String },
    RecallMemory(u32),
    DeleteMemory(u32),

    // Display
    SetSpectrumCfg(SpectrumConfig),

    // Digital modes (FT8/FT4)
    SetDigiConfig(DigiConfig),
    /// Set our transmit tone offset within the passband (Hz).
    SetDigiAudioFreq(f32),
    /// Start calling CQ.
    DigiCallCq,
    /// Begin a QSO with a decoded station.
    DigiStartQso { from: String, grid: Option<String>, snr: i16, audio_hz: f32 },
    /// Gracefully stop the QSO sequence (finish the current burst, then idle).
    DigiStopQso,
    /// Abort any in-progress transmission immediately.
    DigiAbortTx,
    /// Continuous keyboard modes (PSK/RTTY): set the full outgoing text buffer.
    /// The engine keeps already-sent characters and streams the rest.
    DigiTxText(String),
    /// Continuous keyboard modes: enter (true) or leave (false) transmit.
    DigiTxActive(bool),
    /// SSTV: select the mode (also sizes the TX image). `None` = Auto — the RX
    /// auto-detects the mode and TX defaults to Martin 1.
    SstvSetMode(Option<SstvMode>),
    /// SSTV: transmit a composed image (PNG bytes) in the given mode. Keying
    /// starts immediately; `DigiAbortTx` stops it.
    SstvTx { mode: SstvMode, png: Vec<u8> },
    /// FSQ image: transmit a picture (PNG bytes; the engine grayscales/scales it).
    DigiImageTx { png: Vec<u8> },

    // Skimmers
    /// Turn the (CW) skimmer on/off.
    SetSkimmerEnabled(bool),
}
