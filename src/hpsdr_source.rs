//! An [`IqSource`] for an OpenHPSDR ethernet SDR (Protocol 2). The board's DDC
//! delivers wideband complex I/Q, so this drives the engine's normal DDC/demod
//! path exactly like a SoapySDR device (`audio_mode = false`); transmit I/Q goes
//! to the board's DUC.

use std::net::Ipv4Addr;
use std::time::Duration;

use sdroxide_hpsdr::HpsdrHandle;
use sdroxide_radio::{Complex32, IqSource, Result};

pub struct HpsdrSource {
    handle: HpsdrHandle,
    center: f64,
    rx_scratch: Vec<f32>,
    tx_scratch: Vec<f32>,
    label: String,
}

impl HpsdrSource {
    /// Open a Protocol 2 connection and start streaming at `center_hz`.
    pub fn open(ip: Ipv4Addr, sample_rate_hz: f64, center_hz: f64) -> anyhow::Result<Self> {
        let handle = HpsdrHandle::open(ip, sample_rate_hz)?;
        handle.set_rx_freq(center_hz);
        let label =
            format!("HPSDR {} @ {ip} ({:.3} Msps)", handle.board, handle.sample_rate_hz / 1e6);
        Ok(HpsdrSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            tx_scratch: Vec::new(),
            label,
            handle,
        })
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    pub fn board(&self) -> &str {
        &self.handle.board
    }
}

impl IqSource for HpsdrSource {
    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_rx_freq(hz);
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        let need = buf.len() * 2;
        if self.rx_scratch.len() < need {
            self.rx_scratch.resize(need, 0.0);
        }
        let n = self.handle.rx_read(&mut self.rx_scratch[..need]);
        let pairs = n / 2;
        if pairs == 0 {
            // No samples yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for p in 0..pairs {
            buf[p] = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    fn tx_begin(&mut self, center_hz: f64, _rate: f64) -> Result<f64> {
        Ok(self.handle.tx_begin(center_hz))
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        self.tx_scratch.clear();
        self.tx_scratch.reserve(samples.len() * 2);
        for s in samples {
            self.tx_scratch.push(s.re);
            self.tx_scratch.push(s.im);
        }
        self.handle.tx_write(&self.tx_scratch);
        Ok(())
    }

    fn tx_end(&mut self) -> Result<()> {
        self.handle.tx_end();
        Ok(())
    }
}
