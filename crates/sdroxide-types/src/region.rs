//! Which ITU / IARU region the station operates in, and the process-wide
//! setting every band-plan lookup reads.
//!
//! The amateur allocations are not the same the world over. 70 cm stops at
//! 440 MHz across most of Europe and runs to 450 MHz in the Americas; 40 m ends
//! at 7.200 in Regions 1 and 3 and at 7.300 in Region 2; the digital and phone
//! sub-bands sit in different places again. A band plan is therefore only
//! meaningful once you know where the operator is.
//!
//! ## Why a global
//!
//! The region is read from [`Band::edges`](crate::Band::edges),
//! [`Band::containing`](crate::Band::containing) and the segment tables — which
//! are called from roughly thirty places across the engine, the UI, the
//! skimmers and the speech announcer, most of them deep inside code that has no
//! business carrying a region parameter. Threading one through all of them
//! would be a large change that buys nothing: a station is in exactly one
//! region, and it does not move.
//!
//! So the region lives in one atomic, set once at startup from the station's
//! config and again whenever the operator changes it (or, on a remote client,
//! whenever the station announces its own). Every entry point that reads it
//! also has an explicit `…_in(region)` twin, which is what the tests use — they
//! run in one process, in parallel, and must never depend on a global one of
//! them might be changing.

use core::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

/// An ITU radio region, which is also how the IARU divides its band plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Region {
    /// Europe, Africa, the Middle East and northern Asia.
    ///
    /// The default because it is what every band table in sdroxide held before
    /// this setting existed: an installation that never touches it keeps
    /// exactly the band plan it had.
    #[default]
    R1,
    /// The Americas, including Greenland and the eastern Pacific.
    R2,
    /// Southern and eastern Asia, Australasia and the western Pacific.
    R3,
}

impl Region {
    pub const ALL: [Region; 3] = [Region::R1, Region::R2, Region::R3];

    /// 1, 2 or 3.
    pub fn number(self) -> u8 {
        match self {
            Region::R1 => 1,
            Region::R2 => 2,
            Region::R3 => 3,
        }
    }

    /// The part of the world this region covers, for the settings dropdown.
    pub fn area(self) -> &'static str {
        match self {
            Region::R1 => "Europe, Africa, Middle East, Northern Asia",
            Region::R2 => "The Americas",
            Region::R3 => "Southern & Eastern Asia, Australasia, Pacific",
        }
    }

    /// Number and area together — "Region 1 — Europe, Africa, …".
    pub fn label(self) -> &'static str {
        match self {
            Region::R1 => "Region 1 — Europe, Africa, Middle East, Northern Asia",
            Region::R2 => "Region 2 — The Americas",
            Region::R3 => "Region 3 — Southern & Eastern Asia, Australasia, Pacific",
        }
    }

    /// The short form for a status line or a tooltip: "R1".
    pub fn short(self) -> &'static str {
        match self {
            Region::R1 => "R1",
            Region::R2 => "R2",
            Region::R3 => "R3",
        }
    }

    /// This region as a one-bit mask, for the tables that tag a frequency with
    /// the regions it is conventional in.
    pub(crate) const fn bit(self) -> u8 {
        match self {
            Region::R1 => 1,
            Region::R2 => 2,
            Region::R3 => 4,
        }
    }

    /// True when `mask` includes this region. A mask of 0 means "everywhere",
    /// which is how most conventional frequencies are tagged.
    pub(crate) fn in_mask(self, mask: u8) -> bool {
        mask == 0 || mask & self.bit() != 0
    }
}

