//! An [`IqSource`] for a HydraSDR RFOne driven over USB by the native driver in
//! `sdroxide-hydrasdr` — no SoapySDR, no libhydrasdr, no libusb.
//!
//! Receive only: the trait's transmit methods already default to errors, which
//! is the correct answer for this hardware.
//!
//! Note this is **not** the Airspy R2 backend next door, despite the shared
//! ancestry: the two disagree about how wide `SET_FREQ` is, so neither driver
//! tunes the other's hardware correctly.

use std::time::Duration;

use sdroxide_hydrasdr::HydraSdrHandle;
use sdroxide_hydrasdr::protocol::{GainCurve, RfPort};
use sdroxide_radio::{Complex32, IqSource, Result};
use sdroxide_types::{HydraSdrConfig, HydraSdrGain, HydraSdrPort};

/// How long the receiver may deliver nothing before the connection counts as
/// dead. Same three seconds as the other native USB backends: this is a local
/// device, so there is no network to be briefly slow.
const SILENCE_BEFORE_REOPEN: Duration = Duration::from_secs(3);

pub struct HydraSdrSource {
    handle: HydraSdrHandle,
    center: f64,
    rx_scratch: Vec<f32>,
    label: String,
    /// Mirrors of the settings the panel drives, so `current_gains` can answer
    /// without a round trip to the stream thread.
    gain_step: u8,
    curve: HydraSdrGain,
    rf_port: HydraSdrPort,
    bias_tee: bool,
}

impl HydraSdrSource {
    pub fn open(cfg: &HydraSdrConfig, center_hz: f64) -> anyhow::Result<Self> {
        let handle = HydraSdrHandle::open(cfg, center_hz)?;
        let label = format!("{} @ {:.3} Msps", handle.label, handle.sample_rate_hz / 1e6);
        tracing::info!(
            "HydraSDR source ready: {label}, center {center_hz:.0} Hz, {}, firmware {:?}",
            if handle.packing_active { "12-bit packed" } else { "unpacked" },
            handle.firmware,
        );
        Ok(HydraSdrSource {
            center: center_hz,
            rx_scratch: Vec::new(),
            label,
            gain_step: cfg.gain_step,
            curve: cfg.gain_curve,
            rf_port: cfg.rf_port,
            bias_tee: cfg.bias_tee,
            handle,
        })
    }

    /// The complex rates this particular receiver has: the three its firmware
    /// lists plus whichever of the four alternates it turned out to carry.
    pub fn available_rates(&self) -> &[f64] {
        &self.handle.rates_hz
    }
}

impl IqSource for HydraSdrSource {
    /// The engine is transmitting and has stopped reading, or has started
    /// again. Passed straight through to the stream thread, which keeps
    /// receiving either way — this only decides whether a full ring is
    /// reported as an overrun or as the ordinary cost of an over. See
    /// [`IqSource::set_rx_paused`].
    fn set_rx_paused(&mut self, paused: bool) {
        self.handle.set_rx_paused(paused);
    }

    /// The rate the *host* produces, which is half what the receiver runs at.
    /// Read live, because a rate change moves it under the engine.
    fn sample_rate(&self) -> f64 {
        self.handle.current_rate_hz()
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

    /// A centre of its own — the tuner's LO — so the engine may centre the I/Q on a
    /// resting dial (`Engine::recentre_after_dwell`).
    fn recentre_on_dwell(&self) -> bool {
        true
    }

    fn describe(&self) -> String {
        self.label.clone()
    }

    /// The one real gain element — a step along the selected curve — plus
    /// pseudo-elements carrying the switches.
    ///
    /// Routing the switches through `SetGain` rather than adding `Command`
    /// variants keeps `Command`, `DeviceCaps` and the engine untouched for
    /// settings only this backend has. The names live on [`HydraSdrConfig`] so
    /// the two ends cannot drift apart.
    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        match name {
            HydraSdrConfig::GAIN_ELEMENT => {
                self.gain_step = db.clamp(0.0, (HydraSdrConfig::GAIN_STEPS - 1) as f64) as u8;
                self.handle.set_gain_step(self.gain_step);
            }
            HydraSdrConfig::CURVE_ELEMENT => {
                self.curve = HydraSdrGain::from_code(db.clamp(0.0, 1.0) as u8);
                self.handle.set_curve(GainCurve::from_code(self.curve.code()));
            }
            HydraSdrConfig::LNA_AGC_ELEMENT => self.handle.set_lna_agc(db >= 0.5),
            HydraSdrConfig::MIXER_AGC_ELEMENT => self.handle.set_mixer_agc(db >= 0.5),
            HydraSdrConfig::RF_PORT_ELEMENT => {
                self.rf_port = HydraSdrPort::from_code(db.clamp(0.0, 2.0) as u8);
                self.handle.set_rf_port(RfPort::from_code(self.rf_port.code()));
                // Only the antenna socket has a bias tee behind it, and the
                // driver drops it on the way to a cable port. Mirroring that
                // here keeps `open_status` honest about what is on the coax.
                self.bias_tee &= self.rf_port.has_bias_tee();
            }
            HydraSdrConfig::BIAS_TEE_ELEMENT => {
                self.bias_tee = db >= 0.5 && self.rf_port.has_bias_tee();
                self.handle.set_bias_tee(db >= 0.5);
            }
            HydraSdrConfig::DC_BLOCK_ELEMENT => self.handle.set_dc_block(db >= 0.5),
            // Packing is decided at open: it changes how every completion is
            // decoded, and switching it under a running stream would misread
            // whatever is already in flight. The settings tab asks for a
            // reconnect instead.
            _ => {}
        }
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        vec![
            (HydraSdrConfig::GAIN_ELEMENT.to_string(), f64::from(self.gain_step)),
            (HydraSdrConfig::CURVE_ELEMENT.to_string(), f64::from(self.curve.code())),
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
    /// this, changing anything in Settings → Radio on a running HydraSDR fails
    /// with "held by another program" — the other program being us.
    fn release(&mut self) {
        self.handle.release();
    }

    /// Surface what an operator needs to know but cannot see.
    fn open_status(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if self.bias_tee {
            parts.push(format!("{}: bias tee is ON — DC on the antenna coax", self.label));
        }
        if self.rf_port != HydraSdrPort::Ant {
            parts.push(format!(
                "the tuner is on the {} socket, not the antenna port",
                self.rf_port.name()
            ));
        }
        if let Some(from) = self.handle.snapped_from {
            parts.push(format!(
                "{:.3} Msps is not available on this receiver; using {:.3} Msps. \
                 Four of this radio's seven rates live in the firmware's alternate \
                 table, and an older build may not carry them. Pick a listed rate \
                 in Settings → Radio to make this permanent.",
                from / 1e6,
                self.handle.current_rate_hz() / 1e6,
            ));
        }
        if !self.handle.packing_active {
            parts.push(
                "12-bit packing is not available on this firmware, so the USB link is \
                 carrying a third more than it needs to — expect dropped samples at \
                 the higher rates"
                    .to_string(),
            );
        }
        if !self.handle.rf_port_active {
            parts.push(
                "this firmware will not select an RF port, so the receiver is on ANT".to_string(),
            );
        }
        if self.handle.legacy_usb_id {
            parts.push(
                "this board is on the legacy USB id it shares with the Airspy R2, so it \
                 is a prototype rather than a production RFOne"
                    .to_string(),
            );
        }
        parts.push(
            "HydraSDR RFOne support is new and has not been verified against real \
             hardware. If it misbehaves, Settings → Radio has a Copy diagnostic \
             report button."
                .to_string(),
        );
        Some(parts.join(" — "))
    }
}
