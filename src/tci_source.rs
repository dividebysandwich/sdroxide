//! An [`IqSource`] for a TCI (Transceiver Control Interface) server. Receive is
//! wideband IQ over the WebSocket → the engine's normal DDC/demod path
//! (`audio_mode = false`); transmit is raw 48 kHz audio (`tx_write_audio`) which
//! the rig modulates (`caps.tx_audio`). Control (freq/mode/PTT) goes over the
//! same WebSocket.

use std::time::Duration;

use sdroxide_radio::{Complex32, ControlUpdate, IqSource, Result};
use sdroxide_tci::{TciHandle, TciUpdate};

pub struct TciSource {
    handle: TciHandle,
    center: f64,
    scratch: Vec<f32>,
    label: String,
}

impl TciSource {
    pub fn open(address: &str, iq_rate_hz: f64, center_hz: f64) -> anyhow::Result<Self> {
        let handle = TciHandle::connect(address, iq_rate_hz)?;
        handle.set_center(center_hz);
        let label =
            format!("TCI {} @ {address} ({:.0} kHz IQ)", handle.device, iq_rate_hz / 1000.0);
        Ok(TciSource { center: center_hz, scratch: Vec::new(), label, handle })
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.handle.sample_rate_hz
    }
}

impl IqSource for TciSource {
    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.scratch.len() < need {
            self.scratch.resize(need, 0.0);
        }
        let n = self.handle.rx_read(&mut self.scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.scratch[2 * p], self.scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn set_control_mode(&mut self, mode: sdroxide_types::Mode) -> Result<()> {
        self.handle.set_mode(mode);
        Ok(())
    }

    fn poll_control(&mut self) -> Vec<ControlUpdate> {
        self.handle
            .poll_updates()
            .into_iter()
            .map(|u| match u {
                TciUpdate::Freq(hz) => ControlUpdate::Freq(hz),
                TciUpdate::Mode(m) => ControlUpdate::Mode(m),
            })
            .collect()
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        Ok(self.handle.tx_begin(center_hz))
    }

    fn tx_write_audio(&mut self, audio: &[f32]) -> Result<()> {
        self.handle.tx_write(audio);
        Ok(())
    }

    fn tx_end(&mut self) -> Result<()> {
        self.handle.tx_end();
        Ok(())
    }

    fn set_tx_drive(&mut self, frac: f64) {
        self.handle.set_drive(frac);
    }

    fn set_tune_drive(&mut self, frac: f64) {
        self.handle.set_tune_drive(frac);
    }
}
