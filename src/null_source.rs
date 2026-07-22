//! A placeholder [`IqSource`] used when the configured radio interface can't be
//! opened at startup (no SoapySDR device found, the HPSDR is unreachable, etc.).
//! It produces no samples but keeps the engine — and therefore the whole GUI,
//! including the Settings dialog — running, so the user can pick a working
//! interface and restart instead of the program failing to launch.

use sdroxide_radio::{Complex32, IqSource, Result};

pub struct NullSource {
    center: f64,
    status: String,
}

impl NullSource {
    pub fn new(center_hz: f64, status: String) -> Self {
        NullSource { center: center_hz, status }
    }
}

impl IqSource for NullSource {
    fn sample_rate(&self) -> f64 {
        48_000.0
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }

    fn read(&mut self, _buf: &mut [Complex32]) -> Result<usize> {
        // No data; nap so the engine loop doesn't spin.
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(0)
    }

    fn describe(&self) -> String {
        "No radio".into()
    }

    fn open_status(&self) -> Option<String> {
        Some(self.status.clone())
    }
}
