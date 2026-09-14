//! In-process [`RadioController`]: wraps the engine's channel endpoints, and
//! owns the cpal stream handles so audio devices can be swapped at runtime
//! (the engine only ever holds the ring endpoints).

use sdroxide_radio::crossbeam_channel::{Receiver, Sender};
use sdroxide_radio::{AudioParams, EngineHandles, EngineSwap, MicParams, triple_buffer};
use sdroxide_types::{
    AudioDevices, Command, RadioConfig, RadioController, RadioEvent, SpectrumFrame,
};
use tracing::{info, warn};

pub struct LocalController {
    cmd_tx: Sender<Command>,
    event_rx: Receiver<RadioEvent>,
    spectrum: triple_buffer::Output<SpectrumFrame>,
    wide_spectrum: triple_buffer::Output<SpectrumFrame>,
    swap_tx: Sender<EngineSwap>,
    /// The engine thread, joined in [`RadioController::shutdown`] so device
    /// teardown (SoapySDR/libusb) can't race the C libraries' exit handlers.
    /// `None` after shutdown, or when the frontend chose to join it itself.
    thread: Option<std::thread::JoinHandle<()>>,
    /// Live cpal streams (they must outlive their ring endpoints in the engine).
    audio_out: Option<sdroxide_audio::AudioOutput>,
    mic_in: Option<sdroxide_audio::AudioInput>,
    /// A microphone open still running. Opening a capture device can block for
    /// as long as the sound stack feels like it (see
    /// [`sdroxide_audio::PendingInput`]), which is time neither startup nor the
    /// UI thread has to spare, so the open happens on a thread of its own and
    /// [`LocalController::poll_pending_mic`] hands it to the engine when it
    /// lands. TX before then carries silence, exactly as it does with no
    /// microphone at all.
    pending_mic: Option<sdroxide_audio::PendingInput>,
    /// Currently selected device names; `None` = system default.
    out_name: Option<String>,
    in_name: Option<String>,
    /// Where this radio's `radio.json` lives — radio 0 is the legacy root.
    store: sdroxide_config::Store,
}

impl LocalController {
    pub fn new(
        mut handles: EngineHandles,
        audio_out: Option<sdroxide_audio::AudioOutput>,
        pending_mic: Option<sdroxide_audio::PendingInput>,
        out_name: Option<String>,
        in_name: Option<String>,
        store: sdroxide_config::Store,
    ) -> Self {
        LocalController {
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum: handles.spectrum_out,
            wide_spectrum: handles.wide_spectrum_out,
            swap_tx: handles.swap_tx,
            thread: handles.thread.take(),
            audio_out,
            mic_in: None,
            pending_mic,
            out_name,
            in_name,
            store,
        }
    }

    /// Give the engine the microphone as soon as its open finishes. Cheap
    /// enough to call every frame: one non-blocking channel poll.
    fn poll_pending_mic(&mut self) {
        let Some(result) = self.pending_mic.as_mut().and_then(|p| p.take()) else { return };
        self.pending_mic = None;
        match result {
            Ok((input, consumer)) => {
                let rate = input.sample_rate;
                self.mic_in = Some(input);
                let _ = self.swap_tx.send(EngineSwap::Input(Some(MicParams { consumer, rate })));
            }
            Err(e) => warn!("no microphone ({e}); TX carries silence"),
        }
    }

    fn persist_selection(&self) {
        let mut s = sdroxide_config::Settings::load();
        s.audio_output = self.out_name.clone();
        s.audio_input = self.in_name.clone();
        if let Err(e) = s.save() {
            warn!("saving audio device selection: {e}");
        }
    }
}

impl RadioController for LocalController {
    fn send(&mut self, cmd: Command) {
        // The SWR guard is a station setting, not a session one: an operator
        // who raises the limit for a dummy load expects it still raised
        // tomorrow. Persisted on the way past, the same way the audio device
        // selection is, and only for a LOCAL radio — a remote session's
        // config.toml is on the client machine, which is the wrong one.
        if let Command::SetSwrGuard { enabled, limit } = cmd {
            let mut s = sdroxide_config::Settings::load();
            s.swr_guard = enabled;
            s.swr_limit = limit;
            if let Err(e) = s.save() {
                warn!("saving SWR guard setting: {e}");
            }
        }
        let _ = self.cmd_tx.send(cmd);
    }

