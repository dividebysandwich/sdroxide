//! Citizens' band channel plans, per country.
//!
//! 11 m is not one band everywhere. The channel *set* is country law: the
//! CEPT/FCC 40 channels that most of the world shares, Germany's 80-channel
//! plan that adds a high band, and the UK's 27/81 channels, which sit at
//! different frequencies from anyone else's. An operator on 11 m thinks in
//! channel numbers, so the fork carries the plans and shows the channel.
//!
//! ## What a plan does here
//!
//! Only the channels: which frequencies are CB channels, what the band opens
//! on, and (for the UI) the channel number the dial is on. The band's edges are
//! left wide — 26.965–27.860, covering every plan and the freeband between
//! them — so switching plans never moves the operator off the air or changes
//! what transmits. That is deliberate: a narrower plan is about labelling and
//! the default channel, not about locking the receiver.
//!
//! ## The 40-channel table is not a grid
//!
//! Channels 23–25 are the historical oddity (27.255, 27.235, 27.245), and the
//! spacing elsewhere is three 10 kHz steps then a 20 kHz skip. The table is
//! therefore written out rather than generated from a spacing.

use core::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

use crate::Mode;

/// The shared 40-channel 27 MHz table, channel 1 first, in hertz.
pub const CH_40: [u32; 40] = [
    26_965_000, 26_975_000, 26_985_000, 27_005_000, 27_015_000, 27_025_000, 27_035_000,
    27_055_000, 27_065_000, 27_075_000, 27_085_000, 27_105_000, 27_115_000, 27_125_000,
    27_135_000, 27_155_000, 27_165_000, 27_175_000, 27_185_000, 27_205_000, 27_215_000,
    27_225_000, 27_255_000, 27_235_000, 27_245_000, 27_265_000, 27_275_000, 27_285_000,
    27_295_000, 27_305_000, 27_315_000, 27_325_000, 27_335_000, 27_345_000, 27_355_000,
    27_365_000, 27_375_000, 27_385_000, 27_395_000, 27_405_000,
];

/// Germany's high band is the same table shifted up by this much, channels
/// 41–80 at 27.415–27.855.
const HIGH_OFFSET_HZ: u32 = 450_000;

/// The UK 27/81 channels are a plain 10 kHz grid, 27.60125 upward.
const UK_FIRST_HZ: u32 = 27_601_250;
const CHANNEL_SPACING_HZ: u32 = 10_000;

/// How near a channel the dial must be to be *on* it, for the channel readout.
/// Half a channel, so the number flips at the midpoint between two.
const ON_CHANNEL_HZ: f64 = 5_000.0;

/// Which country's CB channels the station works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CbPlan {
    /// The 40 shared channels plus the high band (41–80): the freeband span,
    /// permissive about mode. The default, and what the 11 m band has always
    /// covered.
    #[default]
    World,
    /// CEPT / EU: the 40 shared channels, FM.
    Cept,
    /// Germany: 80 channels, FM.
    Germany80,
    /// The United Kingdom: 27/81, 40 channels at their own frequencies, FM.
    Uk27_81,
    /// The United States: the 40 shared channels, AM and SSB.
    Us,
    /// Australia: the 40 shared channels, AM and SSB.
    Australia,
}

impl CbPlan {
    pub const ALL: [CbPlan; 6] = [
        CbPlan::World,
        CbPlan::Cept,
        CbPlan::Germany80,
        CbPlan::Uk27_81,
        CbPlan::Us,
        CbPlan::Australia,
    ];

    fn index(self) -> u8 {
        match self {
            CbPlan::World => 0,
            CbPlan::Cept => 1,
            CbPlan::Germany80 => 2,
            CbPlan::Uk27_81 => 3,
            CbPlan::Us => 4,
            CbPlan::Australia => 5,
        }
    }

