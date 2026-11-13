//! An [`IqSource`] for an Airspy HF+ driven over USB by the native driver in
//! `sdroxide-airspyhf` — no SoapySDR, no libairspyhf, no libusb.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for this hardware.

use std::time::Duration;

use sdroxide_airspyhf::AirspyHfHandle;
use sdroxide_radio::{Complex32, IqSource, Result};
use sdroxide_types::{AirspyHfConfig, AirspyHfModel};

/// How long the receiver may deliver nothing before the connection counts as
/// dead. Same three seconds as the RTL-SDR: this is a local USB device, so
/// there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

pub struct AirspyHfSource {
    handle: AirspyHfHandle,
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    /// Mirrors of the settings the panel drives, so `current_gains` can answer
    /// without a round trip to the stream thread.
    attenuator_db: f64,
    agc: bool,
    bias_tee: bool,
}

impl AirspyHfSource {
    pub fn open(cfg: &AirspyHfConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = AirspyHfHandle::open(cfg, center_hz)?;
        let label = handle.label.clone();
        tracing::info!("Airspy HF+ source ready: {label}, center {center_hz:.0} Hz");
        Ok(AirspyHfSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            attenuator_db: cfg.attenuator_db,
            agc: cfg.agc,
            bias_tee: cfg.bias_tee,
            handle,
        })
    }

    pub fn model(&self) -> AirspyHfModel {
        self.handle.model
    }

    /// The rates this particular receiver has, which depend on the model and
    /// the firmware together — so the caps must publish these and not a
    /// constant.
    pub fn available_rates(&self) -> &[f64] {
        &self.handle.rates_hz
    }

    /// The attenuator range and step the receiver reported, in dB.
    pub fn attenuator_range_db(&self) -> (f64, f64) {
        (self.handle.att_max_db, self.handle.att_step_db)
    }
}

impl IqSource for AirspyHfSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    fn sample_rate(&self) -> f64 {
        self.handle.sample_rate_hz
    }

    fn center_hz(&self) -> f64 {
        self.center
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        self.handle.set_center_hz(hz);
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
            // Nothing yet — brief nap so the DSP loop doesn't spin hot.
            std::thread::sleep(Duration::from_millis(2));
            return Ok(0);
        }
        for (p, out) in buf.iter_mut().enumerate().take(pairs) {
            *out = Complex32::new(self.rx_scratch[2 * p], self.rx_scratch[2 * p + 1]);
        }
        Ok(pairs)
    }

    /// A centre of its own — the receiver's own LO — so the engine may centre the
    /// I/Q on a resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The attenuator, plus pseudo-elements carrying the switches.
    ///
    /// Routing the switches through `SetGain` rather than adding `Command`
    /// variants keeps `Command`, `DeviceCaps` and the engine untouched for six
    /// settings only this backend has. The encodings live beside the names on
    /// [`AirspyHfConfig`] so the two ends cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            AirspyHfConfig::ATT_ELEMENT => {
                self.attenuator_db = db;
                self.handle.set_attenuator_db(db);
            }
            AirspyHfConfig::AGC_ELEMENT => {
                self.agc = db >= 0.5;
                self.handle.set_agc(self.agc);
            }
            AirspyHfConfig::AGC_THRESHOLD_ELEMENT => {
                self.handle.set_agc_threshold(db >= 0.5);
            }
            AirspyHfConfig::LNA_ELEMENT => {
                self.handle.set_lna(db >= 0.5);
            }
            AirspyHfConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5;
                self.handle.set_bias_tee(self.bias_tee);
            }
            AirspyHfConfig::PPB_ELEMENT => {
                // Parts per billion, so the range is wide: ±1000 ppm covers
                // anything a crystal could plausibly be out by.
                self.handle.set_calibration_ppb(db.round().clamp(-1e6, 1e6) as i32);
            }
            AirspyHfConfig::LIB_DSP_ELEMENT => {
                self.handle.set_lib_dsp(db >= 0.5);
            }
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (AirspyHfConfig::ATT_ELEMENT.to_string(), self.attenuator_db),
            (AirspyHfConfig::AGC_ELEMENT.to_string(), f64::from(u8::from(self.agc))),
        ]
    }

    /// A receiver that has been unplugged, or whose thread has died, is
    /// reported as needing a reopen so the engine reconnects on its own —
    /// which is what makes replugging one Just Work rather than needing Apply
    /// pressed.
    fn needs_reopen(&self) -> bool {
        !self.handle.is_alive() || self.handle.silent_for() >= SILENCE_BEFORE_REOPEN
    }

    /// Hand the receiver back before the engine opens its replacement. Without
    /// this, changing anything in Settings → Radio on a running HF+ fails with
    /// "held by another program" — the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see.
    ///
    /// Three things qualify. A bias tee putting DC on the feedline is worth a
    /// standing on-screen reminder — the setting is persisted, so it survives a
    /// restart with nothing else to indicate it. A rate that had to be snapped
    /// means the radio is not running what the config says. And this backend
    /// has never been tried against real hardware, which an operator seeing odd
    /// behaviour deserves to know before they go looking for a fault in their
    /// antenna.
    fn open_status(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if self.bias_tee {
            parts.push(format!("{}: bias tee is ON — DC on the antenna coax", self.label));
        }
        if let Some(from) = self.handle.snapped_from {
            parts.push(format!(
                "{:.0} kSPS is not available on this receiver; using {:.0} kSPS. \
                 Pick a listed rate in Settings → Radio to make this permanent.",
                from / 1e3,
                self.handle.sample_rate_hz / 1e3,
            ));
        }
        parts.push(
            "Airspy HF+ support is new and has not been verified against real hardware. \
             If it misbehaves, Settings → Radio has a Copy diagnostic report button."
                .to_string(),
        );
        Some(parts.join(" — "))
    }
}
