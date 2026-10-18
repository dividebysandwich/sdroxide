//! A placeholder [`IqSource`] used when the configured radio interface can't be
//! opened at startup (no SoapySDR device found, the HPSDR is unreachable, etc.).
//! It produces no samples but keeps the engine — and therefore the whole GUI,
//! including the Settings dialog — running, so the user can pick a working
//! interface and restart instead of the program failing to launch.

use sdroxide_radio::{Complex32, IqSource, Result};

pub struct NullSource {
    center: f64,
    status: String,
    /// Whether the engine should keep trying the configured interface behind
    /// this stand-in. True for every stand-in that is standing in for
    /// something we still want — false only for [`NullSource::off`], where
    /// there is nothing to reconnect to until the operator says so.
    retry: bool,
}

impl NullSource {
    pub fn new(center_hz: f64, status: String) -> Self {
        NullSource { center: center_hz, status, retry: true }
    }

    /// The stand-in for a radio whose receiver another radio has borrowed as
    /// its panadapter. Its device is open — just not by this engine.
    ///
    /// It keeps retrying like any other stand-in, and the reopen factory keeps
    /// refusing while the pairing stands. That is what makes undoing the
    /// pairing enough on its own: nothing has to remember to wake this radio
    /// up, and an attempt that loses the race to the borrower's release is
    /// simply tried again.
    pub fn attached(center_hz: f64, owner: u32) -> Self {
        NullSource {
            center: center_hz,
            status: format!(
                "This radio is the panadapter for radio {} — its spectrum and audio are on that \
                 radio's tab. Clear the Panadapter receiver box there to use it on its own again.",
                owner + 1
            ),
            retry: true,
        }
    }

    /// The stand-in for a radio the operator has switched off. Nothing of its
    /// interface is open — no device claimed, no CAT port held, no rig dialled
    /// — and, alone among the stand-ins, this one does not ask to be reopened:
    /// a radio that is off is not a radio waiting to come back, and a retry
    /// every few seconds is exactly what switching it off was meant to stop.
    /// Switching it on asks the engine to rebuild the front end outright.
    pub fn off(center_hz: f64) -> Self {
        NullSource { center: center_hz, status: String::new(), retry: false }
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

    /// A switched-off radio has no status line: its tab already says it is off,
    /// and a banner repeating that is noise. Every other stand-in says why it is
    /// standing in.
    fn open_status(&self) -> Option<String> {
        self.retry.then(|| self.status.clone())
    }

    /// Normally always: this is the "no radio" placeholder, so the engine keeps
    /// trying the configured interface until it comes up. A radio the operator
    /// switched off ([`NullSource::off`]) is the exception — there is nothing
    /// there to wait for.
    fn needs_reopen(&self) -> bool {
        self.retry
    }
}