/// Bit masks for the region-tagged frequency tables. `ALL` (zero) is the
/// common case: a convention the whole world shares.
pub(crate) mod mask {
    pub(crate) const ALL: u8 = 0;
    pub(crate) const R1: u8 = 1;
    pub(crate) const R2: u8 = 2;
    /// Region 3 alone. The repeater plan is the first table to need it: the
    /// Region 3 societies' 70 cm shift is neither of the other two regions'.
    pub(crate) const R3: u8 = 4;
    /// Regions 2 and 3 together — the split most 40 m and 80 m conventions
    /// fall along.
    pub(crate) const R23: u8 = 2 | 4;
    /// Regions 1 and 3 together. Only the 13 cm FT8 split falls this way: the
    /// Americas work the 2304 segment and both of the others the 2320 one.
    pub(crate) const R13: u8 = 1 | 4;
    /// Regions 1 and 2 together — where the Australian and New Zealand APRS
    /// channel is worth naming but is nobody's default.
    pub(crate) const R12: u8 = 1 | 2;
}

/// The station's region, as a discriminant index into [`Region::ALL`].
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The region every band-plan lookup uses when it is not given one.
pub fn region() -> Region {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Region::R2,
        2 => Region::R3,
        _ => Region::R1,
    }
}

/// Adopt `r` as the station's region.
///
/// Called from the binary at startup, from the engine when the operator
/// changes the setting, and on a remote client when the station announces its
/// own. Everything downstream reads it on the next lookup, so nothing has to be
/// rebuilt — the band buttons, the waterfall's band strip and the transmit
/// lockout all follow within a frame.
pub fn set_region(r: Region) {
    let idx = match r {
        Region::R1 => 0,
        Region::R2 => 1,
        Region::R3 => 2,
    };
    CURRENT.store(idx, Ordering::Relaxed);
}

/// Which country's licence-free channel plans (CB, Freenet, PMR446) the
/// waterfall marks. National law, which the IARU region does not reach, so a
/// setting of its own beside it — a screen preference in `[ui]`, following
/// the computer's own country unless the operator picks one.
// Why: HDZ 2026-11-09: "beim Land solltest du default das Land des Rechners
//      nehmen, alles andere ist Overengineering".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Country {
    /// No channel plans.
    Off,
    /// CB 1–80 (BNetzA Vfg. 21/2021), Freenet (Vfg. 45/2025), PMR446
    /// (ECC/DEC/(15)05).
    Germany,
    /// Whatever country this computer (or browser) is set to: the region
    /// part of its preferred locale. The default, and declared last because
    /// serde's catch-all must be the final variant — a country this build
    /// does not know degrades to following the computer.
    #[default]
    #[serde(other)]
    System,
}

impl Country {
    /// In the order the settings list them.
    pub const ALL: [Country; 3] = [Country::System, Country::Off, Country::Germany];

    /// The country an ISO 3166 region code stands for, as far as channel
    /// plans go: one this build carries plans for, or [`Country::Off`].
    pub fn from_region_code(code: &str) -> Country {
        match code.to_ascii_uppercase().as_str() {
            "DE" => Country::Germany,
            _ => Country::Off,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_region_round_trips_through_the_global() {
        // Deliberately the only test that touches the global: everything else
        // uses the explicit `…_in(region)` entry points, so this one cannot
        // race them. It restores the default before it returns.
        for r in Region::ALL {
            set_region(r);
            assert_eq!(region(), r);
        }
        set_region(Region::default());
        assert_eq!(region(), Region::R1, "the default must stay Region 1");
    }

    #[test]
    fn masks_select_the_right_regions() {
        assert!(Region::R1.in_mask(mask::ALL) && Region::R3.in_mask(mask::ALL));
        assert!(Region::R1.in_mask(mask::R1));
        assert!(!Region::R2.in_mask(mask::R1) && !Region::R3.in_mask(mask::R1));
        assert!(Region::R2.in_mask(mask::R23) && Region::R3.in_mask(mask::R23));
        assert!(!Region::R1.in_mask(mask::R23));
    }

    #[test]
    fn labels_name_the_region_and_the_place() {
        for r in Region::ALL {
            assert!(r.label().contains(&r.number().to_string()));
            assert!(r.label().ends_with(r.area()));
        }
    }
}