    /// The chip label.
    pub fn short(self) -> &'static str {
        match self {
            CbPlan::World => "WORLD",
            CbPlan::Cept => "EU",
            CbPlan::Germany80 => "DE",
            CbPlan::Uk27_81 => "UK",
            CbPlan::Us => "US",
            CbPlan::Australia => "AU",
        }
    }

    /// The full name, for the settings dropdown and tooltips.
    pub fn label(self) -> &'static str {
        match self {
            CbPlan::World => "World / freeband — 80 channels",
            CbPlan::Cept => "CEPT / EU — 40 channels",
            CbPlan::Germany80 => "Germany — 80 channels",
            CbPlan::Uk27_81 => "United Kingdom — 27/81, 40 channels",
            CbPlan::Us => "United States — 40 channels",
            CbPlan::Australia => "Australia — 40 channels",
        }
    }

    /// The modes the plan is worked in, for a tooltip.
    pub fn modes(self) -> &'static str {
        match self {
            CbPlan::World => "AM · FM · SSB",
            CbPlan::Cept | CbPlan::Germany80 | CbPlan::Uk27_81 => "FM",
            CbPlan::Us | CbPlan::Australia => "AM · SSB",
        }
    }

    /// Every channel of the plan, in order, in hertz.
    pub fn channels(self) -> Vec<u32> {
        match self {
            CbPlan::Uk27_81 => {
                (0..40).map(|i| UK_FIRST_HZ + i * CHANNEL_SPACING_HZ).collect()
            }
            CbPlan::Cept | CbPlan::Us | CbPlan::Australia => CH_40.to_vec(),
            CbPlan::World | CbPlan::Germany80 => {
                let mut out = CH_40.to_vec();
                out.extend(CH_40.iter().map(|hz| hz + HIGH_OFFSET_HZ));
                out
            }
        }
    }

    /// The channel the band opens on: channel 25 (27.245), the agreed 11 m
    /// digital calling channel, where the plan has it — the UK's own channels do
    /// not include 27.245, so UK opens on channel 19 (27.78125) instead.
    pub fn default_entry(self) -> (f64, Mode) {
        match self {
            CbPlan::Uk27_81 => (f64::from(UK_FIRST_HZ + 18 * CHANNEL_SPACING_HZ), Mode::Usb),
            _ => (27_245_000.0, Mode::Usb),
        }
    }

    /// The channel the dial is on, as `(number, hz)`, when it is within half a
    /// channel of one.
    pub fn on_channel(self, hz: f64) -> Option<(usize, u32)> {
        self.channels()
            .into_iter()
            .enumerate()
            .map(|(i, c)| (i + 1, c))
            .find(|(_, c)| (f64::from(*c) - hz).abs() < ON_CHANNEL_HZ)
    }
}

/// The station's CB plan, as a discriminant index into [`CbPlan::ALL`].
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The CB channel plan every channel lookup uses.
pub fn cb_plan() -> CbPlan {
    match CURRENT.load(Ordering::Relaxed) {
        1 => CbPlan::Cept,
        2 => CbPlan::Germany80,
        3 => CbPlan::Uk27_81,
        4 => CbPlan::Us,
        5 => CbPlan::Australia,
        _ => CbPlan::World,
    }
}

/// Adopt `p` as the station's CB plan. Called at startup from the config, by the
/// engine when the operator changes it, and on a remote client when the station
/// announces its own.
pub fn set_cb_plan(p: CbPlan) {
    CURRENT.store(p.index(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_table_is_the_real_cb_channel_list() {
        assert_eq!(CH_40.len(), 40);
        assert_eq!(CH_40[0], 26_965_000, "channel 1");
        assert_eq!(CH_40[24], 27_245_000, "channel 25, the digital calling channel");
        assert_eq!(CH_40[39], 27_405_000, "channel 40");
        // The 23/24/25 quirk: 27.255, 27.235, 27.245 — not a rising grid.
        assert_eq!((CH_40[22], CH_40[23], CH_40[24]), (27_255_000, 27_235_000, 27_245_000));
        // And the 20 kHz skip between 27.035 and 27.055.
        assert_eq!((CH_40[6], CH_40[7]), (27_035_000, 27_055_000));
    }

    #[test]
    fn world_and_germany_are_the_eighty_frequency_plan() {
        let chans = CbPlan::World.channels();
        assert_eq!(chans.len(), 80);
        assert_eq!(chans[40], 27_415_000, "channel 41 is channel 1 plus 450 kHz");
        assert_eq!(chans[79], 27_855_000, "channel 80");
        assert_eq!(CbPlan::Germany80.channels(), chans);
    }

    #[test]
    fn the_uk_plan_is_its_own_grid() {
        let chans = CbPlan::Uk27_81.channels();
        assert_eq!(chans.len(), 40);
        assert_eq!(chans[0], 27_601_250);
        assert_eq!(chans[39], 27_991_250);
        assert_eq!(CbPlan::Uk27_81.default_entry(), (27_781_250.0, Mode::Usb));
    }

    #[test]
    fn a_dial_on_a_channel_reads_back() {
        assert_eq!(CbPlan::Cept.on_channel(27_245_000.0), Some((25, 27_245_000)));
        // Just off it still reads; past the midpoint it does not.
        assert_eq!(CbPlan::Cept.on_channel(27_249_000.0), Some((25, 27_245_000)));
        assert!(CbPlan::Cept.on_channel(27_250_000.0).is_none());
        // 27.245 is not a UK channel, so the UK plan has nothing there.
        assert!(CbPlan::Uk27_81.on_channel(27_245_000.0).is_none());
    }

    #[test]
    fn the_default_entries_are_on_channel() {
        for p in CbPlan::ALL {
            let (hz, _) = p.default_entry();
            assert!(
                p.on_channel(hz).is_some(),
                "{} default {hz} Hz is not on one of its channels",
                p.label()
            );
        }
    }

    #[test]
    fn the_global_round_trips_and_restores() {
        // The only test that touches the global; it restores the default.
        for p in CbPlan::ALL {
            set_cb_plan(p);
            assert_eq!(cb_plan(), p);
        }
        set_cb_plan(CbPlan::default());
        assert_eq!(cb_plan(), CbPlan::World);
    }
}
