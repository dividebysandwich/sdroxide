use serde::{Deserialize, Serialize};

/// Amateur bands plus general coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Band {
    M160,
    M80,
    M60,
    M40,
    M30,
    M20,
    M17,
    M15,
    M12,
    M10,
    M6,
    M2,
    Gen,
}

impl Band {
    pub const ALL: [Band; 13] = [
        Band::M160,
        Band::M80,
        Band::M60,
        Band::M40,
        Band::M30,
        Band::M20,
        Band::M17,
        Band::M15,
        Band::M12,
        Band::M10,
        Band::M6,
        Band::M2,
        Band::Gen,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Band::M160 => "160M",
            Band::M80 => "80M",
            Band::M60 => "60M",
            Band::M40 => "40M",
            Band::M30 => "30M",
            Band::M20 => "20M",
            Band::M17 => "17M",
            Band::M15 => "15M",
            Band::M12 => "12M",
            Band::M10 => "10M",
            Band::M6 => "6M",
            Band::M2 => "2M",
            Band::Gen => "GEN",
        }
    }

    /// Band edges in Hz (IARU Region 1 defaults; region-specific edges come from config).
    /// `None` for general coverage.
    pub fn edges(self) -> Option<(f64, f64)> {
        match self {
            Band::M160 => Some((1_810_000.0, 2_000_000.0)),
            Band::M80 => Some((3_500_000.0, 3_800_000.0)),
            Band::M60 => Some((5_351_500.0, 5_366_500.0)),
            Band::M40 => Some((7_000_000.0, 7_200_000.0)),
            Band::M30 => Some((10_100_000.0, 10_150_000.0)),
            Band::M20 => Some((14_000_000.0, 14_350_000.0)),
            Band::M17 => Some((18_068_000.0, 18_168_000.0)),
            Band::M15 => Some((21_000_000.0, 21_450_000.0)),
            Band::M12 => Some((24_890_000.0, 24_990_000.0)),
            Band::M10 => Some((28_000_000.0, 29_700_000.0)),
            Band::M6 => Some((50_000_000.0, 52_000_000.0)),
            Band::M2 => Some((144_000_000.0, 146_000_000.0)),
            Band::Gen => None,
        }
    }

    /// The band containing `hz`, or `Gen` if none does.
    pub fn containing(hz: f64) -> Band {
        Band::ALL
            .into_iter()
            .find(|b| b.edges().is_some_and(|(lo, hi)| hz >= lo && hz <= hi))
            .unwrap_or(Band::Gen)
    }

    /// A reasonable default frequency/mode when jumping to a band with no stack history.
    pub fn default_entry(self) -> (f64, crate::Mode) {
        use crate::Mode;
        match self {
            Band::M160 => (1_840_000.0, Mode::Lsb),
            Band::M80 => (3_700_000.0, Mode::Lsb),
            Band::M60 => (5_357_000.0, Mode::Usb),
            Band::M40 => (7_100_000.0, Mode::Lsb),
            Band::M30 => (10_120_000.0, Mode::Cw),
            Band::M20 => (14_200_000.0, Mode::Usb),
            Band::M17 => (18_120_000.0, Mode::Usb),
            Band::M15 => (21_250_000.0, Mode::Usb),
            Band::M12 => (24_940_000.0, Mode::Usb),
            Band::M10 => (28_400_000.0, Mode::Usb),
            Band::M6 => (50_150_000.0, Mode::Usb),
            Band::M2 => (145_500_000.0, Mode::Nfm),
            Band::Gen => (7_200_000.0, Mode::Am),
        }
    }
}
