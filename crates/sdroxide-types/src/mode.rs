use serde::{Deserialize, Serialize};

/// Demodulation / modulation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Lsb,
    Usb,
    Cw,
    Am,
    Sam,
    Nfm,
    Wfm,
    Digu,
    Digl,
    Dsb,
    Spec,
    /// FT8 digital mode — USB underneath, decoded/encoded by the digi engine.
    Ft8,
    /// FT4 digital mode — USB underneath, decoded/encoded by the digi engine.
    Ft4,
    /// PSK31 keyboard mode — USB underneath, streaming BPSK31 decode/encode.
    Psk,
    /// RTTY keyboard mode — USB underneath, streaming FSK/Baudot decode/encode.
    Rtty,
    /// SSTV image mode — USB underneath, image decode/encode by the digi engine.
    Sstv,
    /// Olivia MFSK keyboard mode — USB underneath, tones/bandwidth chosen in setup.
    Olivia,
    /// THOR (DominoEX-family MFSK+FEC) keyboard mode — submode chosen in setup.
    Thor,
    /// FSQ (Fast Simple QSO) IFK keyboard mode — undirected/directed/image.
    Fsq,
}

impl Mode {
    pub const ALL: [Mode; 19] = [
        Mode::Lsb,
        Mode::Usb,
        Mode::Cw,
        Mode::Am,
        Mode::Sam,
        Mode::Nfm,
        Mode::Wfm,
        Mode::Digu,
        Mode::Digl,
        Mode::Dsb,
        Mode::Spec,
        Mode::Ft8,
        Mode::Ft4,
        Mode::Psk,
        Mode::Rtty,
        Mode::Sstv,
        Mode::Olivia,
        Mode::Thor,
        Mode::Fsq,
    ];

    /// The digital modes handled by a dedicated decode engine over USB
    /// (slotted FT8/FT4, the continuous keyboard modes, and SSTV).
    pub const DIGITAL: [Mode; 8] = [
        Mode::Ft8,
        Mode::Ft4,
        Mode::Psk,
        Mode::Rtty,
        Mode::Olivia,
        Mode::Thor,
        Mode::Fsq,
        Mode::Sstv,
    ];

    /// True for modes that use a dedicated decode/QSO layer over USB.
    pub fn is_digital(self) -> bool {
        matches!(
            self,
            Mode::Ft8
                | Mode::Ft4
                | Mode::Psk
                | Mode::Rtty
                | Mode::Sstv
                | Mode::Olivia
                | Mode::Thor
                | Mode::Fsq
        )
    }

    /// True for the continuous keyboard text modes (PSK31 / RTTY / Olivia / Thor
    /// / FSQ), as opposed to the slotted FT8/FT4 modes. Drives which decode
    /// engine + panel is used.
    pub fn is_text_modem(self) -> bool {
        matches!(
            self,
            Mode::Psk | Mode::Rtty | Mode::Olivia | Mode::Thor | Mode::Fsq
        )
    }

    /// True for the FSQ mode (adds a directed-message / contacts / image layer
    /// on top of the plain keyboard-modem panel).
    pub fn is_fsq(self) -> bool {
        matches!(self, Mode::Fsq)
    }

