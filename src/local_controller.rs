//! In-process [`RadioController`]: wraps the engine's channel endpoints, and
//! owns the cpal stream handles so audio devices can be swapped at runtime
//! (the engine only ever holds the ring endpoints).

use sdroxide_radio::crossbeam_channel::{Receiver, Sender};
use sdroxide_radio::{AudioParams, AudioSwap, EngineHandles, MicParams, triple_buffer};
use sdroxide_types::{AudioDevices, Command, RadioConfig, RadioController, RadioEvent, SpectrumFrame};
use tracing::warn;

pub struct LocalController {
    cmd_tx: Sender<Command>,
    event_rx: Receiver<RadioEvent>,
    spectrum: triple_buffer::Output<SpectrumFrame>,
    audio_swap_tx: Sender<AudioSwap>,
    /// Live cpal streams (they must outlive their ring endpoints in the engine).
    audio_out: Option<sdroxide_audio::AudioOutput>,
    mic_in: Option<sdroxide_audio::AudioInput>,
    /// Currently selected device names; `None` = system default.
    out_name: Option<String>,
    in_name: Option<String>,
}

impl LocalController {
    pub fn new(
        handles: EngineHandles,
        audio_out: Option<sdroxide_audio::AudioOutput>,
        mic_in: Option<sdroxide_audio::AudioInput>,
        out_name: Option<String>,
        in_name: Option<String>,
    ) -> Self {
        LocalController {
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum: handles.spectrum_out,
            audio_swap_tx: handles.audio_swap_tx,
            audio_out,
            mic_in,
            out_name,
            in_name,
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
        let _ = self.cmd_tx.send(cmd);
    }

    fn poll_event(&mut self) -> Option<RadioEvent> {
        if let Ok(ev) = self.event_rx.try_recv() {
            return Some(ev);
        }
        if self.spectrum.update() {
            let f = self.spectrum.peek_output_buffer();
            if !f.bins.is_empty() {
                return Some(RadioEvent::Spectrum(f.clone()));
            }
        }
        None
    }

    fn wants_repaint_soon(&self) -> bool {
        !self.event_rx.is_empty() || self.spectrum.updated()
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
                        .audio_swap_tx
                        .send(AudioSwap::Output(Some(AudioParams { producer, out_rate })));
                }
                Err(e) => {
                    warn!("audio output {name:?}: {e}; running silent");
                    let _ = self.audio_swap_tx.send(AudioSwap::Output(None));
                }
            }
            self.out_name = name;
        } else {
            self.mic_in = None;
            match sdroxide_audio::start_input(name.as_deref(), 48_000) {
                Ok((input, consumer)) => {
                    let rate = input.sample_rate;
                    self.mic_in = Some(input);
                    let _ = self
                        .audio_swap_tx
                        .send(AudioSwap::Input(Some(MicParams { consumer, rate })));
                }
                Err(e) => {
                    warn!("audio input {name:?}: {e}; TX carries silence");
                    let _ = self.audio_swap_tx.send(AudioSwap::Input(None));
                }
            }
            self.in_name = name;
        }
        self.persist_selection();
    }

    fn soapy_supported(&self) -> bool {
        cfg!(feature = "soapy")
    }

    fn serial_ports(&self) -> Vec<String> {
        sdroxide_cat::available_ports()
    }

    fn discover_hpsdr(&self) -> Vec<sdroxide_types::HpsdrDevice> {
        sdroxide_hpsdr::discover_default()
    }

    fn test_tci(&self, address: &str) -> Result<String, String> {
        sdroxide_tci::test_connection(address, std::time::Duration::from_secs(3))
    }

    fn radio_config(&self) -> Option<RadioConfig> {
        Some(sdroxide_config::load_radio_config())
    }

    fn set_radio_config(&mut self, cfg: RadioConfig) {
        // Persisted now; the source/engine adopt it on the next launch.
        if let Err(e) = sdroxide_config::save_radio_config(&cfg) {
            warn!("saving radio config: {e}");
        }
    }
}
