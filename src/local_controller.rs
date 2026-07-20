//! In-process [`RadioController`]: wraps the engine's channel endpoints.

use sdroxide_radio::EngineHandles;
use sdroxide_radio::crossbeam_channel::{Receiver, Sender};
use sdroxide_radio::triple_buffer;
use sdroxide_types::{Command, RadioController, RadioEvent, SpectrumFrame};

pub struct LocalController {
    cmd_tx: Sender<Command>,
    event_rx: Receiver<RadioEvent>,
    spectrum: triple_buffer::Output<SpectrumFrame>,
}

impl LocalController {
    pub fn new(handles: EngineHandles) -> Self {
        LocalController {
            cmd_tx: handles.cmd_tx,
            event_rx: handles.event_rx,
            spectrum: handles.spectrum_out,
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
}