    fn poll_event(&mut self) -> Option<RadioEvent> {
        self.poll_pending_mic();
        if let Ok(ev) = self.event_rx.try_recv() {
            return Some(ev);
        }
        if self.spectrum.update() {
            let f = self.spectrum.output_buffer();
            if !f.bins.is_empty() {
                return Some(RadioEvent::Spectrum(f.clone()));
            }
        }
        if self.wide_spectrum.update() {
            let f = self.wide_spectrum.output_buffer();
            if !f.bins.is_empty() {
                return Some(RadioEvent::WideSpectrum(f.clone()));
            }
        }
        None
    }

    fn wants_repaint_soon(&self) -> bool {
        !self.event_rx.is_empty() || self.spectrum.updated() || self.wide_spectrum.updated()
    }

    fn audio_devices(&self) -> Option<AudioDevices> {
        Some(AudioDevices {
            outputs: sdroxide_audio::output_device_names(),
            inputs: sdroxide_audio::input_device_names(),
            selected_output: self.out_name.clone(),
            selected_input: self.in_name.clone(),
        })
    }

    fn set_audio_device(&mut self, output: bool, name: Option<String>) {
        if output {
            // Drop the old stream first so an exclusive device is released.
            self.audio_out = None;
            match sdroxide_audio::start_output(name.as_deref(), 48_000) {
                Ok((out, producer)) => {
                    let out_rate = out.sample_rate;
                    self.audio_out = Some(out);
                    let _ = self
                        .swap_tx
                        .send(EngineSwap::Output(Some(AudioParams { producer, out_rate })));
                }
                Err(e) => {
                    warn!("audio output {name:?}: {e}; running silent");
                    let _ = self.swap_tx.send(EngineSwap::Output(None));
                }
            }
            self.out_name = name;
        } else {
            // Release the old device and take the engine off it first, then
            // open the new one in the background: the operator picked from a
            // list, and that click must not freeze the UI for however long the
            // sound stack takes to answer. Assigning `pending_mic` drops any
            // open still in flight, so an earlier pick can never land on top of
            // this one.
            self.mic_in = None;
            let _ = self.swap_tx.send(EngineSwap::Input(None));
            self.pending_mic = Some(sdroxide_audio::start_input_background(name.clone(), 48_000));
            self.in_name = name;
        }
        self.persist_selection();
    }

    /// Every device question this machine can answer, in one place — see
    /// [`crate::devices`]. Answered inline: the radio is on this machine, and
    /// the settings dialog calls this after its window closure rather than
    /// during it, so a blocking scan costs one frame and nothing else.
    fn probe(&mut self, req: sdroxide_types::DeviceProbe) -> Option<sdroxide_types::ProbeAnswer> {
        // Named, because one of the questions is about a radio rather than
        // about this machine: the diagnostic report is this radio's session,
        // and on a station with two of a kind the other one's is worse than
        // none.
        Some(crate::devices::probe(req, self.store.radio_id()))
    }

    fn radio_config(&self) -> Option<RadioConfig> {
        Some(self.store.load_radio_config())
    }

    fn set_radio_config(&mut self, cfg: RadioConfig) {
        // Persist now; `reopen_source` applies it to the running engine.
        if let Err(e) = self.store.save_radio_config(&cfg) {
            warn!("saving radio config: {e}");
        }
    }

    fn reopen_source(&mut self) {
        // The engine rebuilds the source from the freshly persisted radio config.
        let _ = self.swap_tx.send(EngineSwap::ReopenSource);
    }

    fn nudge_shared_stores(&mut self) {
        let _ = self.swap_tx.send(EngineSwap::ReloadSharedStores);
    }

    fn shutdown(&mut self) {
        // The engine runs until its last command sender drops, so disconnect
        // it by swapping the senders for endpoints wired to nothing. The audio
        // streams go too — an exclusive device has to be free before another
        // radio (or another program) can have it.
        //
        // Logged stage by stage: a close that hangs here has to name which join
        // it is stuck on, or the only symptom is a window that will not go away.
        info!("shutdown: disconnecting the engine");
        let (dead_cmd, _) = sdroxide_radio::crossbeam_channel::unbounded();
        self.cmd_tx = dead_cmd;
        let (dead_swap, _) = sdroxide_radio::crossbeam_channel::unbounded();
        self.swap_tx = dead_swap;
        info!("shutdown: closing the audio output");
        self.audio_out = None;
        info!("shutdown: closing the microphone");
        self.mic_in = None;
        // An open still in flight is abandoned rather than waited for; whatever
        // it opens is closed again as soon as it exists.
        self.pending_mic = None;
        if let Some(t) = self.thread.take() {
            info!("shutdown: joining the engine thread");
            let _ = t.join();
            info!("shutdown: engine thread joined");
        }
    }
}
