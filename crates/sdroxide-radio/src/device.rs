use soapysdr::Direction;
use tracing::info;

use sdroxide_types::{DeviceCaps, GainElement};

use crate::{Complex32, IqSource, RadioError, Result};

/// One enumerated device: its label plus the args string that opens it.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub label: String,
    pub driver: String,
    pub args: String,
}

/// Enumerate devices matching a SoapySDR args filter ("" for all).
pub fn enumerate_devices(filter: &str) -> Result<Vec<DeviceInfo>> {
    let found = soapysdr::enumerate(filter)?;
    Ok(found
        .iter()
        .map(|args| DeviceInfo {
            label: args.get("label").unwrap_or_default().to_string(),
            driver: args.get("driver").unwrap_or_default().to_string(),
            args: args.to_string(),
        })
        .collect())
}

/// An open SoapySDR device plus its probed capabilities.
pub struct SoapyDevice {
    dev: soapysdr::Device,
    caps: DeviceCaps,
}

impl SoapyDevice {
    pub fn open(args: &str) -> Result<Self> {
        let dev = soapysdr::Device::new(args)?;
        let caps = probe_caps(&dev)?;
        info!(driver = %caps.driver, label = %caps.label, "opened SoapySDR device");
        Ok(SoapyDevice { dev, caps })
    }

    pub fn caps(&self) -> &DeviceCaps {
        &self.caps
    }

    pub fn device(&self) -> &soapysdr::Device {
        &self.dev
    }

    /// Pick a working sample rate: the requested one if supported, otherwise
    /// the closest supported value, preferring 48 kHz power-of-two multiples.
    pub fn choose_sample_rate(&self, requested: f64) -> f64 {
        let ok = |r: f64| {
            self.caps.rate_ranges.iter().any(|&(lo, hi)| r >= lo && r <= hi)
                || self.caps.sample_rates.iter().any(|&s| (s - r).abs() < 1.0)
        };
        if ok(requested) {
            return requested;
        }
        // Preferred ladder: 48k * 2^k
        let mut candidates: Vec<f64> = (0..12).map(|k| 48_000.0 * (1u64 << k) as f64).collect();
        candidates.extend(self.caps.sample_rates.iter().copied());
        candidates
            .into_iter()
            .filter(|&r| ok(r))
            .min_by(|a, b| {
                (a - requested).abs().total_cmp(&(b - requested).abs())
            })
            .unwrap_or(requested)
    }

    /// Open, configure, and activate an RX stream on channel 0.
    ///
    /// `gain_db`: explicit overall RX gain; `None` enables hardware AGC when
    /// available, else a moderate manual gain.
    pub fn rx_source(
        self,
        sample_rate: f64,
        center_hz: f64,
        gain_db: Option<f64>,
    ) -> Result<SoapyRxSource> {
        let channel = 0;
        let rate = self.choose_sample_rate(sample_rate);
        self.dev.set_sample_rate(Direction::Rx, channel, rate)?;
        self.dev.set_frequency(Direction::Rx, channel, center_hz, ())?;

        if let Some(g) = gain_db {
            let _ = self.dev.set_gain_mode(Direction::Rx, channel, false);
            self.dev.set_gain(Direction::Rx, channel, g)?;
        } else if self.dev.has_gain_mode(Direction::Rx, channel).unwrap_or(false) {
            let _ = self.dev.set_gain_mode(Direction::Rx, channel, true);
        } else if let Ok(range) = self.dev.gain_range(Direction::Rx, channel) {
            let g = range.minimum + 0.4 * (range.maximum - range.minimum);
            let _ = self.dev.set_gain(Direction::Rx, channel, g);
        }

        let actual_rate = self.dev.sample_rate(Direction::Rx, channel)?;

        // SAFETY RAIL: force every TX gain stage to its minimum at startup
        // so keying up can never emit unexpected power.
        if self.caps.tx_channels > 0 {
            let _ = self.dev.set_gain(Direction::Tx, channel, 0.0);
            for name in self.dev.list_gains(Direction::Tx, channel).unwrap_or_default() {
                if let Ok(r) = self.dev.gain_element_range(Direction::Tx, channel, name.as_str()) {
                    let _ = self
                        .dev
                        .set_gain_element(Direction::Tx, channel, name.as_str(), r.minimum);
                }
            }
        }

        let mut source = SoapyRxSource {
            dev: self.dev,
            caps: self.caps,
            rx_stream: None,
            tx_stream: None,
            channel,
            sample_rate: actual_rate,
            center_hz,
            overflows: 0,
            tx_underflows: 0,
        };
        source.open_rx_stream()?;
        Ok(source)
    }
}