    /// True for the SSTV image mode. Forks the digi panel to the image UI and
    /// skips the FT8/text-modem overlays.
    pub fn is_sstv(self) -> bool {
        matches!(self, Mode::Sstv)
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Lsb => "LSB",
            Mode::Usb => "USB",
            Mode::Cw => "CW",
            Mode::Am => "AM",
            Mode::Sam => "SAM",
            Mode::Nfm => "NFM",
            Mode::Wfm => "WFM",
            Mode::Digu => "DIGU",
            Mode::Digl => "DIGL",
            Mode::Dsb => "DSB",
            Mode::Spec => "SPEC",
            Mode::Ft8 => "FT8",
            Mode::Ft4 => "FT4",
            Mode::Psk => "PSK",
            Mode::Rtty => "RTTY",
            Mode::Sstv => "SSTV",
            Mode::Olivia => "OLIVIA",
            Mode::Thor => "THOR",
            Mode::Fsq => "FSQ",
        }
    }

    /// Default audio passband edges in Hz relative to the carrier/VFO.
    /// Negative frequencies are below the carrier (LSB side).
    pub fn default_filter(self) -> (f32, f32) {
        match self {
            Mode::Lsb => (-2850.0, -150.0),
            Mode::Usb => (150.0, 2850.0),
            // CW passband is centered on the sidetone pitch (default 700 Hz).
            Mode::Cw => (450.0, 950.0),
            Mode::Am | Mode::Sam => (-5000.0, 5000.0),
            Mode::Nfm => (-8000.0, 8000.0),
            Mode::Wfm => (-96_000.0, 96_000.0),
            Mode::Digu => (200.0, 3200.0),
            Mode::Digl => (-3200.0, -200.0),
            Mode::Dsb => (-2850.0, 2850.0),
            Mode::Spec => (-5000.0, 5000.0),
            // FT8/FT4 occupy the whole USB audio passband (tones 0..~3500 Hz).
            // PSK/RTTY/Olivia/Thor/FSQ do the same (the modem filters narrowly
            // around audio_hz). SSTV occupies the full USB audio passband.
            Mode::Ft8
            | Mode::Ft4
            | Mode::Psk
            | Mode::Rtty
            | Mode::Sstv
            | Mode::Olivia
            | Mode::Thor
            | Mode::Fsq => (100.0, 3300.0),
        }
    }

    /// True for modes that place the displayed carrier below the passband.
    pub fn is_lower_sideband(self) -> bool {
        matches!(self, Mode::Lsb | Mode::Digl)
    }

    /// Furthest a filter edge may be dragged from the carrier — bounded by
    /// the mode's DSP channel bandwidth.
    pub fn max_filter_hz(self) -> f32 {
        match self {
            Mode::Wfm => 120_000.0,
            _ => 24_000.0,
        }
    }

    /// Filter width presets: (label, lo, hi) relative to the carrier.
    pub fn filter_presets(self) -> &'static [(&'static str, f32, f32)] {
        match self {
            Mode::Usb | Mode::Digu => &[
                ("1.8k", 200.0, 2000.0),
                ("2.4k", 200.0, 2600.0),
                ("2.7k", 150.0, 2850.0),
                ("3.3k", 100.0, 3400.0),
            ],
            Mode::Lsb | Mode::Digl => &[
                ("1.8k", -2000.0, -200.0),
                ("2.4k", -2600.0, -200.0),
                ("2.7k", -2850.0, -150.0),
                ("3.3k", -3400.0, -100.0),
            ],
            Mode::Cw => &[
                ("100", 650.0, 750.0),
                ("250", 575.0, 825.0),
                ("500", 450.0, 950.0),
                ("1k", 200.0, 1200.0),
            ],
            Mode::Am | Mode::Sam => &[
                ("6k", -3000.0, 3000.0),
                ("10k", -5000.0, 5000.0),
                ("16k", -8000.0, 8000.0),
            ],
            Mode::Nfm => &[("8k", -4000.0, 4000.0), ("16k", -8000.0, 8000.0)],
            Mode::Dsb => &[("5k", -2500.0, 2500.0), ("6k", -3000.0, 3000.0)],
            // Digital modes have a fixed wide passband; no presets.
            Mode::Wfm
            | Mode::Spec
            | Mode::Ft8
            | Mode::Ft4
            | Mode::Psk
            | Mode::Rtty
            | Mode::Sstv
            | Mode::Olivia
            | Mode::Thor
            | Mode::Fsq => &[],
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Mode::ALL
            .into_iter()
            .find(|m| m.label().eq_ignore_ascii_case(s))
            .ok_or_else(|| format!("unknown mode {s:?} (try USB, LSB, CW, AM, SAM, NFM, WFM…)"))
    }
}

/// Audio noise-reduction intensity — spectral NR applied to the demodulated
/// audio to pull voice out of static/white noise. Cycled Off → Low → Med → High.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum NrLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl NrLevel {
    pub const ALL: [NrLevel; 4] = [NrLevel::Off, NrLevel::Low, NrLevel::Medium, NrLevel::High];

    pub fn label(self) -> &'static str {
        match self {
            NrLevel::Off => "Off",
            NrLevel::Low => "Low",
            NrLevel::Medium => "Med",
            NrLevel::High => "High",
        }
    }

    pub fn is_on(self) -> bool {
        !matches!(self, NrLevel::Off)
    }

    /// Cycle to the next intensity (High wraps back to Off).
    pub fn next(self) -> NrLevel {
        match self {
            NrLevel::Off => NrLevel::Low,
            NrLevel::Low => NrLevel::Medium,
            NrLevel::Medium => NrLevel::High,
            NrLevel::High => NrLevel::Off,
        }
    }

    /// Spectral-NR tuning: `(noise over-estimation factor, minimum gain floor)`.
    /// A larger over-estimate removes more of the noise; a lower floor lets weak
    /// bins be attenuated further — more aggressive, at more risk of artefacts.
    /// The over-factors are modest because the MCRA estimator is unbiased (it
    /// tracks the noise mean, not an under-estimated minimum), so ~1.0 already
    /// removes stationary noise; higher values are pure over-subtraction.
    pub fn params(self) -> (f32, f32) {
        match self {
            NrLevel::Off => (1.0, 1.0),
            NrLevel::Low => (1.0, 0.30),
            NrLevel::Medium => (1.4, 0.14),
            NrLevel::High => (2.0, 0.07),
        }
    }

    /// Make-up gain applied to the listener audio after noise reduction:
    /// suppression lowers the overall level (more so at higher settings), so a
    /// progressively larger boost keeps the perceived loudness roughly constant.
    pub fn makeup_gain(self) -> f32 {
        match self {
            NrLevel::Off => 1.0,
            NrLevel::Low => 1.3,
            NrLevel::Medium => 1.7,
            NrLevel::High => 2.1,
        }
    }
}

/// AGC behavior for a receiver channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgcMode {
    Off,
    Slow,
    Med,
    Fast,
}

impl AgcMode {
    pub const ALL: [AgcMode; 4] = [AgcMode::Off, AgcMode::Slow, AgcMode::Med, AgcMode::Fast];

    pub fn label(self) -> &'static str {
        match self {
            AgcMode::Off => "Off",
            AgcMode::Slow => "Slow",
            AgcMode::Med => "Med",
            AgcMode::Fast => "Fast",
        }
    }

    /// Hang time in milliseconds; `None` means AGC disabled.
    pub fn hang_ms(self) -> Option<f32> {
        match self {
            AgcMode::Off => None,
            AgcMode::Slow => Some(1000.0),
            AgcMode::Med => Some(500.0),
            AgcMode::Fast => Some(100.0),
        }
    }
}
