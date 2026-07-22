//! Settings and radio-data persistence under the user config directory
//! (`~/.config/sdroxide/` on Linux).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no home/config directory available")]
    NoConfigDir,
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// User settings (`config.toml`). Everything has a default so a missing or
/// partial file always loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// SoapySDR device args, e.g. "driver=hackrf". Empty = first device found.
    pub device_args: String,
    /// Preferred hardware sample rate in Hz.
    pub sample_rate: f64,
    /// dB offset applied to convert dBFS to dBm for the S-meter.
    pub cal_offset_db: f64,
    pub spectrum_fft: u32,
    pub spectrum_fps: u8,
    /// Server mode bind address.
    pub server_bind: String,
    pub server_port: u16,
    /// Refuse to transmit outside amateur bands.
    pub tx_ham_only: bool,
    /// Preferred audio output device name; `None` = system default.
    pub audio_output: Option<String>,
    /// Preferred audio input (microphone) device name; `None` = system default.
    pub audio_input: Option<String>,
    /// UI / display preferences (frame rate, waterfall + spectrum speed).
    pub ui: sdroxide_types::UiSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            device_args: String::new(),
            sample_rate: 1_536_000.0,
            cal_offset_db: 0.0,
            spectrum_fft: 4096,
            spectrum_fps: 30,
            server_bind: "0.0.0.0".into(),
            server_port: 4950,
            tx_ham_only: true,
            audio_output: None,
            audio_input: None,
            ui: sdroxide_types::UiSettings::default(),
        }
    }
}

/// Load just the UI/display preferences (frame rate, waterfall + spectrum speed).
pub fn load_ui_settings() -> sdroxide_types::UiSettings {
    Settings::load().ui
}

/// Persist the UI/display preferences, preserving every other setting
/// (read-modify-write so a concurrent edit elsewhere isn't clobbered).
pub fn save_ui_settings(ui: &sdroxide_types::UiSettings) -> Result<(), ConfigError> {
    let mut s = Settings::load();
    s.ui = *ui;
    s.save()
}

pub fn config_dir() -> Result<PathBuf, ConfigError> {
    directories::ProjectDirs::from("org", "sdroxide", "sdroxide")
        .map(|d| d.config_dir().to_path_buf())
        .ok_or(ConfigError::NoConfigDir)
}

/// Directory for received SSTV images (`~/.config/sdroxide/sstv_rx`), created
/// on demand.
pub fn sstv_rx_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("sstv_rx");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory for the operator's transmit-image slots
/// (`~/.config/sdroxide/sstv_tx`), created on demand.
pub fn sstv_tx_dir() -> Result<PathBuf, ConfigError> {
    let dir = config_dir()?.join("sstv_tx");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

impl Settings {
    /// Load settings; missing file or unreadable content falls back to
    /// defaults (with a warning), so startup never fails on config.
    pub fn load() -> Settings {
        let path = match config_dir() {
            Ok(d) => d.join("config.toml"),
            Err(e) => {
                warn!("no config dir: {e}; using default settings");
                return Settings::default();
            }
        };
        match fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(s) => s,
                Err(e) => {
                    warn!("failed to parse {}: {e}; using defaults", path.display());
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let dir = config_dir()?;
        fs::create_dir_all(&dir)?;
        let text = toml::to_string_pretty(self)?;
        fs::write(dir.join("config.toml"), text)?;
        Ok(())
    }
}

/// Band-stack registers: up to 3 remembered (freq, mode, filter) per band.
pub type BandStacks = std::collections::HashMap<sdroxide_types::Band, Vec<sdroxide_types::BandStackEntry>>;

fn load_json<T: serde::de::DeserializeOwned + Default>(file: &str) -> T {
    let Ok(dir) = config_dir() else { return T::default() };
    match fs::read_to_string(dir.join(file)) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            warn!("failed to parse {file}: {e}; starting fresh");
            T::default()
        }),
        Err(_) => T::default(),
    }
}

fn save_json<T: serde::Serialize>(file: &str, value: &T) -> Result<(), ConfigError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;
    let text = serde_json::to_string_pretty(value).expect("serialize");
    fs::write(dir.join(file), text)?;
    Ok(())
}

pub fn load_bandstacks() -> BandStacks {
    load_json("bandstacks.json")
}

pub fn save_bandstacks(stacks: &BandStacks) -> Result<(), ConfigError> {
    save_json("bandstacks.json", stacks)
}

pub fn load_memories() -> Vec<sdroxide_types::MemoryChannel> {
    load_json("memories.json")
}

pub fn save_memories(memories: &[sdroxide_types::MemoryChannel]) -> Result<(), ConfigError> {
    save_json("memories.json", &memories)
}

/// Radio backend config (SoapySDR vs CAT rig; serial + sound-card settings).
pub fn load_radio_config() -> sdroxide_types::RadioConfig {
    load_json("radio.json")
}

pub fn save_radio_config(cfg: &sdroxide_types::RadioConfig) -> Result<(), ConfigError> {
    save_json("radio.json", cfg)
}

/// FT8/FT4 operator config (own call, grid, message templates).
pub fn load_digi_config() -> sdroxide_types::DigiConfig {
    load_json("digi.json")
}

pub fn save_digi_config(cfg: &sdroxide_types::DigiConfig) -> Result<(), ConfigError> {
    save_json("digi.json", cfg)
}

/// Persistent logbook (digital + manual QSO entries).
pub fn load_qso_log() -> Vec<sdroxide_types::QsoRecord> {
    load_json("qso_log.json")
}

pub fn save_qso_log(log: &[sdroxide_types::QsoRecord]) -> Result<(), ConfigError> {
    save_json("qso_log.json", &log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digi_config_roundtrip_via_json() {
        let cfg = sdroxide_types::DigiConfig {
            my_call: "AB1CD".into(),
            my_grid: "FN42".into(),
            ..Default::default()
        };
        let text = serde_json::to_string_pretty(&cfg).unwrap();
        let back: sdroxide_types::DigiConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.my_call, "AB1CD");
        assert_eq!(back.my_grid, "FN42");
        assert_eq!(back, cfg);
    }

    #[test]
    fn bandstacks_roundtrip_via_json() {
        use sdroxide_types::{Band, BandStackEntry, Mode};
        let mut stacks = BandStacks::default();
        stacks.insert(
            Band::M40,
            vec![BandStackEntry { freq_hz: 7_100_000.0, mode: Mode::Lsb, filter_lo: -2850.0, filter_hi: -150.0 }],
        );
        let text = serde_json::to_string(&stacks).unwrap();
        let back: BandStacks = serde_json::from_str(&text).unwrap();
        assert_eq!(back, stacks);
    }

    #[test]
    fn default_settings_roundtrip_via_toml() {
        let s = Settings::default();
        let text = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn partial_file_fills_defaults() {
        let s: Settings = toml::from_str("sample_rate = 2400000.0").unwrap();
        assert_eq!(s.sample_rate, 2_400_000.0);
        assert_eq!(s.server_port, Settings::default().server_port);
    }
}