/// A live SoapySDR device: an RX stream (closed while transmitting on
/// half-duplex hardware), plus a TX stream while keyed.
pub struct SoapyRxSource {
    dev: soapysdr::Device,
    caps: DeviceCaps,
    rx_stream: Option<soapysdr::RxStream<Complex32>>,
    tx_stream: Option<soapysdr::TxStream<Complex32>>,
    channel: usize,
    sample_rate: f64,
    center_hz: f64,
    overflows: u64,
    tx_underflows: u64,
}

impl SoapyRxSource {
    pub fn caps(&self) -> &DeviceCaps {
        &self.caps
    }

    /// Create and activate a fresh RX stream, reasserting the RX rate and
    /// frequency first (half-duplex hardware shares the LO/clock with TX, so
    /// a TX cycle can leave them on TX values).
    fn open_rx_stream(&mut self) -> Result<()> {
        self.dev.set_sample_rate(Direction::Rx, self.channel, self.sample_rate)?;
        self.dev.set_frequency(Direction::Rx, self.channel, self.center_hz, ())?;
        let mut stream = self.dev.rx_stream::<Complex32>(&[self.channel])?;
        stream.activate(None)?;
        self.rx_stream = Some(stream);
        info!(rate = self.sample_rate, center = self.center_hz, "RX stream active");
        Ok(())
    }
}

impl IqSource for SoapyRxSource {
    fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    fn center_hz(&self) -> f64 {
        self.center_hz
    }

    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.dev.set_frequency(Direction::Rx, self.channel, hz, ())?;
        self.center_hz = hz;
        Ok(())
    }

    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // No RX stream while transmitting on half-duplex hardware.
        let Some(stream) = self.rx_stream.as_mut() else {
            return Ok(0);
        };
        match stream.read(&mut [buf], 200_000) {
            Ok(n) => Ok(n),
            Err(e) if e.code == soapysdr::ErrorCode::Timeout => Ok(0),
            // Overflow = samples dropped because we read too slowly.
            // Recoverable: log and keep streaming.
            Err(e) if e.code == soapysdr::ErrorCode::Overflow => {
                self.overflows += 1;
                if self.overflows.is_power_of_two() {
                    tracing::warn!(count = self.overflows, "RX overflow (samples dropped)");
                }
                Ok(0)
            }
            Err(e) => Err(RadioError::Soapy(e)),
        }
    }

    fn describe(&self) -> String {
        format!("{} ({:.3} Msps)", self.caps.label, self.sample_rate / 1e6)
    }

    fn set_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        let _ = self.dev.set_gain_mode(Direction::Rx, self.channel, false);
        self.dev.set_gain_element(Direction::Rx, self.channel, name, db)?;
        Ok(())
    }

    fn set_antenna(&mut self, name: &str) -> Result<()> {
        self.dev.set_antenna(Direction::Rx, self.channel, name)?;
        Ok(())
    }

    fn current_gains(&self) -> Vec<(String, f64)> {
        self.dev
            .list_gains(Direction::Rx, self.channel)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|name| {
                let db = self.dev.gain_element(Direction::Rx, self.channel, name.as_str()).ok()?;
                Some((name, db))
            })
            .collect()
    }

    fn current_antenna(&self) -> String {
        self.dev.antenna(Direction::Rx, self.channel).unwrap_or_default()
    }

    fn tx_begin(&mut self, center_hz: f64, rate: f64) -> Result<f64> {
        if self.caps.tx_channels == 0 {
            return Err(RadioError::Msg("device is not transmit capable".into()));
        }
        // Half duplex (HackRF): fully CLOSE the RX stream before switching
        // direction. Merely deactivating it and reactivating after TX leaves
        // the SoapyHackRF RX path corrupted (repeated/aliased buffers) until
        // the device is reopened; a fresh stream avoids that.
        if !self.caps.full_duplex {
            self.rx_stream = None;
        }
        self.dev.set_sample_rate(Direction::Tx, self.channel, rate)?;
        self.dev.set_frequency(Direction::Tx, self.channel, center_hz, ())?;
        let actual = self.dev.sample_rate(Direction::Tx, self.channel)?;

        let mut tx = self.dev.tx_stream::<Complex32>(&[self.channel])?;
        tx.activate(None)?;
        self.tx_stream = Some(tx);
        tracing::info!(center_hz, rate = actual, "TX active");
        Ok(actual)
    }

    fn tx_write(&mut self, samples: &[Complex32]) -> Result<()> {
        let Some(tx) = self.tx_stream.as_mut() else {
            return Err(RadioError::Msg("TX not active".into()));
        };
        match tx.write_all(&[samples], None, false, 500_000) {
            Ok(()) => Ok(()),
            Err(e) if e.code == soapysdr::ErrorCode::Underflow => {
                self.tx_underflows += 1;
                if self.tx_underflows.is_power_of_two() {
                    tracing::warn!(count = self.tx_underflows, "TX underflow");
                }
                Ok(())
            }
            Err(e) => Err(RadioError::Soapy(e)),
        }
    }

    fn tx_end(&mut self) -> Result<()> {
        // Dropping the TX stream deactivates and closes it.
        self.tx_stream = None;
        if !self.caps.full_duplex {
            // Reopen a fresh RX stream (see tx_begin).
            self.open_rx_stream()?;
        }
        tracing::info!("TX ended, RX restored");
        Ok(())
    }

    fn set_tx_gain_element(&mut self, name: &str, db: f64) -> Result<()> {
        self.dev.set_gain_element(Direction::Tx, self.channel, name, db)?;
        Ok(())
    }

    fn current_tx_gains(&self) -> Vec<(String, f64)> {
        self.dev
            .list_gains(Direction::Tx, self.channel)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|name| {
                let db = self.dev.gain_element(Direction::Tx, self.channel, name.as_str()).ok()?;
                Some((name, db))
            })
            .collect()
    }
}

