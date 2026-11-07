//! The client's speech runtime: an [`Announcer`] plus, on native, the
//! synthesizer behind it.
//!
//! All the interesting logic lives in `sdroxide-speech` — this is the seam that
//! feeds it the events the UI already drains, and the one place that knows how
//! to build a real sink for the current platform. On wasm the announcer still
//! runs, against a null sink; that keeps a single code path here and leaves a
//! browser backend as a matter of implementing one trait.

use crate::i18n::tr;
use sdroxide_speech::announce::Announcer;
use sdroxide_types::SpeechSettings;

/// What the settings dialog shows about the speech backend.
///
/// The browser build only ever reports `Idle` or `Failed` — there is no
/// synthesizer there to be loading or ready.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub enum SpeechStatus {
    /// No backend has been asked for — speech is switched off.
    Idle,
    /// Reading the voice model and the dictionary.
    Loading,
    Ready(String),
    Failed(String),
}

impl SpeechStatus {
    /// A line for the settings dialog, or `None` when there is nothing to say.
    pub fn note(&self) -> Option<String> {
        match self {
            SpeechStatus::Idle => None,
            SpeechStatus::Loading => Some(tr!("speech-loading-voice")),
            SpeechStatus::Ready(v) => Some(tr!("speech-voice-2", voice = v)),
            SpeechStatus::Failed(e) => {
                Some(tr!("speech-speech-unavailable", error = e.to_string()))
            }
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, SpeechStatus::Failed(_))
    }
}

/// Owns the announcer and whatever is speaking for it.
pub struct SpeechRuntime {
    pub announcer: Announcer,
    /// The settings the current backend was built from. A change to any of
    /// these needs the worker restarted; rate and volume do not.
    backend_key: BackendKey,
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<sdroxide_speech::synth::SpeechHandle>,
}

/// The parts of the settings that decide *which* backend to build.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BackendKey {
    enabled: bool,
    voice: String,
    device: Option<String>,
}

impl BackendKey {
    fn of(cfg: &SpeechSettings) -> Self {
        BackendKey { enabled: cfg.enabled, voice: cfg.voice.clone(), device: cfg.device.clone() }
    }
}

impl SpeechRuntime {
    pub fn new(cfg: SpeechSettings) -> Self {
        let key = BackendKey::of(&cfg);
        let mut rt = SpeechRuntime {
            announcer: Announcer::new(cfg),
            // Deliberately a mismatch, so `sync_backend` builds the backend on
            // the first call rather than duplicating the construction here.
            backend_key: BackendKey { enabled: !key.enabled, ..key },
            #[cfg(not(target_arch = "wasm32"))]
            handle: None,
        };
        rt.sync_backend();
        rt
    }

    pub fn settings(&self) -> &SpeechSettings {
        self.announcer.settings()
    }

    /// Adopt edited settings, rebuilding the backend only if it has to be.
    pub fn set_settings(&mut self, cfg: SpeechSettings) {
        self.announcer.set_settings(cfg);
        self.sync_backend();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(h) = self.handle.as_ref() {
            let cfg = self.announcer.settings();
            h.set_levels(cfg.rate(), cfg.volume());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_backend(&mut self) {
        let cfg = self.announcer.settings().clone();
        let key = BackendKey::of(&cfg);
        if key == self.backend_key {
            return;
        }
        self.backend_key = key.clone();

        if !key.enabled {
            self.handle = None;
            self.announcer.set_sink(Box::new(sdroxide_speech::NullSink::default()));
            return;
        }

        // Operator-supplied voices first, so a hand-picked one wins over the
        // shipped one.
        let user_dir = sdroxide_config::speech_voice_dir().ok();
        let dirs = sdroxide_speech::synth::voice_dirs(user_dir);
        let engine = sdroxide_speech::synth::SpeechEngine::start(
            dirs,
            key.voice,
            key.device,
            cfg.rate(),
            cfg.volume(),
        );
        self.handle = Some(engine.handle());
        self.announcer.set_sink(Box::new(engine));
    }

    #[cfg(target_arch = "wasm32")]
    fn sync_backend(&mut self) {
        // No synthesizer in the browser yet: the announcer runs and decides
        // what would be said, and a null sink discards it. Implementing
        // `SpeechSink` over the Web Speech API is all that stands between this
        // and a talking browser client.
        self.backend_key = BackendKey::of(self.announcer.settings());
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn status(&self) -> SpeechStatus {
        use sdroxide_speech::synth::SpeechStatus as S;
        match self.handle.as_ref().map(|h| h.status()) {
            None => SpeechStatus::Idle,
            Some(S::Loading) => SpeechStatus::Loading,
            Some(S::Ready(v)) => SpeechStatus::Ready(v),
            Some(S::Failed(e)) => SpeechStatus::Failed(e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn status(&self) -> SpeechStatus {
        if self.announcer.settings().enabled {
            SpeechStatus::Failed(tr!("speech-browser-client-cannot-speak-yet").into())
        } else {
            SpeechStatus::Idle
        }
    }

    /// Voices available to choose from.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn voices() -> Vec<String> {
        let user_dir = sdroxide_config::speech_voice_dir().ok();
        sdroxide_speech::synth::list_voices(&sdroxide_speech::synth::voice_dirs(user_dir))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn voices() -> Vec<String> {
        Vec::new()
    }
}