fn probe_caps(dev: &soapysdr::Device) -> Result<DeviceCaps> {
    let driver = dev.driver_key().unwrap_or_default();
    let hw_info = dev.hardware_info().unwrap_or_default();
    let label = hw_info
        .get("label")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{} ({})", driver, dev.hardware_key().unwrap_or_default()));

    let rx_channels = dev.num_channels(Direction::Rx).unwrap_or(0);
    let tx_channels = dev.num_channels(Direction::Tx).unwrap_or(0);
    let full_duplex = rx_channels > 0 && dev.full_duplex(Direction::Rx, 0).unwrap_or(false);

    let ranges = |dir: Direction, has: bool| -> Vec<(f64, f64)> {
        if !has {
            return Vec::new();
        }
        dev.frequency_range(dir, 0)
            .map(|v| v.into_iter().map(|r| (r.minimum, r.maximum)).collect())
            .unwrap_or_default()
    };
    let freq_ranges_rx = ranges(Direction::Rx, rx_channels > 0);
    let freq_ranges_tx = ranges(Direction::Tx, tx_channels > 0);

    let mut sample_rates = Vec::new();
    let mut rate_ranges = Vec::new();
    if rx_channels > 0 {
        for r in dev.get_sample_rate_range(Direction::Rx, 0).unwrap_or_default() {
            if (r.maximum - r.minimum).abs() < 1.0 {
                sample_rates.push(r.minimum);
            } else {
                rate_ranges.push((r.minimum, r.maximum));
            }
        }
    }

    let mut gains = Vec::new();
    for (dir, typed, n) in [
        (Direction::Rx, sdroxide_types::Direction::Rx, rx_channels),
        (Direction::Tx, sdroxide_types::Direction::Tx, tx_channels),
    ] {
        if n == 0 {
            continue;
        }
        for name in dev.list_gains(dir, 0).unwrap_or_default() {
            if let Ok(r) = dev.gain_element_range(dir, 0, name.as_str()) {
                gains.push(GainElement {
                    name,
                    direction: typed,
                    min_db: r.minimum,
                    max_db: r.maximum,
                    step_db: r.step,
                });
            }
        }
    }

    let antennas_rx = if rx_channels > 0 {
        dev.antennas(Direction::Rx, 0).unwrap_or_default()
    } else {
        Vec::new()
    };
    let antennas_tx = if tx_channels > 0 {
        dev.antennas(Direction::Tx, 0).unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut sensors: Vec<String> = dev.list_sensors().unwrap_or_default();
    for (dir, n) in [(Direction::Rx, rx_channels), (Direction::Tx, tx_channels)] {
        if n > 0 {
            sensors.extend(dev.list_channel_sensors(dir, 0).unwrap_or_default());
        }
    }
    let lower: Vec<String> = sensors.iter().map(|s| s.to_lowercase()).collect();
    let has_swr_sensor = lower.iter().any(|s| s.contains("swr") || s.contains("vswr"));
    let has_fwd_power_sensor = lower
        .iter()
        .any(|s| s.contains("forward") || s.contains("fwd") || s.contains("tx_power"));

    Ok(DeviceCaps {
        driver,
        label,
        rx_channels,
        tx_channels,
        full_duplex,
        audio_mode: false,
        freq_ranges_rx,
        freq_ranges_tx,
        sample_rates,
        rate_ranges,
        gains,
        antennas_rx,
        antennas_tx,
        sensors,
        has_swr_sensor,
        has_fwd_power_sensor,
    })
}
